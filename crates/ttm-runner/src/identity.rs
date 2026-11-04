//! Commit identity of a job and the `commit-msg` hook every job goes through.
//!
//! - With `job.git` the agent's author/committer name and email are set through `GIT_AUTHOR_*` / `GIT_COMMITTER_*`.
//! - Every job (with or without `git`, whatever the workspace's `commit_identity`) runs git with
//!   `core.hooksPath` pointing at a per-run hooks directory (through `GIT_CONFIG_COUNT/KEY/VALUE`, so no config file
//!   is touched). Its `commit-msg` removes every Claude/Anthropic attribution line, adds the job's trailers (only
//!   `delegator_with_trailer` has some: the agent's own name and email), then runs the repository's own
//!   `commit-msg`. The repository's other hooks are chained by small shims.
//!
//! PO rule: no commit made by a runner ever carries a Claude/Anthropic line.

use std::path::{Path, PathBuf};
use ttm_api::factory::JobGit;
use ttm_core::worktree::git;

/// The lines removed from every commit message (POSIX extended regex, case-insensitive), one per pattern.
pub const STRIP_PATTERNS: [&str; 4] = [
    "^co-authored-by:.*(claude|anthropic)",
    "generated (with|by) .*claude",
    "🤖 generated with",
    "noreply@anthropic\\.com",
];

/// Environment of git (and of the agent, which runs git) for a job.
pub fn env(git: Option<&JobGit>, hooks_dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(g) = git {
        let pairs = [
            ("GIT_AUTHOR_NAME", &g.author_name),
            ("GIT_AUTHOR_EMAIL", &g.author_email),
            (
                "GIT_COMMITTER_NAME",
                if g.committer_name.is_empty() {
                    &g.author_name
                } else {
                    &g.committer_name
                },
            ),
            (
                "GIT_COMMITTER_EMAIL",
                if g.committer_email.is_empty() {
                    &g.author_email
                } else {
                    &g.committer_email
                },
            ),
        ];
        for (k, v) in pairs {
            if !v.is_empty() {
                out.push((k.to_string(), v.clone()));
            }
        }
    }
    out.push(("GIT_CONFIG_COUNT".into(), "1".into()));
    out.push(("GIT_CONFIG_KEY_0".into(), "core.hooksPath".into()));
    out.push(("GIT_CONFIG_VALUE_0".into(), hooks_dir.display().to_string()));
    out
}

/// Single-quotes `s` for `sh`.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The repository's own hooks directory: `core.hooksPath` (relative to the worktree root) or
/// `<git-common-dir>/hooks`.
pub fn original_hooks_dir(worktree: &Path) -> Option<PathBuf> {
    if let Ok(path) = git(worktree, &["config", "core.hooksPath"])
        && !path.is_empty()
    {
        let p = ttm_core::worktree::expand_home(&path);
        return Some(if p.is_absolute() {
            p
        } else {
            let top = git(worktree, &["rev-parse", "--show-toplevel"]).ok()?;
            Path::new(&top).join(p)
        });
    }
    let common = git(worktree, &["rev-parse", "--git-common-dir"]).ok()?;
    let common = PathBuf::from(common);
    let common = if common.is_absolute() {
        common
    } else {
        worktree.join(common)
    };
    Some(common.join("hooks"))
}

/// The `commit-msg` script: strip, add trailers, chain.
pub fn commit_msg_script(trailers: &[String], original: Option<&Path>) -> String {
    let pattern = STRIP_PATTERNS.join("|");
    let mut s = String::from(
        "#!/bin/sh\n# ttm runner: removes Claude/Anthropic attribution lines, adds the job's trailers, then runs the\n# repository's own commit-msg hook.\nmsg=\"$1\"\ntmp=\"$msg.ttm-tmp\"\n",
    );
    s.push_str(&format!(
        "grep -viE {} \"$msg\" > \"$tmp\" || true\ncat \"$tmp\" > \"$msg\"\nrm -f \"$tmp\"\n",
        sh_quote(&pattern)
    ));
    for t in trailers {
        s.push_str(&format!(
            "git interpret-trailers --in-place --if-exists addIfDifferent --trailer {} \"$msg\"\n",
            sh_quote(t)
        ));
    }
    if let Some(orig) = original {
        let hook = orig.join("commit-msg");
        s.push_str(&format!(
            "if [ -x {q} ]; then exec {q} \"$@\"; fi\n",
            q = sh_quote(&hook.display().to_string())
        ));
    }
    s.push_str("exit 0\n");
    s
}

fn write_exec(path: &Path, text: &str) -> std::io::Result<()> {
    std::fs::write(path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    true
}

/// The per-run hooks directory; removed when dropped.
#[derive(Debug)]
pub struct Hooks {
    pub dir: PathBuf,
}

impl Drop for Hooks {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Creates `<cache>/hooks/<run_id>/` with the `commit-msg` hook and a shim for every other executable hook of the
/// repository (`.sample` files are skipped).
pub fn prepare_hooks(
    worktree: &Path,
    cache_dir: &Path,
    run_id: &str,
    trailers: &[String],
) -> std::io::Result<Hooks> {
    let dir = cache_dir.join("hooks").join(sanitize(run_id));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let original = original_hooks_dir(worktree).filter(|d| d.is_dir() && *d != dir);
    if let Some(orig) = &original {
        for entry in std::fs::read_dir(orig)?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "commit-msg" || name.ends_with(".sample") || !executable(&entry.path()) {
                continue;
            }
            let shim = format!(
                "#!/bin/sh\n# ttm runner: the repository's own {name} hook.\nexec {} \"$@\"\n",
                sh_quote(&entry.path().display().to_string())
            );
            write_exec(&dir.join(&name), &shim)?;
        }
    }
    write_exec(
        &dir.join("commit-msg"),
        &commit_msg_script(trailers, original.as_deref()),
    )?;
    Ok(Hooks { dir })
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// Claude Code settings given to every job with `--settings`: no co-author line, no attribution in commits or PRs.
pub fn claude_settings() -> serde_json::Value {
    serde_json::json!({ "includeCoAuthoredBy": false, "attribution": { "commit": "", "pr": "" } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_with_and_without_identity() {
        let g = JobGit {
            author_name: "DEV · Code".into(),
            author_email: "dev+aurora@agents.toantranmei.com".into(),
            ..JobGit::default()
        };
        let e = env(Some(&g), Path::new("/c/hooks/r1"));
        assert!(e.contains(&("GIT_COMMITTER_NAME".into(), "DEV · Code".into())));
        assert!(e.contains(&("GIT_CONFIG_VALUE_0".into(), "/c/hooks/r1".into())));
        let e = env(None, Path::new("/h"));
        assert_eq!(e.len(), 3);
        assert!(e.iter().all(|(k, _)| k.starts_with("GIT_CONFIG")));
    }

    #[test]
    fn script_shape() {
        let s = commit_msg_script(
            &["Co-authored-by: DEV <dev@x>".into()],
            Some(Path::new("/repo/.git/hooks")),
        );
        assert!(s.contains("grep -viE"));
        assert!(s.contains("--trailer 'Co-authored-by: DEV <dev@x>'"));
        assert!(s.contains("exec '/repo/.git/hooks/commit-msg'"));
        assert_eq!(sh_quote("a'b"), "'a'\\''b'");
    }
}
