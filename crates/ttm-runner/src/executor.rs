//! Runs one job: worktree → rules → MCP config → harness → commit/push/PR → finish.

use crate::SharedClient;
use crate::config::RunnerConfig;
use crate::harness::{self, HarnessOutcome};
use crate::{git, guard, worktree};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use ttm_api::factory::{JobSpec, RunEvent, RunFinish};

/// Default job timeout when the spec has none.
pub const DEFAULT_TIMEOUT_MIN: u64 = 120;
const BATCH: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobOutcome {
    pub run_id: String,
    pub status: String,
    pub branch: String,
    pub worktree: Option<PathBuf>,
    pub pr_url: Option<String>,
    pub paused_until: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Executor {
    /// Client authenticated as the runner; swapped when the runner token is renewed.
    pub client: SharedClient,
    /// Server URL given to `ttm-mcp`.
    pub host: String,
    pub config: RunnerConfig,
    /// Root of worktrees.
    pub worktree_root: PathBuf,
    /// Scratch: MCP config files, worktree ledger.
    pub cache_dir: PathBuf,
    /// `ttm` (for `ttm rules apply`) and `ttm-mcp` binaries.
    pub ttm_exe: PathBuf,
    pub ttm_mcp_exe: PathBuf,
    /// Also print events to stdout (`ttm agent run`).
    pub echo: bool,
}

/// MCP config handed to the harness: `ttm-mcp` with the job token and run context in its environment.
pub fn mcp_config(job: &JobSpec, host: &str, ttm_mcp: &Path) -> serde_json::Value {
    let toolsets = if job.mcp.toolsets.is_empty() {
        "factory,notee".to_string()
    } else {
        job.mcp.toolsets.join(",")
    };
    json!({
        "mcpServers": {
            "ttm": {
                "command": ttm_mcp.display().to_string(),
                "args": ["--toolsets", toolsets],
                "env": {
                    "TTM_TOKEN": job.mcp.token,
                    "TTM_HOST": host,
                    "TTM_NO_KEYRING": "1",
                    "TTM_RUN_ID": job.run_id,
                    "TTM_TASK": job.task_key,
                    "TTM_PROJECT": job.project_id.clone().unwrap_or_else(|| job.project_key())
                }
            }
        }
    })
}

/// Commit message for what the agent left uncommitted.
pub fn leftover_commit_message(job: &JobSpec) -> String {
    format!(
        "chore({}): changes from agent run {}",
        job.task_key.to_lowercase(),
        job.run_id
    )
}

impl Executor {
    pub async fn run(&self, job: JobSpec, cancel: watch::Receiver<bool>) -> JobOutcome {
        let (tx, rx) = mpsc::unbounded_channel();
        let forwarder = tokio::spawn(forward(self.client.clone(), job.run_id.clone(), rx, self.echo));
        let result = self.execute(&job, &tx, cancel).await;
        let (outcome, finish) = match result {
            Ok(done) => done,
            Err(err) => {
                let _ = tx.send(RunEvent::now("log", format!("runner error: {err:#}"), None));
                let outcome = JobOutcome {
                    run_id: job.run_id.clone(),
                    status: "failed".into(),
                    branch: job.branch.clone(),
                    worktree: None,
                    pr_url: None,
                    paused_until: None,
                };
                let finish = failed_finish(&job, &err);
                (outcome, finish)
            }
        };
        drop(tx);
        let _ = forwarder.await;
        if let Err(err) = self.client.get().finish_run(&job.run_id, &finish).await {
            tracing::warn!("could not report the end of run {}: {err}", job.run_id);
        }
        outcome
    }

    async fn execute(
        &self,
        job: &JobSpec,
        events: &mpsc::UnboundedSender<RunEvent>,
        cancel: watch::Receiver<bool>,
    ) -> anyhow::Result<(JobOutcome, RunFinish)> {
        let key = job.project_key();
        // Only the owner's configured clone, and only when it is the job's repository; never clone a URL.
        let repo = guard::check(
            job,
            self.config.repo_for(&key, job.project_id.as_deref()),
            |dir| ttm_core::worktree::git(dir, &["remote", "get-url", "origin"]).ok(),
        )
        .inspect_err(|refused| {
            tracing::error!("refusing run {} ({}): {refused}", job.run_id, job.task_key);
            eprintln!("ttm runner: refused {}: {}", job.task_key, refused.message);
        })?;
        let base = job
            .repo
            .base_branch
            .clone()
            .unwrap_or_else(|| ttm_core::worktree::default_branch(&repo));
        let path = match &job.worktree_hint {
            Some(hint) if Path::new(hint).is_absolute() => PathBuf::from(hint),
            _ => worktree::worktree_path(&self.worktree_root, &key, &job.branch),
        };
        let _ = events.send(RunEvent::now(
            "log",
            format!("worktree {} on {} from {base}", path.display(), job.branch),
            None,
        ));
        worktree::add(&repo, &path, &job.branch, &base)?;

        // Project rules (AGENTS.md / CLAUDE.md block, hooks) inside the worktree; best effort.
        let rules = tokio::process::Command::new(&self.ttm_exe)
            .args(["rules", "apply"])
            .current_dir(&path)
            .output()
            .await;
        if !rules.as_ref().is_ok_and(|o| o.status.success()) {
            let _ = events.send(RunEvent::now(
                "log",
                "ttm rules apply skipped (project not linked in this repository?)",
                None,
            ));
        }

        let mcp_file = self.cache_dir.join(format!("mcp-{}.json", job.run_id));
        let config = serde_json::to_vec_pretty(&mcp_config(job, &self.host, &self.ttm_mcp_exe))?;
        ttm_core::paths::write_atomic(&mcp_file, &config, Some(0o600))?;

        let harness = harness::by_name(&job.harness)
            .ok_or_else(|| anyhow::anyhow!("unknown harness {}", job.harness))?;
        let timeout = Duration::from_secs(60 * job.limits.timeout_min.unwrap_or(DEFAULT_TIMEOUT_MIN));
        let run = tokio::time::timeout(
            timeout,
            harness.run(job, &path, &mcp_file, events.clone(), cancel),
        )
        .await;
        let _ = std::fs::remove_file(&mcp_file);
        let outcome: HarnessOutcome = match run {
            Ok(result) => result?,
            Err(_) => HarnessOutcome {
                status: "failed".into(),
                summary: Some("timed out".into()),
                ..HarnessOutcome::default()
            },
        };

        let mut finish = RunFinish {
            status: outcome.status.clone(),
            branch: Some(job.branch.clone()),
            tokens_in: outcome.tokens_in,
            tokens_out: outcome.tokens_out,
            summary: outcome.summary.clone(),
            paused_until: outcome.paused_until.clone(),
            tests_pass: outcome.tests_pass,
            error: (outcome.status == "failed")
                .then(|| outcome.summary.clone().unwrap_or_else(|| "agent failed".into())),
            ..RunFinish::default()
        };
        if let Some(passed) = outcome.tests_pass {
            let artifact = serde_json::json!({
                "kind": "test_result",
                "title": outcome.tests_command.clone().unwrap_or_else(|| "tests".into()),
                "passed": passed
            });
            if let Err(err) = self.client.get().attach_artifact(&job.run_id, &artifact).await {
                tracing::debug!("could not attach the test result: {err}");
            }
        }
        if outcome.status == "succeeded" {
            if git::is_dirty(&path).unwrap_or(false) {
                git::commit_all(&path, &leftover_commit_message(job))?;
            }
            finish.commit = git::head(&path).ok();
            if git::commits_ahead(&path, &base) > 0 && git::has_remote(&path) {
                match git::push(&path, &job.branch) {
                    Ok(()) => {
                        let title = format!(
                            "{}: {}",
                            job.task_key,
                            outcome
                                .summary
                                .as_deref()
                                .and_then(|s| s.lines().next())
                                .unwrap_or("agent changes")
                        );
                        let body = format!(
                            "Run {} by ttm runner.\n\n{}",
                            job.run_id,
                            outcome.summary.as_deref().unwrap_or("")
                        );
                        finish.pr_url = git::open_pull_request(&path, &job.branch, &base, &title, &body);
                    }
                    Err(err) => {
                        let _ = events.send(RunEvent::now("log", format!("push failed: {err}"), None));
                    }
                }
            }
        }
        let mut ledger = worktree::Ledger::load(&self.cache_dir);
        ledger.record(&repo, &path, &job.run_id);
        let _ = ledger.save(&self.cache_dir);

        let outcome = JobOutcome {
            run_id: job.run_id.clone(),
            status: finish.status.clone(),
            branch: job.branch.clone(),
            worktree: Some(path),
            pr_url: finish.pr_url.clone(),
            paused_until: finish.paused_until.clone(),
        };
        Ok((outcome, finish))
    }
}

/// The finish report of a job that could not run. A refusal reports its code (`repo_not_configured`,
/// `invalid_branch`) as `error`; an invalid branch name is never sent (the server answers 400).
pub fn failed_finish(job: &JobSpec, err: &anyhow::Error) -> RunFinish {
    let (error, summary) = match err.downcast_ref::<guard::Refused>() {
        Some(refused) => (refused.code.to_string(), refused.message.clone()),
        None => (format!("{err:#}"), format!("{err:#}")),
    };
    RunFinish {
        status: "failed".into(),
        branch: guard::valid_branch(&job.branch).then(|| job.branch.clone()),
        summary: Some(summary),
        error: Some(error),
        ..RunFinish::default()
    }
}

/// Sends events in batches (every second or every 25 events).
async fn forward(
    client: SharedClient,
    run_id: String,
    mut rx: mpsc::UnboundedReceiver<RunEvent>,
    echo: bool,
) {
    let mut buf: Vec<RunEvent> = Vec::new();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            event = rx.recv() => match event {
                Some(e) => {
                    if echo {
                        println!("[{}] {}", e.kind, e.message);
                    }
                    buf.push(e);
                    if buf.len() >= BATCH {
                        flush(&client, &run_id, &mut buf).await;
                    }
                }
                None => break,
            },
            _ = tick.tick() => flush(&client, &run_id, &mut buf).await,
        }
    }
    flush(&client, &run_id, &mut buf).await;
}

