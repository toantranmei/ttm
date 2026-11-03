//! `runner.toml` next to `config.toml`:
//!
//! ```toml
//! name = "macbook"
//! max_parallel = 2
//! accept_team_jobs = false
//! worktree_root = "~/orca/workspaces"
//! keep_days = 3
//! runner_id = "…"            # set by the first `ttm runner up`
//!
//! [repos]                    # project key → local clone
//! AUR = "/Users/me/code/aurora"
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerConfig {
    #[serde(default = "hostname")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_parallel: Option<usize>,
    /// Take jobs of teammates' tasks too (not only the tasks you own).
    #[serde(default)]
    pub accept_team_jobs: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_root: Option<String>,
    /// Days to keep finished worktrees.
    #[serde(default = "three")]
    pub keep_days: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_id: Option<String>,
    #[serde(default)]
    pub repos: BTreeMap<String, PathBuf>,
}

fn three() -> u32 {
    3
}

/// Machine name from the environment.
pub fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .filter(|h| !h.is_empty())
        .or_else(|| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|h| !h.is_empty())
        })
        .unwrap_or_else(|| "ttm-runner".into())
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            name: hostname(),
            max_parallel: None,
            accept_team_jobs: false,
            worktree_root: None,
            keep_days: 3,
            runner_id: None,
            repos: BTreeMap::new(),
        }
    }
}

impl RunnerConfig {
    pub fn path(config_dir: &Path) -> PathBuf {
        config_dir.join("runner.toml")
    }

    pub fn load(config_dir: &Path) -> anyhow::Result<Self> {
        match std::fs::read_to_string(Self::path(config_dir)) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self, config_dir: &Path) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(self)?;
        ttm_core::paths::write_atomic(&Self::path(config_dir), text.as_bytes(), Some(0o644))?;
        Ok(())
    }

    /// Local clone for a project key (case-insensitive) or id.
    pub fn repo_for(&self, key: &str, project_id: Option<&str>) -> Option<&PathBuf> {
        self.repos
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key) || Some(k.as_str()) == project_id)
            .map(|(_, v)| v)
    }

    /// Worktree root: runner setting, else `config.toml`'s, else `~/.ttm/worktrees`.
    pub fn worktree_root(&self, fallback: Option<PathBuf>) -> Option<PathBuf> {
        self.worktree_root
            .as_deref()
            .map(ttm_core::worktree::expand_home)
            .or(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_repo_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = RunnerConfig {
            name: "mac".into(),
            max_parallel: Some(2),
            ..RunnerConfig::default()
        };
        c.repos.insert("AUR".into(), PathBuf::from("/code/aurora"));
        c.save(dir.path()).unwrap();
        let loaded = RunnerConfig::load(dir.path()).unwrap();
        assert_eq!(loaded, c);
        assert_eq!(loaded.repo_for("aur", None), Some(&PathBuf::from("/code/aurora")));
        assert_eq!(loaded.repo_for("XYZ", None), None);
        assert_eq!(RunnerConfig::load(&dir.path().join("none")).unwrap().keep_days, 3);
    }
}
