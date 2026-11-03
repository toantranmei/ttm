//! `config.toml`:
//!
//! ```toml
//! default_profile = "work"
//!
//! [profiles.default]
//! host = "https://toantranmei.com"
//!
//! [profiles.work]
//! host = "http://localhost:3000"
//! workspace = "3f0c…"
//! ```

use crate::paths::write_atomic;
use crate::profile::Profile;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_profile: Option<String>,
    /// Where task and resolve worktrees go (default `~/.ttm/worktrees`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_root: Option<String>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}

impl Config {
    /// Reads the file; a missing file is an empty config.
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text, path),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(Error::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub fn parse(text: &str, path: &Path) -> Result<Self> {
        toml::from_str(text).map_err(|err| Error::Toml {
            path: path.to_path_buf(),
            message: err.to_string(),
        })
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self).map_err(|err| Error::Toml {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?;
        write_atomic(path, text.as_bytes(), Some(0o644))
    }

    /// Root directory of worktrees.
    pub fn worktree_root(&self) -> Option<std::path::PathBuf> {
        match &self.worktree_root {
            Some(root) => Some(crate::worktree::expand_home(root)),
            None => crate::worktree::default_root(),
        }
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// The profile, created empty when missing.
    pub fn profile_mut(&mut self, name: &str) -> &mut Profile {
        self.profiles.entry(name.to_string()).or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            Config::load(&dir.path().join("nope.toml")).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = Config {
            default_profile: Some("work".into()),
            ..Config::default()
        };
        config.profile_mut("work").host = Some("http://localhost:3000".into());
        config.profile_mut("work").workspace = Some("ws-1".into());
        config.save(&path).unwrap();
        let loaded = Config::load(&path).unwrap();
        assert_eq!(loaded, config);
        assert_eq!(loaded.profile("work").unwrap().workspace.as_deref(), Some("ws-1"));
    }

    #[test]
    fn parses_documented_example() {
        let text = r#"
default_profile = "work"
[profiles.default]
host = "https://toantranmei.com"
[profiles.work]
host = "http://localhost:3000"
workspace = "abc"
"#;
        let config = Config::parse(text, Path::new("config.toml")).unwrap();
        assert_eq!(config.default_profile.as_deref(), Some("work"));
        assert_eq!(config.profiles.len(), 2);
    }

    #[test]
    fn bad_toml_reports_path() {
        let err = Config::parse("default_profile = [", Path::new("x.toml")).unwrap_err();
        assert!(err.to_string().contains("x.toml"));
    }
}
