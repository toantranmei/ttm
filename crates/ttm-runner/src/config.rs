//! `runner.toml` next to `config.toml`:
//!
//! ```toml
//! name = "macbook"
//! max_parallel = 2
//! accept_team_jobs = false
//! worktree_root = "~/orca/workspaces"
//! keep_days = 3
//! runner_id = "…"            # set by the first `ttm runner up`
//! auto_merge = false          # merge the head of the merge queue locally and push
//!
//! [repos]                    # project key → local clone
//! AUR = "/Users/me/code/aurora"
//!
//! [mcp.local.github-local]   # local MCP servers (see `local_mcp`)
//! command = "npx"
//! allow = ["dev"]
//! ```

use crate::local_mcp::McpSection;
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
    /// Merge the head of each project's merge queue locally and push it (off by default).
    #[serde(default)]
    pub auto_merge: bool,
    #[serde(default)]
    pub repos: BTreeMap<String, PathBuf>,
    /// Local MCP servers (`[mcp.local.<name>]`).
    #[serde(default, skip_serializing_if = "McpSection::is_empty")]
    pub mcp: McpSection,
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
            auto_merge: false,
            repos: BTreeMap::new(),
            mcp: McpSection::default(),
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

    /// Only the `[mcp]` section of `runner.toml` (fresh from disk: jobs see edits without a restart).
    pub fn load_mcp(config_dir: &Path) -> McpSection {
        Self::load(config_dir).map(|c| c.mcp).unwrap_or_default()
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

    /// The projects announced to the server (`projects` of register and heartbeat): the keys (or ids) of
    /// `[repos]`, trimmed, without empty ones or duplicates (case-insensitive, first spelling kept). The server offers
    /// this runner only their jobs; an empty list means none.
    pub fn announced_projects(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for key in self.repos.keys().map(|k| k.trim()).filter(|k| !k.is_empty()) {
            if !out.iter().any(|seen| seen.eq_ignore_ascii_case(key)) {
                out.push(key.to_string());
            }
        }
        out
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

    #[test]
    fn announced_projects_come_from_the_configured_clones() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            RunnerConfig::path(dir.path()),
            "name = \"mac\"\n[repos]\nAUR = \"/code/aurora\"\naur = \"/code/other\"\nNOTE = \"/code/notes\"\n\" \" = \"/x\"\n",
        )
        .unwrap();
        let loaded = RunnerConfig::load(dir.path()).unwrap();
        assert_eq!(
            loaded.announced_projects(),
            vec!["AUR".to_string(), "NOTE".to_string()]
        );
        assert!(
            RunnerConfig::default().announced_projects().is_empty(),
            "no clone: no project"
        );
    }
}
