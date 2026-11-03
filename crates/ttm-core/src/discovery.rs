//! `GET <host>/.well-known/ttm`: where the API, sync, MCP and OAuth endpoints of a host live. Cached for one hour in
//! `<cache>/discovery-<host>.json`; a stale cache is used when the host cannot be reached.

use crate::paths::write_atomic;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const CACHE_TTL: Duration = Duration::from_secs(3600);
pub const WELL_KNOWN_PATH: &str = "/.well-known/ttm";

/// The discovery document. Fields the server may omit fall back to the documented defaults under the host.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discovery {
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(default)]
    pub sync_url: Option<String>,
    #[serde(default)]
    pub mcp_url: Option<String>,
    #[serde(default)]
    pub issuer: Option<String>,
    #[serde(default)]
    pub authorization_endpoint: Option<String>,
    #[serde(default)]
    pub token_endpoint: Option<String>,
    #[serde(default)]
    pub device_authorization_endpoint: Option<String>,
    #[serde(default)]
    pub verification_uri: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub min_cli: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

impl Discovery {
    /// `<host>/api/v1` unless the document says otherwise.
    pub fn api_base(&self, host: &str) -> String {
        trim(self.api_base.clone()).unwrap_or_else(|| format!("{host}/api/v1"))
    }

    /// Web consent page for the loopback login (`/ket-noi/cli`).
    pub fn authorization_endpoint(&self, host: &str) -> String {
        trim(self.authorization_endpoint.clone()).unwrap_or_else(|| format!("{host}/ket-noi/cli"))
    }

    pub fn token_endpoint(&self, host: &str) -> String {
        trim(self.token_endpoint.clone()).unwrap_or_else(|| format!("{}/oauth/token", self.api_base(host)))
    }

    pub fn device_authorization_endpoint(&self, host: &str) -> String {
        trim(self.device_authorization_endpoint.clone())
            .unwrap_or_else(|| format!("{}/oauth/device_authorization", self.api_base(host)))
    }

    /// RFC 7009 revocation; not in the document, always next to the token endpoint.
    pub fn revocation_endpoint(&self, host: &str) -> String {
        format!("{}/oauth/revoke", self.api_base(host))
    }

