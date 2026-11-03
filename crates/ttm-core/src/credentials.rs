//! Where the token is kept.
//!
//! Order on read: `TTM_TOKEN` (wins over everything) > OS keyring (service `ttm`, account `<profile>@<host>`) >
//! `credentials.toml` (mode 0600). On write the keyring is tried first; when it is unavailable (headless Linux,
//! containers) or disabled with `TTM_NO_KEYRING=1`, the file is used. Secrets never go to logs: [`StoredToken`]'s
//! `Debug` masks them.

use crate::paths::write_atomic;
use crate::{Error, Paths, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use time::OffsetDateTime;

/// Keyring service name.
pub const KEYRING_SERVICE: &str = "ttm";

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub expires_at: Option<OffsetDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl StoredToken {
    pub fn bare(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            refresh_token: None,
            expires_at: None,
            scope: None,
        }
    }

    pub fn is_expired(&self, now: OffsetDateTime) -> bool {
        self.expires_at.is_some_and(|at| at <= now)
    }

    /// `ttm_pat_AbCd…` → `ttm_pat_AbCd…` cut to the prefix plus 4 characters, for display.
    pub fn masked(&self) -> String {
        mask(&self.access_token)
    }
}

impl std::fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredToken")
            .field("access_token", &self.masked())
            .field("refresh_token", &self.refresh_token.as_ref().map(|t| mask(t)))
            .field("expires_at", &self.expires_at)
            .field("scope", &self.scope)
            .finish()
    }
}

