//! Git steps of a job (thin wrappers over the `git` and `gh` CLIs).

use std::path::Path;
use ttm_core::worktree::{GitError, git, git_env};

/// Whether the worktree has uncommitted changes.
pub fn is_dirty(dir: &Path) -> Result<bool, GitError> {
    Ok(!git(dir, &["status", "--porcelain"])?.is_empty())
}

/// Commits everything left uncommitted by the agent, with the job's identity and hooks (`env`).
pub fn commit_all(dir: &Path, message: &str, env: &[(String, String)]) -> Result<(), GitError> {
    git_env(dir, &["add", "-A"], env)?;
    git_env(dir, &["commit", "-q", "-m", message], env).map(drop)
}

pub fn head(dir: &Path) -> Result<String, GitError> {
    git(dir, &["rev-parse", "HEAD"])
}

/// Number of commits on `branch` that are not on `base`.
pub fn commits_ahead(dir: &Path, base: &str) -> usize {
    let range = format!("{base}..HEAD");
    git(dir, &["rev-list", "--count", &range])
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

pub fn has_remote(dir: &Path) -> bool {
    git(dir, &["remote", "get-url", "origin"]).is_ok()
}

/// Pushes with the job's environment (its hooks, e.g. `pre-push`, are chained).
pub fn push(dir: &Path, branch: &str, env: &[(String, String)]) -> Result<(), GitError> {
    git_env(dir, &["push", "-u", "origin", branch], env).map(drop)
}

/// Opens a pull request with the `gh` CLI when available; returns its URL.
pub fn open_pull_request(dir: &Path, branch: &str, base: &str, title: &str, body: &str) -> Option<String> {
    let out = std::process::Command::new("gh")
        .current_dir(dir)
        .args([
            "pr", "create", "--head", branch, "--base", base, "--title", title, "--body", body,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        tracing::debug!("gh pr create failed: {}", String::from_utf8_lossy(&out.stderr));
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .rev()
        .find(|l| l.starts_with("http"))
        .map(String::from)
}
