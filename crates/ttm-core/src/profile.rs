//! A profile is a named (host, workspace) pair in `config.toml`, e.g. `[profiles.work]`.

use crate::{DEFAULT_HOST, Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// Server base URL, e.g. `https://toantranmei.com` or `http://localhost:3000`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Current workspace id (set by `ttm workspace switch`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

/// Picks the host: `flag` (`--host`) > `env` (`TTM_HOST`) > `profile` > [`DEFAULT_HOST`]. Empty values are skipped.
/// The result is normalised (no trailing slash, scheme checked).
pub fn resolve_host(flag: Option<&str>, env: Option<&str>, profile: Option<&str>) -> Result<String> {
    let chosen = [flag, env, profile]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|v| !v.is_empty())
        .unwrap_or(DEFAULT_HOST);
    normalize_host(chosen)
}

/// `Toantranmei.com/` → `https://toantranmei.com`; `http://localhost:3000/` → `http://localhost:3000`.
/// A bare name without a scheme gets `https://`, except `localhost` / `127.0.0.1`, which get `http://`.
pub fn normalize_host(raw: &str) -> Result<String> {
    let raw = raw.trim();
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else if raw.starts_with("localhost") || raw.starts_with("127.0.0.1") {
        format!("http://{raw}")
    } else {
        format!("https://{raw}")
    };
    let url = url::Url::parse(&with_scheme).map_err(|_| Error::InvalidHost(raw.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(Error::InvalidHost(raw.to_string()));
    }
    let mut out = format!(
        "{}://{}",
        url.scheme(),
        url.host_str().unwrap_or_default().to_ascii_lowercase()
    );
    if let Some(port) = url.port() {
        out.push_str(&format!(":{port}"));
    }
    let path = url.path().trim_end_matches('/');
    out.push_str(path);
    Ok(out)
}

/// Picks the profile name: `flag` (`--profile`) > `env` (`TTM_PROFILE`) > `default_profile` > `default`.
pub fn resolve_profile_name(flag: Option<&str>, env: Option<&str>, configured: Option<&str>) -> String {
    [flag, env, configured]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|v| !v.is_empty())
        .unwrap_or(crate::DEFAULT_PROFILE)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_order_is_flag_env_profile_default() {
        assert_eq!(
            resolve_host(
                Some("https://a.com"),
                Some("https://b.com"),
                Some("https://c.com")
            )
            .unwrap(),
            "https://a.com"
        );
        assert_eq!(
            resolve_host(None, Some("https://b.com"), Some("https://c.com")).unwrap(),
            "https://b.com"
        );
        assert_eq!(
            resolve_host(None, Some(""), Some("https://c.com")).unwrap(),
            "https://c.com"
        );
        assert_eq!(
            resolve_host(None, None, None).unwrap(),
            normalize_host(DEFAULT_HOST).unwrap()
        );
    }

    #[test]
    fn normalize_trims_and_adds_scheme() {
        assert_eq!(
            normalize_host("https://Toantranmei.com/").unwrap(),
            "https://toantranmei.com"
        );
        assert_eq!(
            normalize_host("toantranmei.com").unwrap(),
            "https://toantranmei.com"
        );
        assert_eq!(normalize_host("localhost:3000").unwrap(), "http://localhost:3000");
        assert_eq!(
            normalize_host("http://127.0.0.1:8080/base/").unwrap(),
            "http://127.0.0.1:8080/base"
        );
    }

    #[test]
    fn normalize_rejects_other_schemes() {
        assert!(normalize_host("ftp://example.com").is_err());
        assert!(normalize_host("https://").is_err());
    }

    #[test]
    fn profile_name_order() {
        assert_eq!(
            resolve_profile_name(Some("work"), Some("ci"), Some("home")),
            "work"
        );
        assert_eq!(resolve_profile_name(None, Some("ci"), Some("home")), "ci");
        assert_eq!(resolve_profile_name(None, None, Some("home")), "home");
        assert_eq!(resolve_profile_name(None, None, None), "default");
    }
}
