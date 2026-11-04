//! What a runner refuses to run.
//!
//! A job names a repository (`repo.url`, `repo.provider`). The runner only works in clones its owner configured with
//! `ttm runner config --repo KEY=PATH`, and only when that clone's `origin` is the job's repository (for provider
//! `local`, when the job's path is the configured path). It never clones a URL it was sent.

use std::path::{Path, PathBuf};
use ttm_api::factory::JobSpec;

/// Error code reported when a job's repository is not one the owner configured.
pub const REPO_NOT_CONFIGURED: &str = "repo_not_configured";
/// Error code reported when the branch name is not a valid git branch.
pub const INVALID_BRANCH: &str = "invalid_branch";
/// Error code reported when the job's `worktree_hint` would put the worktree outside the runner's `worktree_root`.
pub const INVALID_WORKTREE: &str = "invalid_worktree";

/// `https://user@GitHub.com/a/b.git/`, `git@github.com:a/b.git`, `ssh://git@github.com/a/b` → `github.com/a/b`.
pub fn normalize_url(url: &str) -> String {
    let mut u = url.trim().to_string();
    if let Some(i) = u.find("://") {
        u = u[i + 3..].to_string();
    } else if let Some((host, path)) = u.split_once(':')
        && !host.contains('/')
        && !path.starts_with("//")
    {
        // scp-like `git@host:owner/repo`
        u = format!("{host}/{path}");
    }
    if let Some(at) = u.find('@')
        && u[..at].find('/').is_none()
    {
        u = u[at + 1..].to_string();
    }
    let u = u.trim_end_matches('/');
    let u = u.strip_suffix(".git").unwrap_or(u);
    let (host, path) = u.split_once('/').unwrap_or((u, ""));
    // Drop an explicit port (ssh://host:22/…) and lowercase the host.
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    if path.is_empty() {
        host
    } else {
        format!("{host}/{path}")
    }
}

/// git branch name rules (`git check-ref-format --branch`), checked locally before reporting a run.
pub fn valid_branch(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name != "@"
        && !name.starts_with('-')
        && !name.starts_with('/')
        && !name.ends_with('/')
        && !name.ends_with('.')
        && !name.ends_with(".lock")
        && !name.contains("..")
        && !name.contains("//")
        && !name.contains("@{")
        && !name
            .chars()
            .any(|c| c.is_control() || c == ' ' || "~^:?*[\\".contains(c))
        && name
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.') && !part.ends_with(".lock"))
}

/// Why a job was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Refused {
    pub code: &'static str,
    pub message: String,
}

fn refuse(code: &'static str, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(a) == canon(b)
}

/// Checks the job against the configured clone (`None` when the project has no clone configured). Returns the
/// clone to work in.
pub fn check(
    job: &JobSpec,
    configured: Option<&PathBuf>,
    origin_of: impl Fn(&Path) -> Option<String>,
) -> Result<PathBuf, Refused> {
    let key = job.project_key();
    let hint = format!("add it with `ttm runner config --repo {key}=<path to your clone>`");
    if !valid_branch(&job.branch) {
        return Err(refuse(
            INVALID_BRANCH,
            format!("`{}` is not a valid branch name", job.branch),
        ));
    }
    let Some(repo) = configured else {
        return Err(refuse(
            REPO_NOT_CONFIGURED,
            format!("no local clone is configured for project {key}; {hint}"),
        ));
    };
    let Some(url) = job.repo.url.as_deref().map(str::trim).filter(|u| !u.is_empty()) else {
        return Err(refuse(
            REPO_NOT_CONFIGURED,
            format!("the job for {key} names no repository; refusing to guess"),
        ));
    };
    if job.repo.provider.as_deref() == Some("local") {
        let path = Path::new(url.strip_prefix("file://").unwrap_or(url));
        return if same_path(path, repo) {
            Ok(repo.clone())
        } else {
            Err(refuse(
                REPO_NOT_CONFIGURED,
                format!(
                    "the job's repository {} is not the configured clone {} for {key}; {hint}",
                    path.display(),
                    repo.display()
                ),
            ))
        };
    }
    let Some(origin) = origin_of(repo) else {
        return Err(refuse(
            REPO_NOT_CONFIGURED,
            format!(
                "{} has no `origin` remote to compare with {url}; {hint}",
                repo.display()
            ),
        ));
    };
    if normalize_url(&origin) == normalize_url(url) {
        Ok(repo.clone())
    } else {
        Err(refuse(
            REPO_NOT_CONFIGURED,
            format!(
                "the job's repository {url} is not the origin of {} ({origin}); {hint}",
                repo.display()
            ),
        ))
    }
}

