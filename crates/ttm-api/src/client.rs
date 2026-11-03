//! The HTTP client shared by every endpoint module.

use crate::{ApiError, Result, VERSION};
use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(30);
/// Retries after the first attempt.
pub const MAX_RETRIES: u32 = 3;
/// Upper bound for one `Retry-After` wait.
pub const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// `ttm/0.1.0 (macos; aarch64)`.
pub fn user_agent() -> String {
    format!(
        "ttm/{VERSION} ({}; {})",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// Request body, kept owned so a retry can send it again.
#[derive(Debug, Clone)]
pub enum Body {
    Json(serde_json::Value),
    Form(Vec<(String, String)>),
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    api_base: String,
    token: Option<String>,
    deprecation_file: Option<PathBuf>,
    retry_base: Duration,
}

impl Client {
    /// `api_base` is the discovery `api_base` (`https://toantranmei.com/api/v1`); `token` is sent as Bearer.
    pub fn new(api_base: impl Into<String>, token: Option<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(user_agent())
            .timeout(TIMEOUT)
            .build()?;
        Ok(Self {
            http,
            api_base: api_base.into().trim_end_matches('/').to_string(),
            token,
            deprecation_file: None,
            retry_base: Duration::from_millis(500),
        })
    }

    /// Where to remember which deprecation warnings were shown today (`~/.cache/ttm/deprecations`).
    pub fn with_cache_dir(mut self, cache_dir: impl Into<PathBuf>) -> Self {
        self.deprecation_file = Some(cache_dir.into().join("deprecations"));
        self
    }

    /// Base delay of the exponential backoff when the server gives no `Retry-After` (tests use zero).
    pub fn with_retry_base(mut self, base: Duration) -> Self {
        self.retry_base = base;
        self
    }

    pub fn api_base(&self) -> &str {
        &self.api_base
    }

    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// The underlying reqwest client (same User-Agent and timeout), e.g. for discovery.
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// `path` relative to the API base (`/me`), or an absolute URL.
    pub fn url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}/{}", self.api_base, path.trim_start_matches('/'))
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.json(Method::GET, path, &[], None).await
    }

    pub async fn get_query<T: DeserializeOwned>(&self, path: &str, query: &[(&str, String)]) -> Result<T> {
        self.json(Method::GET, path, query, None).await
    }

    pub async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        self.json(Method::POST, path, &[], Some(Body::Json(to_value(body)?)))
            .await
    }

    pub async fn patch<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T> {
        self.json(Method::PATCH, path, &[], Some(Body::Json(to_value(body)?)))
            .await
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        self.send(Method::DELETE, path, &[], None).await.map(drop)
    }

    /// Form POST without the Bearer token (OAuth endpoints are unauthenticated).
    pub async fn post_form<T: DeserializeOwned>(&self, url: &str, form: Vec<(String, String)>) -> Result<T> {
        let response = self
            .send_inner(Method::POST, url, &[], Some(Body::Form(form)), false)
            .await?;
        decode(response).await
    }

    /// Sends and decodes a JSON response.
    pub async fn json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Body>,
    ) -> Result<T> {
        decode(self.send(method, path, query, body).await?).await
    }

    /// Sends with retries; non-success statuses become [`ApiError`].
    pub async fn send(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Body>,
    ) -> Result<reqwest::Response> {
        self.send_inner(method, path, query, body, true).await
    }

    async fn send_inner(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Body>,
        auth: bool,
    ) -> Result<reqwest::Response> {
        let url = self.url(path);
        let idempotent = matches!(
            method,
            Method::GET | Method::HEAD | Method::PUT | Method::DELETE | Method::OPTIONS
        );
        let mut attempt = 0;
        loop {
            let mut request = self
                .http
                .request(method.clone(), &url)
                .header(header::ACCEPT, "application/json");
            if !query.is_empty() {
                request = request.query(query);
            }
            if auth && let Some(token) = &self.token {
                request = request.bearer_auth(token);
            }
            request = match &body {
                Some(Body::Json(value)) => request.json(value),
                Some(Body::Form(form)) => request.form(form),
                None => request,
            };
            let result = request.send().await;
            let response = match result {
                Ok(response) => response,
                Err(err) if attempt < MAX_RETRIES && idempotent && (err.is_connect() || err.is_timeout()) => {
                    tokio::time::sleep(self.backoff(attempt)).await;
                    attempt += 1;
                    continue;
                }
                Err(err) => return Err(err.into()),
            };
            let status = response.status();
            self.note_deprecation(&method, &url, response.headers());
            if status.is_success() {
                return Ok(response);
            }
            let retryable =
                status == StatusCode::TOO_MANY_REQUESTS || (status.is_server_error() && idempotent);
            if retryable && attempt < MAX_RETRIES {
                let wait = retry_after(response.headers()).unwrap_or_else(|| self.backoff(attempt));
                tracing::debug!("HTTP {status} from {url}; retry {} in {wait:?}", attempt + 1);
                tokio::time::sleep(wait).await;
                attempt += 1;
                continue;
            }
            return Err(error_from(response).await);
        }
    }

    fn backoff(&self, attempt: u32) -> Duration {
        self.retry_base.saturating_mul(2u32.saturating_pow(attempt))
    }

    /// `Deprecation` header → one warning per endpoint per day.
    fn note_deprecation(&self, method: &Method, url: &str, headers: &HeaderMap) {
        let Some(deprecation) = headers.get("deprecation").and_then(|v| v.to_str().ok()) else {
            return;
        };
        let sunset = headers.get("sunset").and_then(|v| v.to_str().ok());
        let path = url::Url::parse(url)
            .map(|u| u.path().to_string())
            .unwrap_or_else(|_| url.to_string());
        let key = format!("{method} {path}");
        if !should_warn_today(self.deprecation_file.as_ref(), &key) {
            return;
        }
        let link = headers
            .get_all(header::LINK)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find(|v| v.contains("deprecation"))
            .map(|v| format!(" See {v}."))
            .unwrap_or_default();
        let sunset = sunset.map(|s| format!(", removed after {s}")).unwrap_or_default();
        tracing::warn!(
            "{key} is deprecated ({deprecation}{sunset}); update ttm with `ttm self-update`.{link}"
        );
    }
}

