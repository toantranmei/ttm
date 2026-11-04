//! Git worktrees for tasks and conflict resolution: `<root>/<project>/<branch>`, root `~/.ttm/worktrees` by default
//! or `worktree_root` in `config.toml` (e.g. `~/orca/workspaces`).

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("git {args} failed: {stderr}")]
    Failed { args: String, stderr: String },
    #[error("cannot run git: {0}")]
    Spawn(#[from] std::io::Error),
}

/// Default root: `~/.ttm/worktrees`.
pub fn default_root() -> Option<PathBuf> {
    crate::paths::home_dir().map(|home| home.join(".ttm").join("worktrees"))
}

/// `~/x` → home-relative; anything else as is.
pub fn expand_home(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), crate::paths::home_dir()) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(path),
    }
}

/// `<root>/<project>/<branch with / replaced>`.
pub fn worktree_path(root: &Path, project: &str, branch: &str) -> PathBuf {
    root.join(sanitize(project)).join(sanitize(branch))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Runs git in `dir`, returning trimmed stdout.
pub fn git(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(GitError::Failed {
            args: args.join(" "),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }
}

/// Like [`git`], with extra environment variables (commit identity, `core.hooksPath` through `GIT_CONFIG_*`).
pub fn git_env(dir: &Path, args: &[&str], env: &[(String, String)]) -> Result<String, GitError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(GitError::Failed {
            args: args.join(" "),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }
}

/// Current branch name (`HEAD` when detached).
pub fn current_branch(dir: &Path) -> Result<String, GitError> {
    git(dir, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// `origin/HEAD`'s branch, else `main` if it exists, else `master`.
pub fn default_branch(dir: &Path) -> String {
    if let Ok(r) = git(dir, &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]) {
        return r.trim_start_matches("origin/").to_string();
    }
    if git(dir, &["rev-parse", "--verify", "--quiet", "main"]).is_ok() {
        "main".into()
    } else {
        "master".into()
    }
}

/// Creates `path` on a new `branch` from `origin/<base>` (after a fetch), falling back to the local `<base>`.
pub fn add(repo: &Path, path: &Path, branch: &str, base: &str) -> Result<(), GitError> {
    let _ = git(repo, &["fetch", "--quiet", "origin", base]);
    let remote = format!("origin/{base}");
    let start = if git(repo, &["rev-parse", "--verify", "--quiet", &remote]).is_ok() {
        remote
    } else {
        base.to_string()
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let path_str = path.to_string_lossy();
    git(repo, &["worktree", "add", "-B", branch, &path_str, &start]).map(drop)
}

/// Removes the worktree (forced) and prunes.
pub fn remove(repo: &Path, path: &Path) -> Result<(), GitError> {
    let path_str = path.to_string_lossy();
    git(repo, &["worktree", "remove", "--force", &path_str])?;
    let _ = git(repo, &["worktree", "prune"]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_sanitized() {
        let p = worktree_path(Path::new("/w"), "AUR", "ttm/resolve-12");
        assert_eq!(p, PathBuf::from("/w/AUR/ttm-resolve-12"));
    }

    #[test]
    fn home_expansion() {
        assert_eq!(expand_home("/abs/x"), PathBuf::from("/abs/x"));
        assert!(!expand_home("~/x").starts_with("~"));
    }

    #[test]
    fn worktree_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        if git(&repo, &["init", "-q", "-b", "main"]).is_err() {
            return; // git missing on this machine
        }
        let _ = git(
            &repo,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "init",
            ],
        );
        assert_eq!(default_branch(&repo), "main");
        let wt = worktree_path(&dir.path().join("wt"), "AUR", "ttm/resolve-1");
        add(&repo, &wt, "ttm/resolve-1", "main").unwrap();
        assert_eq!(current_branch(&wt).unwrap(), "ttm/resolve-1");
        remove(&repo, &wt).unwrap();
        assert!(!wt.exists());
    }
}