async fn flush(client: &SharedClient, run_id: &str, buf: &mut Vec<RunEvent>) {
    if buf.is_empty() {
        return;
    }
    if let Err(err) = client.get().post_run_events(run_id, buf).await {
        tracing::debug!("could not send {} event(s): {err}", buf.len());
    }
    buf.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> JobSpec {
        serde_json::from_value(json!({
            "run_id": "r1", "task_key": "AUR-12", "project_id": "p1", "branch": "aur-12", "prompt": "x",
            "mcp": {"token": "ttm_job_abc", "toolsets": []}
        }))
        .unwrap()
    }

    #[test]
    fn mcp_config_carries_token_and_context() {
        let c = mcp_config(&job(), "https://toantranmei.com", Path::new("/bin/ttm-mcp"));
        let env = &c["mcpServers"]["ttm"]["env"];
        assert_eq!(env["TTM_TOKEN"], "ttm_job_abc");
        assert_eq!(env["TTM_RUN_ID"], "r1");
        assert_eq!(env["TTM_PROJECT"], "p1");
        assert_eq!(c["mcpServers"]["ttm"]["args"][1], "factory,notee");
    }

    #[test]
    fn refusals_report_their_code() {
        let mut j = job();
        j.branch = "bad branch".into();
        let err = anyhow::Error::new(guard::Refused {
            code: guard::REPO_NOT_CONFIGURED,
            message: "no clone".into(),
        });
        let f = failed_finish(&j, &err);
        assert_eq!(
            (f.error.as_deref(), f.summary.as_deref(), f.branch),
            (Some("repo_not_configured"), Some("no clone"), None)
        );
        let other = failed_finish(&job(), &anyhow::anyhow!("boom"));
        assert_eq!(
            (other.error.as_deref(), other.branch.as_deref()),
            (Some("boom"), Some("aur-12"))
        );
    }

    #[test]
    fn leftover_message_is_conventional() {
        assert_eq!(
            leftover_commit_message(&job()),
            "chore(aur-12): changes from agent run r1"
        );
    }
}