fn to_value<B: Serialize>(body: &B) -> Result<serde_json::Value> {
    serde_json::to_value(body).map_err(|e| ApiError::Decode {
        status: 0,
        message: format!("cannot encode body: {e}"),
    })
}

/// `Retry-After: <seconds>` (HTTP-date is rare from our server and treated as one second), capped.
fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(header::RETRY_AFTER)?.to_str().ok()?.trim();
    let wait = match value.parse::<u64>() {
        Ok(secs) => Duration::from_secs(secs),
        Err(_) => Duration::from_secs(1),
    };
    Some(wait.min(MAX_RETRY_AFTER))
}

async fn error_from(response: reqwest::Response) -> ApiError {
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v: &HeaderValue| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body = response.text().await.unwrap_or_default();
    ApiError::from_body(status, &content_type, &body)
}

async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T> {
    let status = response.status().as_u16();
    let bytes = response.bytes().await?;
    let bytes: &[u8] = if bytes.is_empty() { b"null" } else { &bytes };
    serde_json::from_slice(bytes).map_err(|e| ApiError::Decode {
        status,
        message: e.to_string(),
    })
}

fn today() -> String {
    time::OffsetDateTime::now_utc().date().to_string()
}

/// Reads and updates the `deprecations` file (`{"GET /api/v1/x": "2026-11-04"}`). Without a file, always warns.
fn should_warn_today(file: Option<&PathBuf>, key: &str) -> bool {
    let Some(file) = file else { return true };
    let mut seen: BTreeMap<String, String> = std::fs::read(file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let today = today();
    if seen.get(key) == Some(&today) {
        return false;
    }
    seen.insert(key.to_string(), today);
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(bytes) = serde_json::to_vec_pretty(&seen) {
        let _ = std::fs::write(file, bytes);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_format() {
        let ua = user_agent();
        assert!(ua.starts_with(&format!("ttm/{VERSION} (")), "{ua}");
        assert!(ua.contains(std::env::consts::OS) && ua.ends_with(&format!("{})", std::env::consts::ARCH)));
    }

    #[test]
    fn url_joins_relative_and_keeps_absolute() {
        let c = Client::new("https://a.com/api/v1/", None).unwrap();
        assert_eq!(c.url("/me"), "https://a.com/api/v1/me");
        assert_eq!(c.url("me"), "https://a.com/api/v1/me");
        assert_eq!(c.url("https://b.com/x"), "https://b.com/x");
    }

    #[test]
    fn retry_after_parsing() {
        let mut h = HeaderMap::new();
        assert_eq!(retry_after(&h), None);
        h.insert(header::RETRY_AFTER, HeaderValue::from_static("2"));
        assert_eq!(retry_after(&h), Some(Duration::from_secs(2)));
        h.insert(header::RETRY_AFTER, HeaderValue::from_static("9999"));
        assert_eq!(retry_after(&h), Some(MAX_RETRY_AFTER));
    }

    #[test]
    fn deprecation_warns_once_a_day() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("deprecations");
        assert!(should_warn_today(Some(&file), "GET /api/v1/old"));
        assert!(!should_warn_today(Some(&file), "GET /api/v1/old"));
        assert!(should_warn_today(Some(&file), "GET /api/v1/other"));
        assert!(should_warn_today(None, "GET /api/v1/old"));
    }

    #[test]
    fn backoff_doubles() {
        let c = Client::new("https://a.com", None)
            .unwrap()
            .with_retry_base(Duration::from_millis(100));
        assert_eq!(c.backoff(0), Duration::from_millis(100));
        assert_eq!(c.backoff(2), Duration::from_millis(400));
    }
}
