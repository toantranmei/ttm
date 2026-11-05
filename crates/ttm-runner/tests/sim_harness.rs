//! The scripted fake harness `ttm-sim` in a real (temporary) git repository.

#![cfg(all(unix, feature = "sim-harness"))]

use serde_json::json;
use std::path::Path;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use ttm_api::factory::{JobGit, JobSpec, RunEvent};
use ttm_core::worktree::git;
use ttm_runner::harness::sim::{NAME, SimHarness};
use ttm_runner::harness::{self, Harness, HarnessOutcome, RunContext};
use ttm_runner::identity;

struct Setup {
    repo: tempfile::TempDir,
    scripts: tempfile::TempDir,
    _cache: tempfile::TempDir,
    _hooks: identity::Hooks,
    ctx: RunContext,
}

fn setup() -> Setup {
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "-q", "-b", "main"]).unwrap();
    git(repo.path(), &["config", "user.name", "Machine Owner"]).unwrap();
    git(repo.path(), &["config", "user.email", "owner@example.com"]).unwrap();
    std::fs::write(repo.path().join("README.md"), "todo\n").unwrap();
    git(repo.path(), &["add", "-A"]).unwrap();
    git(repo.path(), &["commit", "-q", "-m", "init"]).unwrap();
    let cache = tempfile::tempdir().unwrap();
    let hooks = identity::prepare_hooks(repo.path(), cache.path(), "r1", &[]).unwrap();
    let agent = JobGit {
        author_name: "DEV · Code".into(),
        author_email: "dev+todo@agents.toantranmei.com".into(),
        ..JobGit::default()
    };
    let ctx = RunContext {
        env: identity::env(Some(&agent), &hooks.dir),
        settings: None,
        ..RunContext::default()
    };
    Setup {
        repo,
        scripts: tempfile::tempdir().unwrap(),
        _cache: cache,
        _hooks: hooks,
        ctx,
    }
}

fn job(budget: Option<u64>) -> JobSpec {
    serde_json::from_value(json!({
        "run_id": "r1", "task_key": "TODO-3", "branch": "ttm/todo-3", "harness": NAME,
        "prompt": "Do task TODO-3.\n\n---\n\n# TODO-3: Clear completed\n\nProject Todo (TODO) · state `doing` · role DEV · priority 2\n",
        "mcp": {"token": "ttm_job_x"}, "limits": {"budget_tokens": budget}
    }))
    .unwrap()
}

impl Setup {
    fn script(&self, name: &str, script: serde_json::Value) {
        std::fs::write(self.scripts.path().join(name), script.to_string()).unwrap();
    }

    fn harness(&self) -> SimHarness {
        SimHarness {
            script_dir: Some(self.scripts.path().to_path_buf()),
        }
    }

    async fn run_with(
        &self,
        job: &JobSpec,
        mcp: &Path,
        cancel: watch::Receiver<bool>,
    ) -> (anyhow::Result<HarnessOutcome>, Vec<RunEvent>) {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let out = self
            .harness()
            .run(job, self.repo.path(), mcp, &self.ctx, tx, cancel)
            .await;
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }
        (out, events)
    }

    async fn run(&self, job: &JobSpec) -> (anyhow::Result<HarnessOutcome>, Vec<RunEvent>) {
        let (_keep, cancel) = watch::channel(false);
        self.run_with(job, Path::new("/nonexistent/mcp.json"), cancel)
            .await
    }
}

#[test]
fn listed_only_with_the_feature_and_a_script_dir() {
    assert!(harness::all().iter().any(|h| h.name() == NAME));
    let missing = SimHarness {
        script_dir: Some("/nonexistent/ttm-sim".into()),
    };
    assert!(!missing.available());
    let s = setup();
    assert!(s.harness().available());
}

#[tokio::test]
async fn succeeded_writes_and_commits_with_the_agent_identity() {
    let s = setup();
    s.script(
        "role-DEV.json",
        json!({ "steps": [
            { "event": "thought", "text": "Reading {{task.key}}" },
            { "write": { "path": "src/clear.ts", "content": "export const clear = 1\n" } },
            { "run": "test -f src/clear.ts", "report_tests": true },
            { "usage": { "input": 1200, "output": 240 } },
            { "commit": "feat: {{task.title}}\n\nCo-Authored-By: Claude <noreply@anthropic.com>" }
        ], "outcome": "succeeded" }),
    );
    let (out, events) = s.run(&job(None)).await;
    let out = out.unwrap();
    assert_eq!(out.status, "succeeded");
    assert_eq!((out.tokens_in, out.tokens_out), (1200, 240));
    assert_eq!(out.tests_pass, Some(true));
    assert_eq!(out.tests_command.as_deref(), Some("test -f src/clear.ts"));
    let kinds: Vec<_> = events.iter().map(|e| e.kind.as_str()).collect();
    for k in ["thought", "action", "usage", "response"] {
        assert!(kinds.contains(&k), "{kinds:?}");
    }
    assert_eq!(events[1].message, "Reading TODO-3");
    let log = git(s.repo.path(), &["log", "-1", "--format=%an <%ae>|%B"]).unwrap();
    assert!(
        log.starts_with("DEV · Code <dev+todo@agents.toantranmei.com>|feat: Clear completed"),
        "{log}"
    );
    assert!(!log.to_lowercase().contains("anthropic"), "{log}");
}

