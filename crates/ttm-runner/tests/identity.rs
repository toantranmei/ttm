//! Commit identity and the commit-msg hook in a real (temporary) git repository.

#![cfg(unix)]

use std::path::Path;
use ttm_api::factory::JobGit;
use ttm_core::worktree::{git, git_env};
use ttm_runner::identity;

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]).unwrap();
    git(dir.path(), &["config", "user.name", "Machine Owner"]).unwrap();
    git(dir.path(), &["config", "user.email", "owner@example.com"]).unwrap();
    dir
}

fn commit(dir: &Path, env: &[(String, String)], message: &str) -> String {
    std::fs::write(dir.join("f.txt"), message).unwrap();
    git_env(dir, &["add", "-A"], env).unwrap();
    git_env(dir, &["commit", "-q", "-m", message], env).unwrap();
    git(dir, &["log", "-1", "--format=%an <%ae>|%cn <%ce>|%B"]).unwrap()
}

const CLAUDE: &str = "Fix login\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>";

fn agent_git(trailers: Vec<String>) -> JobGit {
    JobGit {
        author_name: "DEV · Code".into(),
        author_email: "dev+aurora@agents.toantranmei.com".into(),
        committer_name: "DEV · Code".into(),
        committer_email: "dev+aurora@agents.toantranmei.com".into(),
        trailers,
    }
}

#[test]
fn agent_identity_and_no_claude_lines() {
    let repo = repo();
    let cache = tempfile::tempdir().unwrap();
    let hooks = identity::prepare_hooks(repo.path(), cache.path(), "r1", &[]).unwrap();
    let env = identity::env(Some(&agent_git(vec![])), &hooks.dir);
    let log = commit(repo.path(), &env, CLAUDE);
    assert!(
        log.starts_with("DEV · Code <dev+aurora@agents.toantranmei.com>|DEV · Code"),
        "{log}"
    );
    assert!(
        !log.to_lowercase().contains("claude") && !log.contains("anthropic"),
        "{log}"
    );
    assert!(log.contains("Fix login"));
}

#[test]
fn delegator_with_trailer_adds_the_agent_once() {
    let repo = repo();
    let cache = tempfile::tempdir().unwrap();
    let trailer = "Co-authored-by: DEV · Code <dev+aurora@agents.toantranmei.com>".to_string();
    let hooks =
        identity::prepare_hooks(repo.path(), cache.path(), "r2", std::slice::from_ref(&trailer)).unwrap();
    let delegator = JobGit {
        author_name: "An".into(),
        author_email: "an@example.com".into(),
        committer_name: "An".into(),
        committer_email: "an@example.com".into(),
        trailers: vec![trailer.clone()],
    };
    let env = identity::env(Some(&delegator), &hooks.dir);
    let log = commit(repo.path(), &env, &format!("{CLAUDE}\n{trailer}"));
    assert_eq!(log.matches(&trailer).count(), 1, "{log}");
    assert!(!log.contains("anthropic"), "{log}");
}

#[test]
fn person_task_keeps_the_machine_identity_but_still_strips() {
    let repo = repo();
    let cache = tempfile::tempdir().unwrap();
    let hooks = identity::prepare_hooks(repo.path(), cache.path(), "r3", &[]).unwrap();
    let env = identity::env(None, &hooks.dir);
    let log = commit(repo.path(), &env, CLAUDE);
    assert!(log.starts_with("Machine Owner <owner@example.com>"), "{log}");
    assert!(!log.contains("anthropic"), "{log}");
}

#[test]
fn the_repository_commit_msg_hook_still_runs() {
    use std::os::unix::fs::PermissionsExt;
    let repo = repo();
    let hook = repo.path().join(".git/hooks/commit-msg");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'Signed-off-by: Hook <hook@example.com>' >> \"$1\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cache = tempfile::tempdir().unwrap();
    let hooks = identity::prepare_hooks(repo.path(), cache.path(), "r4", &[]).unwrap();
    let dir = hooks.dir.clone();
    let env = identity::env(Some(&agent_git(vec![])), &hooks.dir);
    let log = commit(repo.path(), &env, CLAUDE);
    assert!(log.contains("Signed-off-by: Hook"), "{log}");
    drop(hooks);
    assert!(!dir.exists(), "the per-run hooks are removed");
}