/// Keeps the `ttm_xxx_` prefix and at most 4 characters, never more than half of the secret part:
/// `ttm_pat_AbCd…`. Other strings keep at most 4 characters and half their length.
pub fn mask(token: &str) -> String {
    let (prefix, secret) = if token.starts_with("ttm_") && token.len() > 8 && token.is_char_boundary(8) {
        token.split_at(8)
    } else {
        ("", token)
    };
    let keep = (secret.chars().count() / 2).min(4);
    let visible: String = secret.chars().take(keep).collect();
    format!("{prefix}{visible}…")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenSource {
    /// `TTM_TOKEN` environment variable.
    Env,
    Keyring,
    File,
}

impl std::fmt::Display for TokenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TokenSource::Env => "TTM_TOKEN",
            TokenSource::Keyring => "keyring",
            TokenSource::File => "credentials.toml",
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CredentialsFile {
    #[serde(default)]
    credentials: BTreeMap<String, StoredToken>,
}

/// Reads and writes tokens for `(profile, host)`.
#[derive(Debug, Clone)]
pub struct CredentialStore {
    file: PathBuf,
    use_keyring: bool,
    env_token: Option<String>,
}

impl CredentialStore {
    /// The store for this process: `TTM_TOKEN` and `TTM_NO_KEYRING` are read from the environment.
    pub fn from_env(paths: &Paths) -> Self {
        let env_token = std::env::var("TTM_TOKEN").ok().filter(|t| !t.trim().is_empty());
        let no_keyring = std::env::var("TTM_NO_KEYRING").is_ok_and(|v| !v.is_empty() && v != "0");
        Self::new(paths, !no_keyring, env_token)
    }

    pub fn new(paths: &Paths, use_keyring: bool, env_token: Option<String>) -> Self {
        Self {
            file: paths.credentials_file(),
            use_keyring,
            env_token,
        }
    }

    /// `<profile>@<host>`, the keyring account and the key in `credentials.toml`.
    pub fn account(profile: &str, host: &str) -> String {
        format!("{profile}@{host}")
    }

    pub fn load(&self, profile: &str, host: &str) -> Result<Option<(StoredToken, TokenSource)>> {
        if let Some(token) = &self.env_token {
            return Ok(Some((StoredToken::bare(token.trim()), TokenSource::Env)));
        }
        let account = Self::account(profile, host);
        if self.use_keyring {
            match keyring_get(&account) {
                Ok(Some(token)) => return Ok(Some((token, TokenSource::Keyring))),
                Ok(None) => {}
                Err(err) => tracing::debug!("keyring read failed, falling back to file: {err}"),
            }
        }
        Ok(self
            .read_file()?
            .credentials
            .remove(&account)
            .map(|t| (t, TokenSource::File)))
    }

    /// Saves to the keyring, or to the file when the keyring is off or fails. Returns where it went.
    pub fn save(&self, profile: &str, host: &str, token: &StoredToken) -> Result<TokenSource> {
        let account = Self::account(profile, host);
        if self.use_keyring {
            match keyring_set(&account, token) {
                Ok(()) => {
                    // Do not leave an older copy in the file.
                    self.remove_from_file(&account)?;
                    return Ok(TokenSource::Keyring);
                }
                Err(err) => {
                    tracing::warn!("keyring unavailable ({err}); saving the token to credentials.toml")
                }
            }
        }
        let mut file = self.read_file()?;
        file.credentials.insert(account, token.clone());
        self.write_file(&file)?;
        Ok(TokenSource::File)
    }

    /// Removes the token from both the keyring and the file. Returns whether anything was removed.
    pub fn delete(&self, profile: &str, host: &str) -> Result<bool> {
        let account = Self::account(profile, host);
        let mut removed = false;
        if self.use_keyring {
            match keyring_delete(&account) {
                Ok(found) => removed |= found,
                Err(err) => tracing::debug!("keyring delete failed: {err}"),
            }
        }
        removed |= self.remove_from_file(&account)?;
        Ok(removed)
    }

    /// Whether the OS keyring can be used here (for `ttm doctor`).
    pub fn keyring_status(&self) -> std::result::Result<(), String> {
        if !self.use_keyring {
            return Err("disabled (TTM_NO_KEYRING)".into());
        }
        keyring::Entry::store_status()
            .as_ref()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn has_env_token(&self) -> bool {
        self.env_token.is_some()
    }

    fn read_file(&self) -> Result<CredentialsFile> {
        match std::fs::read_to_string(&self.file) {
            Ok(text) => toml::from_str(&text).map_err(|err| Error::Toml {
                path: self.file.clone(),
                message: err.to_string(),
            }),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(CredentialsFile::default()),
            Err(source) => Err(Error::Read {
                path: self.file.clone(),
                source,
            }),
        }
    }

    fn write_file(&self, file: &CredentialsFile) -> Result<()> {
        let text = toml::to_string_pretty(file).map_err(|err| Error::Toml {
            path: self.file.clone(),
            message: err.to_string(),
        })?;
        write_atomic(&self.file, text.as_bytes(), Some(0o600))
    }

    fn remove_from_file(&self, account: &str) -> Result<bool> {
        if !self.file.exists() {
            return Ok(false);
        }
        let mut file = self.read_file()?;
        let removed = file.credentials.remove(account).is_some();
        if removed {
            self.write_file(&file)?;
        }
        Ok(removed)
    }
}

fn keyring_get(account: &str) -> std::result::Result<Option<StoredToken>, keyring::Error> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, account)?;
    match entry.get_password() {
        Ok(json) => Ok(serde_json::from_str(&json)
            .ok()
            .or_else(|| Some(StoredToken::bare(json)))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

fn keyring_set(account: &str, token: &StoredToken) -> std::result::Result<(), keyring::Error> {
    let json = serde_json::to_string(token).unwrap_or_else(|_| token.access_token.clone());
    keyring::Entry::new(KEYRING_SERVICE, account)?.set_password(&json)
}

fn keyring_delete(account: &str) -> std::result::Result<bool, keyring::Error> {
    match keyring::Entry::new(KEYRING_SERVICE, account)?.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &std::path::Path, env: Option<&str>) -> CredentialStore {
        CredentialStore::new(&Paths::under(dir), false, env.map(String::from))
    }

    #[test]
    fn file_round_trip_and_delete() {
        let dir = tempfile::tempdir().unwrap();
        let store = store(dir.path(), None);
        assert!(store.load("default", "https://a.com").unwrap().is_none());
        let token = StoredToken {
            access_token: "ttm_oat_abcdefghijklmnop".into(),
            refresh_token: Some("ttm_ort_zzz".into()),
            expires_at: Some(OffsetDateTime::from_unix_timestamp(2_000_000_000).unwrap()),
            scope: Some("workspace:read".into()),
        };
        assert_eq!(
            store.save("default", "https://a.com", &token).unwrap(),
            TokenSource::File
        );
        let (loaded, source) = store.load("default", "https://a.com").unwrap().unwrap();
        assert_eq!((loaded, source), (token, TokenSource::File));
        assert!(
            store.load("work", "https://a.com").unwrap().is_none(),
            "keyed by profile"
        );
        assert!(store.delete("default", "https://a.com").unwrap());
        assert!(store.load("default", "https://a.com").unwrap().is_none());
        assert!(!store.delete("default", "https://a.com").unwrap());
    }

    #[test]
    fn env_token_wins() {
        let dir = tempfile::tempdir().unwrap();
        let file_store = store(dir.path(), None);
        file_store
            .save("default", "https://a.com", &StoredToken::bare("ttm_pat_file"))
            .unwrap();
        let env_store = store(dir.path(), Some(" ttm_pat_env "));
        let (token, source) = env_store.load("default", "https://a.com").unwrap().unwrap();
        assert_eq!(
            (token.access_token.as_str(), source),
            ("ttm_pat_env", TokenSource::Env)
        );
    }

    #[cfg(unix)]
    #[test]
    fn credentials_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        store(dir.path(), None)
            .save("p", "https://a.com", &StoredToken::bare("ttm_pat_x"))
            .unwrap();
        let mode = std::fs::metadata(Paths::under(dir.path()).credentials_file())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn debug_masks_secrets() {
        let token = StoredToken {
            access_token: "ttm_pat_SECRETSECRETSECRET".into(),
            refresh_token: Some("ttm_ort_SECRETSECRET".into()),
            expires_at: None,
            scope: None,
        };
        let debug = format!("{token:?}");
        assert!(!debug.contains("SECRETSECRET"), "{debug}");
        assert_eq!(mask("ttm_pat_ABCDEFGH"), "ttm_pat_ABCD…");
        assert_eq!(mask("short"), "sh…");
        assert_eq!(
            mask("ttm_pat_good"),
            "ttm_pat_go…",
            "a short secret is never shown whole"
        );
    }

    #[test]
    fn expiry() {
        let now = OffsetDateTime::from_unix_timestamp(1_000).unwrap();
        let mut token = StoredToken::bare("t");
        assert!(!token.is_expired(now));
        token.expires_at = Some(OffsetDateTime::from_unix_timestamp(999).unwrap());
        assert!(token.is_expired(now));
    }
}