#[tokio::test]
async fn tests_fail_reports_failed_tests() {
    let s = setup();
    std::fs::write(
        s.scripts.path().join("break.patch"),
        "diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1 +1 @@\n-todo\n+broken\n",
    )
    .unwrap();
    s.script(
        "TODO-3.json",
        json!({ "steps": [
            { "patch": "break.patch" },
            { "run": "grep -q todo README.md", "report_tests": true }
        ], "outcome": "tests_fail" }),
    );
    let (out, events) = s.run(&job(None)).await;
    let out = out.unwrap();
    assert_eq!(out.status, "succeeded");
    assert_eq!(out.tests_pass, Some(false));
    assert!(events.iter().any(|e| e.message.starts_with("tests failed")));
}

#[tokio::test]
async fn hang_stops_on_cancel() {
    let s = setup();
    s.script("default.json", json!({ "steps": [], "outcome": "hang" }));
    let (tx, cancel) = watch::channel(false);
    let stop = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        tx.send(true).unwrap();
        tx
    });
    let job = job(None);
    let (out, _) = tokio::time::timeout(
        Duration::from_secs(10),
        s.run_with(&job, Path::new("/nonexistent"), cancel),
    )
    .await
    .expect("hang ends on cancel");
    assert_eq!(out.unwrap().status, "cancelled");
    drop(stop.await);
}

#[tokio::test]
async fn budget_reports_usage_until_past_the_budget() {
    let s = setup();
    s.script(
        "default.json",
        json!({ "outcome": "budget", "budget_interval_ms": 1, "budget_grace_ms": 20 }),
    );
    let (out, events) = s.run(&job(Some(1000))).await;
    let out = out.unwrap();
    assert_eq!(out.status, "failed");
    assert_eq!(out.summary.as_deref(), Some("budget exhausted"));
    assert!(out.tokens_in + out.tokens_out > 1000);
    assert_eq!(events.iter().filter(|e| e.kind == "usage").count(), 11);
}

#[tokio::test]
async fn crash_and_pause() {
    let s = setup();
    s.script(
        "default.json",
        json!({ "steps": [{ "sleep_ms": 1 }, { "sleep_ms": 1 }], "outcome": "crash" }),
    );
    let (out, _) = s.run(&job(None)).await;
    assert!(out.unwrap_err().to_string().contains("crashed after 1 step"));
    s.script("default.json", json!({ "outcome": "pause" }));
    let (out, _) = s.run(&job(None)).await;
    let out = out.unwrap();
    assert_eq!(out.status, "paused");
    assert!(out.paused_until.is_some());
}

#[tokio::test]
async fn mcp_calls_a_stdio_server_of_the_job_config() {
    let s = setup();
    let server = s.scripts.path().join("server.sh");
    std::fs::write(
        &server,
        "#!/bin/sh\nread l\necho '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-06-18\",\"capabilities\":{}}}'\nread l\nread l\necho \"{\\\"jsonrpc\\\":\\\"2.0\\\",\\\"id\\\":2,\\\"result\\\":{\\\"content\\\":[{\\\"type\\\":\\\"text\\\",\\\"text\\\":\\\"$SIM_WORD\\\"}]}}\"\n",
    )
    .unwrap();
    let mcp = s.scripts.path().join("mcp.json");
    std::fs::write(
        &mcp,
        json!({ "mcpServers": { "ttm": { "command": "sh", "args": [server.display().to_string()],
            "env": { "SIM_WORD": "commented" } } } })
        .to_string(),
    )
    .unwrap();
    s.script(
        "default.json",
        json!({ "steps": [{ "mcp": { "tool": "factory-task-comment", "args": { "task": "{{task.key}}" } } }] }),
    );
    let (_keep, cancel) = watch::channel(false);
    let (out, events) = s.run_with(&job(None), &mcp, cancel).await;
    assert_eq!(out.unwrap().status, "succeeded");
    let action = events
        .iter()
        .find(|e| e.kind == "action" && e.message == "mcp__ttm__factory-task-comment")
        .expect("an action event");
    let data = action.data.as_ref().unwrap();
    assert_eq!(data["input"]["task"], "TODO-3");
    assert_eq!(data["result"]["content"][0]["text"], "commented");
}
