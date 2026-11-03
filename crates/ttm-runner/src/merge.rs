//! Local merge of the head of a project's merge queue (`auto_merge = true`): merge the run's branch into the base
//! branch in a scratch worktree, push, and report `merged` or `conflict` (the server then creates a "resolve" task).

use crate::worktree;
use std::path::Path;
use ttm_api::factory::MergeEntry;
use ttm_core::worktree::git;

/// Outcome of one merge attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merged {
    Merged { commit: String },
    Conflict { detail: String },
}

/// Commit message of the merge.
pub fn merge_message(entry: &MergeEntry) -> String {
    format!("merge: {} ({})", entry.task_key, entry.branch)
}

/// Merges `origin/<entry.branch>` into `origin/<base>` in a worktree under `root` and pushes `HEAD:<base>`.
pub fn merge_entry(
    repo: &Path,
    root: &Path,
    project_key: &str,
    base: &str,
    entry: &MergeEntry,
) -> anyhow::Result<Merged> {
    let short: String = entry.id.chars().take(8).collect();
    let branch = format!("ttm/merge-{short}");
    let path = worktree::worktree_path(root, project_key, &branch);
    git(repo, &["fetch", "--quiet", "origin", base, &entry.branch])?;
    worktree::add(repo, &path, &branch, base)?;
    let remote_branch = format!("origin/{}", entry.branch);
    let message = merge_message(entry);
    let result = match git(
        &path,
        &["merge", "--no-ff", "--no-edit", "-m", &message, &remote_branch],
    ) {
        Ok(_) => {
            let target = format!("HEAD:{base}");
            git(&path, &["push", "origin", &target])?;
            Ok(Merged::Merged {
                commit: git(&path, &["rev-parse", "HEAD"])?,
            })
        }
        Err(err) => {
            let _ = git(&path, &["merge", "--abort"]);
            let files = git(&path, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
            let detail = if files.is_empty() {
                err.to_string()
            } else {
                format!("conflicting files: {}", files.replace('\n', ", "))
            };
            Ok(Merged::Conflict { detail })
        }
    };
    let _ = worktree::remove(repo, &path);
    let _ = git(repo, &["branch", "-D", &branch]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> MergeEntry {
        serde_json::from_value(serde_json::json!({
            "id": "0192aabbccdd", "run_id": "r1", "task_key": "AUR-12", "branch": "aur-12-login", "status": "ready", "can_merge": true
        }))
        .unwrap()
    }

    #[test]
    fn message() {
        assert_eq!(merge_message(&entry()), "merge: AUR-12 (aur-12-login)");
    }

    #[test]
    fn merges_and_detects_conflicts_against_a_local_origin() {
        let dir = tempfile::tempdir().unwrap();
        let origin = dir.path().join("origin.git");
        let repo = dir.path().join("repo");
        let run = |d: &Path, args: &[&str]| git(d, args);
        std::fs::create_dir_all(&origin).unwrap();
        if run(&origin, &["init", "-q", "--bare", "-b", "main"]).is_err() {
            return; // git missing
        }
        std::fs::create_dir_all(&repo).unwrap();
        let cfg = ["-c", "user.email=t@t", "-c", "user.name=t"];
        run(&repo, &["init", "-q", "-b", "main"]).unwrap();
        run(&repo, &["remote", "add", "origin", &origin.display().to_string()]).unwrap();
        std::fs::write(repo.join("a.txt"), "base\n").unwrap();
        run(&repo, &["add", "."]).unwrap();
        run(
            &repo,
            &[cfg[0], cfg[1], cfg[2], cfg[3], "commit", "-q", "-m", "init"],
        )
        .unwrap();
        run(&repo, &["push", "-q", "origin", "main"]).unwrap();
        run(&repo, &["checkout", "-q", "-b", "aur-12-login"]).unwrap();
        std::fs::write(repo.join("b.txt"), "feature\n").unwrap();
        run(&repo, &["add", "."]).unwrap();
        run(
            &repo,
            &[cfg[0], cfg[1], cfg[2], cfg[3], "commit", "-q", "-m", "feat"],
        )
        .unwrap();
        run(&repo, &["push", "-q", "origin", "aur-12-login"]).unwrap();
        run(&repo, &["checkout", "-q", "main"]).unwrap();
        // The merge commit needs an identity inside the worktree.
        run(&repo, &["config", "user.email", "t@t"]).unwrap();
        run(&repo, &["config", "user.name", "t"]).unwrap();
        let root = dir.path().join("wt");
        let merged = merge_entry(&repo, &root, "AUR", "main", &entry()).unwrap();
        assert!(matches!(merged, Merged::Merged { .. }), "{merged:?}");
    }
}