#[test]
fn commits_made_around_the_hook_are_cleaned_before_push() {
    let repo = repo();
    std::fs::write(repo.path().join("base.txt"), "x").unwrap();
    git(repo.path(), &["add", "-A"]).unwrap();
    git(repo.path(), &["commit", "-q", "-m", "base"]).unwrap();
    git(repo.path(), &["checkout", "-q", "-b", "aur-1"]).unwrap();
    // `--no-verify` skips every hook, so the attribution lines land in the commit.
    std::fs::write(repo.path().join("f.txt"), "1").unwrap();
    git(repo.path(), &["add", "-A"]).unwrap();
    git(repo.path(), &["commit", "-q", "--no-verify", "-m", CLAUDE]).unwrap();
    assert_eq!(identity::tainted_commits(repo.path(), "main").len(), 1);
    let author_before = git(repo.path(), &["log", "-1", "--format=%an %ae %ad"]).unwrap();
    let n = identity::sanitize_history(repo.path(), "main", &[]).unwrap();
    assert_eq!(n, 1);
    assert!(identity::tainted_commits(repo.path(), "main").is_empty());
    let log = git(repo.path(), &["log", "-1", "--format=%B"]).unwrap();
    assert!(
        log.contains("Fix login") && !log.to_lowercase().contains("claude"),
        "{log}"
    );
    assert_eq!(
        git(repo.path(), &["log", "-1", "--format=%an %ae %ad"]).unwrap(),
        author_before
    );
    assert!(
        git(repo.path(), &["for-each-ref", "refs/original/"])
            .unwrap()
            .is_empty()
    );
    // Nothing to do the second time.
    assert_eq!(identity::sanitize_history(repo.path(), "main", &[]).unwrap(), 0);
}

#[test]
fn original_hook_runs_first_and_cannot_add_attribution() {
    let repo = repo();
    let own = repo.path().join(".git/hooks/commit-msg");
    std::fs::write(
        &own,
        "#!/bin/sh\nprintf '\\nCo-Authored-By: Claude <noreply@anthropic.com>\\n' >> \"$1\"\n",
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&own, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cache = tempfile::tempdir().unwrap();
    let bad = "Co-authored-by: Claude <noreply@anthropic.com>".to_string();
    let good = "Co-authored-by: DEV · Code <dev+aurora@agents.toantranmei.com>".to_string();
    let hooks = identity::prepare_hooks(repo.path(), cache.path(), "r1", &[bad, good]).unwrap();
    let env = identity::env(None, &hooks.dir);
    let log = commit(repo.path(), &env, "Fix login");
    assert!(!log.to_lowercase().contains("anthropic"), "{log}");
    assert_eq!(log.matches("DEV · Code").count(), 1, "{log}");
}

#[test]
fn attribution_lines_in_rust() {
    assert!(identity::is_attribution_line(
        "Co-Authored-By: Claude <noreply@anthropic.com>"
    ));
    assert!(identity::is_attribution_line(
        "🤖 Generated with [Claude Code](https://claude.com/claude-code)"
    ));
    assert!(identity::is_attribution_line(
        "generated by an agent using Claude"
    ));
    assert!(!identity::is_attribution_line(
        "Co-authored-by: DEV · Code <dev@agents.toantranmei.com>"
    ));
    assert_eq!(
        identity::strip_text("Fix\n\nCo-Authored-By: Claude <x>\n"),
        "Fix\n\n"
    );
    let (title, body) = ttm_runner::executor::pr_text(
        "AUR-1: 🤖 Generated with Claude Code",
        "Run r1.\n\nDone.\nCo-Authored-By: Claude <noreply@anthropic.com>",
    );
    assert_eq!(title, "agent changes");
    assert!(!body.contains("Claude"), "{body}");
}