/// Where the job's worktree goes. `worktree_hint` is only a suggestion relative to the runner's `root`
/// (`<project key>/<branch>`): an absolute path, a drive or root prefix, or a `..` step is refused, and so is a hint
/// whose existing part leads outside `root` through a symlink. Without a hint the runner picks
/// `<root>/<project key>/<branch>`.
pub fn worktree_for(job: &JobSpec, root: &Path) -> Result<PathBuf, Refused> {
    use std::path::Component;
    let Some(hint) = job
        .worktree_hint
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
    else {
        return Ok(crate::worktree::worktree_path(
            root,
            &job.project_key(),
            &job.branch,
        ));
    };
    let bad = |why: &str| {
        refuse(
            INVALID_WORKTREE,
            format!(
                "the job's worktree_hint `{hint}` {why}; worktrees stay under the runner's worktree_root {}",
                root.display()
            ),
        )
    };
    // `\` separates on Windows only; refuse it everywhere so a hint means the same on every runner.
    if hint.contains('\\') || hint.contains(':') {
        return Err(bad("contains `\\` or `:`"));
    }
    let mut path = root.to_path_buf();
    let mut depth = 0;
    for part in Path::new(hint).components() {
        match part {
            Component::Normal(name) => {
                path.push(name);
                depth += 1;
            }
            Component::CurDir => {}
            Component::ParentDir => return Err(bad("goes up with `..`")),
            Component::RootDir | Component::Prefix(_) => return Err(bad("is an absolute path")),
        }
    }
    if depth == 0 {
        return Err(bad("names no directory"));
    }
    // A symlink inside the root must not lead elsewhere: compare the deepest existing ancestor.
    let canon = |p: &Path| std::fs::canonicalize(p).ok();
    if let Some(real_root) = canon(root) {
        let existing = path.ancestors().find(|p| p.exists()).and_then(canon);
        if existing.is_some_and(|real| !real.starts_with(&real_root)) {
            return Err(bad("leads outside it through a symlink"));
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn job(url: Option<&str>, provider: &str, branch: &str) -> JobSpec {
        serde_json::from_value(json!({
            "run_id": "r1", "task_key": "AUR-1", "project_key": "AUR", "branch": branch, "prompt": "p",
            "repo": {"url": url, "base_branch": "main", "provider": provider}, "mcp": {"token": "t"}
        }))
        .unwrap()
    }

    #[test]
    fn url_normalization() {
        let want = "github.com/toantranmei/ttm";
        for u in [
            "https://github.com/toantranmei/ttm.git",
            "https://user:pw@GitHub.com/toantranmei/ttm/",
            "git@github.com:toantranmei/ttm.git",
            "ssh://git@github.com:22/toantranmei/ttm",
            "github.com/toantranmei/ttm",
        ] {
            assert_eq!(normalize_url(u), want, "{u}");
        }
        assert_ne!(normalize_url("https://github.com/evil/ttm"), want);
    }

    #[test]
    fn branch_rules() {
        for ok in ["aur-12-login", "ttm/aur-12", "feature/x.y"] {
            assert!(valid_branch(ok), "{ok}");
        }
        for bad in [
            "", "-x", "a..b", "a b", "a~1", "x.lock", "a/.b", "a//b", "a/", "@", "a@{1}", "a:b",
        ] {
            assert!(!valid_branch(bad), "{bad}");
        }
    }

    #[test]
    fn remote_must_match_the_configured_clone() {
        let repo = PathBuf::from("/code/aurora");
        let origin = |_: &Path| Some("git@github.com:toantranmei/aurora.git".to_string());
        let ok = job(Some("https://github.com/toantranmei/aurora"), "github", "aur-1");
        assert_eq!(check(&ok, Some(&repo), origin).unwrap(), repo);
        let other = job(Some("https://github.com/attacker/aurora"), "github", "aur-1");
        assert_eq!(
            check(&other, Some(&repo), origin).unwrap_err().code,
            REPO_NOT_CONFIGURED
        );
        assert_eq!(check(&ok, None, origin).unwrap_err().code, REPO_NOT_CONFIGURED);
        assert_eq!(
            check(&job(None, "github", "aur-1"), Some(&repo), origin)
                .unwrap_err()
                .code,
            REPO_NOT_CONFIGURED
        );
        assert_eq!(
            check(&ok, Some(&repo), |_: &Path| None).unwrap_err().code,
            REPO_NOT_CONFIGURED
        );
        assert_eq!(
            check(
                &job(Some("https://github.com/toantranmei/aurora"), "github", "a b"),
                Some(&repo),
                origin
            )
            .unwrap_err()
            .code,
            INVALID_BRANCH
        );
    }

    #[test]
    fn worktree_hint_stays_under_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("worktrees");
        std::fs::create_dir_all(&root).unwrap();
        let with_hint = |hint: Option<&str>| {
            let mut j = job(Some("https://github.com/a/b"), "github", "ttm/aur-1-login");
            j.worktree_hint = hint.map(str::to_owned);
            worktree_for(&j, &root)
        };
        assert_eq!(
            with_hint(Some("aur/aur-1-login")).unwrap(),
            root.join("aur").join("aur-1-login")
        );
        assert_eq!(with_hint(Some("./aur/x")).unwrap(), root.join("aur").join("x"));
        assert_eq!(with_hint(None).unwrap(), root.join("AUR").join("ttm-aur-1-login"));
        assert_eq!(
            with_hint(Some("  ")).unwrap(),
            root.join("AUR").join("ttm-aur-1-login")
        );
        for bad in [
            "/etc/x",
            "../outside",
            "aur/../../x",
            r"C:\x",
            r"aur\..\x",
            ".",
            "~/../x/..",
        ] {
            let err = with_hint(Some(bad)).unwrap_err();
            assert_eq!(err.code, INVALID_WORKTREE, "{bad}");
            assert!(err.message.contains("worktree_root"), "{}", err.message);
        }
        #[cfg(unix)]
        {
            let outside = dir.path().join("elsewhere");
            std::fs::create_dir_all(&outside).unwrap();
            std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
            assert_eq!(with_hint(Some("link/x")).unwrap_err().code, INVALID_WORKTREE);
        }
    }

    #[test]
    fn local_provider_compares_paths() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let none = |_: &Path| None;
        let same = job(Some(&repo.display().to_string()), "local", "aur-1");
        assert!(check(&same, Some(&repo), none).is_ok());
        let other = job(Some("/somewhere/else"), "local", "aur-1");
        assert_eq!(
            check(&other, Some(&repo), none).unwrap_err().code,
            REPO_NOT_CONFIGURED
        );
    }
}