    pub fn verification_uri(&self, host: &str) -> String {
        trim(self.verification_uri.clone()).unwrap_or_else(|| format!("{host}/activate"))
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// `Some(min_cli)` when `current` is older than the server's `min_cli`.
    pub fn upgrade_required(&self, current: &str) -> Option<String> {
        let min = self.min_cli.as_deref()?;
        let (Ok(min_v), Ok(cur_v)) = (semver::Version::parse(min), semver::Version::parse(current)) else {
            return None;
        };
        (cur_v < min_v).then(|| min.to_string())
    }
}

fn trim(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    fetched_at: u64,
    discovery: Discovery,
}

/// `https://toantranmei.com:8443` → `discovery-https_toantranmei.com_8443.json`.
pub fn cache_file(cache_dir: &Path, host: &str) -> PathBuf {
    let safe: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.replace("___", "_");
    cache_dir.join(format!("discovery-{safe}.json"))
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_cache(path: &Path) -> Option<CacheEntry> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Returns the discovery document of `host`, from cache when younger than [`CACHE_TTL`] (unless `refresh`).
pub async fn discover(
    http: &reqwest::Client,
    host: &str,
    cache_dir: &Path,
    refresh: bool,
) -> Result<Discovery> {
    let path = cache_file(cache_dir, host);
    let cached = read_cache(&path);
    if !refresh
        && let Some(entry) = &cached
        && now_secs().saturating_sub(entry.fetched_at) < CACHE_TTL.as_secs()
    {
        return Ok(entry.discovery.clone());
    }
    match fetch(http, host).await {
        Ok(discovery) => {
            let entry = CacheEntry {
                fetched_at: now_secs(),
                discovery: discovery.clone(),
            };
            if let Ok(bytes) = serde_json::to_vec_pretty(&entry)
                && let Err(err) = write_atomic(&path, &bytes, None)
            {
                tracing::debug!("could not cache discovery: {err}");
            }
            Ok(discovery)
        }
        Err(err) => match cached {
            Some(entry) => {
                tracing::warn!("{err}; using the cached discovery document");
                Ok(entry.discovery)
            }
            None => Err(err),
        },
    }
}

/// Fetches without the cache.
pub async fn fetch(http: &reqwest::Client, host: &str) -> Result<Discovery> {
    let fail = |message: String| Error::Discovery {
        host: host.to_string(),
        message,
    };
    let response = http
        .get(format!("{host}{WELL_KNOWN_PATH}"))
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| fail(e.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(fail(format!("HTTP {status}")));
    }
    response
        .json::<Discovery>()
        .await
        .map_err(|e| fail(format!("invalid JSON: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample() -> serde_json::Value {
        serde_json::json!({
            "api_base": "https://x.com/api/v1",
            "sync_url": "wss://x.com/sync",
            "mcp_url": "https://x.com/mcp",
            "issuer": "https://x.com",
            "authorization_endpoint": "https://x.com/ket-noi/cli",
            "token_endpoint": "https://x.com/api/v1/oauth/token",
            "device_authorization_endpoint": "https://x.com/api/v1/oauth/device_authorization",
            "verification_uri": "https://x.com/activate",
            "api_version": "v1",
            "min_cli": "0.1.0",
            "features": ["notee", "factory"]
        })
    }

    #[test]
    fn defaults_follow_the_host() {
        let d = Discovery::default();
        assert_eq!(
            d.api_base("http://localhost:3000"),
            "http://localhost:3000/api/v1"
        );
        assert_eq!(
            d.token_endpoint("https://a.com"),
            "https://a.com/api/v1/oauth/token"
        );
        assert_eq!(
            d.revocation_endpoint("https://a.com"),
            "https://a.com/api/v1/oauth/revoke"
        );
        assert_eq!(
            d.authorization_endpoint("https://a.com"),
            "https://a.com/ket-noi/cli"
        );
        assert_eq!(d.verification_uri("https://a.com"), "https://a.com/activate");
    }

    #[test]
    fn parses_server_document() {
        let d: Discovery = serde_json::from_value(sample()).unwrap();
        assert_eq!(d.api_base("ignored"), "https://x.com/api/v1");
        assert!(d.has_feature("factory"));
        assert!(!d.has_feature("languu"));
    }

    #[test]
    fn upgrade_check() {
        let d = Discovery {
            min_cli: Some("0.3.0".into()),
            ..Discovery::default()
        };
        assert_eq!(d.upgrade_required("0.2.9"), Some("0.3.0".into()));
        assert_eq!(d.upgrade_required("0.3.0"), None);
        assert_eq!(Discovery::default().upgrade_required("0.1.0"), None);
    }

    #[test]
    fn cache_file_name_is_safe() {
        let p = cache_file(Path::new("/c"), "http://localhost:3000");
        let name = p.file_name().unwrap().to_str().unwrap();
        assert!(
            name.starts_with("discovery-http_") && name.ends_with(".json"),
            "{name}"
        );
        assert!(!name.contains('/') && !name.contains(':'));
    }

    #[tokio::test]
    async fn caches_for_an_hour_and_falls_back_to_stale() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(WELL_KNOWN_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample()))
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let http = reqwest::Client::new();
        let first = discover(&http, &server.uri(), dir.path(), false).await.unwrap();
        let second = discover(&http, &server.uri(), dir.path(), false).await.unwrap();
        assert_eq!(first, second);
        server.verify().await;

        // Host down: a stale cache still answers.
        let file = cache_file(dir.path(), &server.uri());
        let mut entry: CacheEntry = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        entry.fetched_at = 0;
        std::fs::write(&file, serde_json::to_vec(&entry).unwrap()).unwrap();
        server.reset().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let stale = discover(&http, &server.uri(), dir.path(), false).await.unwrap();
        assert_eq!(stale, first);
    }

    #[tokio::test]
    async fn error_without_cache() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let err = discover(&reqwest::Client::new(), &server.uri(), dir.path(), true)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("404"), "{err}");
    }
}
