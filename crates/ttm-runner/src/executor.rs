//! Runs one job: worktree → rules → MCP config → harness → commit/push/PR → finish.

use crate::SharedClient;
use crate::config::RunnerConfig;
use crate::harness::{self, HarnessOutcome};
use crate::local_mcp::{self, McpSection};
use crate::{git, guard, identity, skills, worktree};
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
    /// Where run lines (start, end, refusals) and echoed events are printed.
    pub ui: ttm_ui::Ui,
    /// Directory of `runner.toml`, re-read for `[mcp.local.*]` at each job (edits apply without a restart).
    pub config_dir: Option<PathBuf>,
    /// OAuth token of Claude Code for teammates' jobs (`ttm runner config claude-token`): such jobs run with an
    /// empty `CLAUDE_CONFIG_DIR`, so they never see this machine's settings, hooks, plugins or personal skills.
    pub team_claude_token: Option<String>,
}

/// One remote MCP of the job (through the gateway) or one granted local MCP, as written in the config; warnings for
/// what was left out.
pub fn mcp_config(
    job: &JobSpec,
    host: &str,
    ttm_mcp: &Path,
    local: &McpSection,
    getenv: &dyn Fn(&str) -> Option<String>,
) -> (serde_json::Value, Vec<String>) {
    let mut config = ttm_mcp_config(job, host, ttm_mcp);
    let mut warnings = Vec::new();
    let servers = config["mcpServers"].as_object_mut().expect("object");
    for remote in &job.mcp.remote {
        if remote.name == "ttm" || servers.contains_key(&remote.name) {
            warnings.push(format!("MCP `{}` skipped: the name is taken", remote.name));
            continue;
        }
        servers.insert(
            remote.name.clone(),
            json!({
                "type": "http",
                "url": remote.url,
                "headers": { "Authorization": format!("Bearer {}", job.mcp.token) }
            }),
        );
    }
    let handle = job.agent.as_ref().map(|a| a.handle.as_str());
    for name in &job.mcp.local {
        if name == "ttm" || servers.contains_key(name) {
            warnings.push(format!("local MCP `{name}` skipped: the name is taken"));
            continue;
        }
        let Some(mcp) = local_mcp::grant(name, handle, job.team_job, local) else {
            warnings.push(format!(
                "local MCP `{name}` skipped: not configured on this runner, or not allowed for this agent or job"
            ));
            continue;
        };
        match local_mcp::resolve_env(&mcp.env, getenv) {
            Ok(env) => {
                servers.insert(
                    name.clone(),
                    json!({ "command": mcp.command, "args": mcp.args, "env": env }),
                );
            }
            Err(var) => warnings.push(format!(
                "local MCP `{name}` skipped: the environment variable {var} is not set on this runner"
            )),
        }
    }
    (config, warnings)
}

/// MCP config handed to the harness: `ttm-mcp` with the job token and run context in its environment.
pub fn ttm_mcp_config(job: &JobSpec, host: &str, ttm_mcp: &Path) -> serde_json::Value {
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
    /// Features of this runner (with `team_isolation` when a team token is set).
    pub fn features(&self) -> Vec<String> {
        crate::features(self.team_claude_token.is_some())
    }

    fn local_mcp(&self) -> McpSection {
        match &self.config_dir {
            Some(dir) => RunnerConfig::load_mcp(dir),
            None => self.config.mcp.clone(),
        }
    }

    fn refuse(&self, job: &JobSpec, refused: guard::Refused) -> anyhow::Error {
        tracing::error!("refusing run {} ({}): {refused}", job.run_id, job.task_key);
        self.ui.eline(&crate::say::refused(
            &self.ui,
            &job.task_key,
            refused.code,
            &refused.message,
        ));
        anyhow::Error::new(refused)
    }

    pub async fn run(&self, job: JobSpec, cancel: watch::Receiver<bool>) -> JobOutcome {
        let (tx, rx) = mpsc::unbounded_channel();
        let echo = self.echo.then(|| self.ui.clone());
        let forwarder = tokio::spawn(forward(self.client.clone(), job.run_id.clone(), rx, echo));
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
        if !outcome.status.is_empty() && finish.error.as_deref().is_none_or(|e| !is_refusal(e)) {
            self.ui
                .eline(&crate::say::finished(&self.ui, &job.task_key, &outcome));
        }
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
        if let Err(refused) = guard::check_requires(job, &self.features()) {
            return Err(self.refuse(job, refused));
        }
        if let Err(refused) = guard::check_team_isolation(job, self.team_claude_token.is_some()) {
            return Err(self.refuse(job, refused));
        }
        // Only the owner's configured clone, and only when it is the job's repository; never clone a URL.
        let repo = guard::check(
            job,
            self.config.repo_for(&key, job.project_id.as_deref()),
            |dir| ttm_core::worktree::git(dir, &["remote", "get-url", "origin"]).ok(),
        )
        .inspect_err(|refused| {
            tracing::error!("refusing run {} ({}): {refused}", job.run_id, job.task_key);
            self.ui.eline(&crate::say::refused(
                &self.ui,
                &job.task_key,
                refused.code,
                &refused.message,
            ));
        })?;
        let base = job
            .repo
            .base_branch
            .clone()
            .unwrap_or_else(|| ttm_core::worktree::default_branch(&repo));
        // Never a path from the server outside the configured root.
        let path = guard::worktree_for(job, &self.worktree_root).inspect_err(|refused| {
            tracing::error!("refusing run {} ({}): {refused}", job.run_id, job.task_key);
            self.ui.eline(&crate::say::refused(
                &self.ui,
                &job.task_key,
                refused.code,
                &refused.message,
            ));
        })?;
        let _ = events.send(RunEvent::now(
            "log",
            format!("worktree {} on {} from {base}", path.display(), job.branch),
            None,
        ));
        worktree::add(&repo, &path, &job.branch, &base)?;
        self.ui.eline(&crate::say::started(&self.ui, job, &path));

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

        // Commit identity and the commit-msg hook (every job: no Claude/Anthropic line in any commit).
        let trailers = job.git.as_ref().map(|g| g.trailers.clone()).unwrap_or_default();
        let hooks = identity::prepare_hooks(&path, &self.cache_dir, &job.run_id, &trailers)?;
        let git_env = identity::env(job.git.as_ref(), &hooks.dir);

        // Reviewed skills, checked by SHA-256; removed when `_skills` drops (any end of the run).
        let http = reqwest::Client::builder()
            .user_agent(ttm_api::user_agent())
            .build()?;
        let _skills = match skills::install(&http, job, &path, &self.cache_dir, events).await {
            Ok(installed) => installed,
            Err(err @ skills::SkillError::HashMismatch { .. }) => {
                let _ = events.send(RunEvent::now("error", err.to_string(), None));
                return Err(self.refuse(
                    job,
                    guard::Refused {
                        code: skills::SKILL_HASH_MISMATCH,
                        message: err.to_string(),
                    },
                ));
            }
            Err(err) => return Err(err.into()),
        };

        let mcp_file = self.cache_dir.join(format!("mcp-{}.json", job.run_id));
        let (mcp, warnings) = mcp_config(job, &self.host, &self.ttm_mcp_exe, &self.local_mcp(), &|v| {
            std::env::var(v).ok()
        });
        for w in warnings {
            let _ = events.send(RunEvent::now("log", w, None));
        }
        ttm_core::paths::write_atomic(&mcp_file, &serde_json::to_vec_pretty(&mcp)?, Some(0o600))?;
        let settings_file = self.cache_dir.join(format!("settings-{}.json", job.run_id));
        ttm_core::paths::write_atomic(
            &settings_file,
            &serde_json::to_vec_pretty(&identity::claude_settings())?,
            Some(0o600),
        )?;
        let mut env = git_env.clone();
        // Lets `ttm hook pre-bash` know it runs inside a job (stricter rules).
        env.push(("TTM_RUN_ID".into(), job.run_id.clone()));
        let mut team_dir = None;
        if job.team_job
            && let Some(token) = &self.team_claude_token
        {
            let dir = self.cache_dir.join("claude-team").join(&job.run_id);
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)?;
            env.push(("CLAUDE_CONFIG_DIR".into(), dir.display().to_string()));
            env.push(("CLAUDE_CODE_OAUTH_TOKEN".into(), token.clone()));
            team_dir = Some(dir);
        }
        let ctx = harness::RunContext {
            env,
            settings: Some(settings_file.clone()),
        };

        let harness = harness::by_name(&job.harness)
            .ok_or_else(|| anyhow::anyhow!("unknown harness {}", job.harness))?;
        let timeout = Duration::from_secs(60 * job.limits.timeout_min.unwrap_or(DEFAULT_TIMEOUT_MIN));
        let run = tokio::time::timeout(
            timeout,
            harness.run(job, &path, &mcp_file, &ctx, events.clone(), cancel),
        )
        .await;
        let _ = std::fs::remove_file(&mcp_file);
        let _ = std::fs::remove_file(&settings_file);
        if let Some(dir) = team_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
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
                git::commit_all(&path, &leftover_commit_message(job), &git_env)?;
            }
            finish.commit = git::head(&path).ok();
            if git::commits_ahead(&path, &base) > 0 && git::has_remote(&path) {
                // Whatever made the commits, none may carry an attribution line when pushed.
                let pushed = identity::sanitize_history(&path, &base, &git_env)
                    .inspect(|n| {
                        if *n > 0 {
                            let _ = events.send(RunEvent::now(
                                "log",
                                format!("removed attribution lines from {n} commit message(s)"),
                                None,
                            ));
                        }
                    })
                    .and_then(|_| git::push(&path, &job.branch, &git_env).map_err(anyhow::Error::from));
                finish.commit = git::head(&path).ok();
                match pushed {
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
                        let (title, body) = pr_text(&title, &body);
                        finish.pr_url = git::open_pull_request(&path, &job.branch, &base, &title, &body);
                    }
                    Err(err) => {
                        self.ui.eprint(
                            &ttm_ui::Callout::warning(format!("{}: push failed", job.task_key))
                                .detail(err.to_string()),
                        );
                        let _ = events.send(RunEvent::now("log", format!("push failed: {err}"), None));
                    }
                }
            }
        }
        let mut ledger = worktree::Ledger::load(&self.cache_dir);
        ledger.record(&repo, &path, &job.run_id);
        let _ = ledger.save(&self.cache_dir);
        drop(hooks);

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

/// Pull request title and body without attribution lines (the agent's summary may carry one).
pub fn pr_text(title: &str, body: &str) -> (String, String) {
    let title = if identity::is_attribution_line(title) {
        "agent changes".to_string()
    } else {
        title.to_string()
    };
    (title, identity::strip_text(body))
}

/// Whether a finish `error` is a refusal code (already announced by the refusal line).
fn is_refusal(error: &str) -> bool {
    matches!(
        error,
        guard::REPO_NOT_CONFIGURED
            | guard::INVALID_BRANCH
            | guard::INVALID_WORKTREE
            | guard::UNSUPPORTED_FEATURE
            | skills::SKILL_HASH_MISMATCH
    )
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
    echo: Option<ttm_ui::Ui>,
) {
    let mut buf: Vec<RunEvent> = Vec::new();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            event = rx.recv() => match event {
                Some(e) => {
                    if let Some(ui) = &echo {
                        ui.line(&crate::say::event_line(ui, &e));
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
        let (c, _) = mcp_config(
            &job(),
            "https://toantranmei.com",
            Path::new("/bin/ttm-mcp"),
            &McpSection::default(),
            &|_| None,
        );
        let env = &c["mcpServers"]["ttm"]["env"];
        assert_eq!(env["TTM_TOKEN"], "ttm_job_abc");
        assert_eq!(env["TTM_RUN_ID"], "r1");
        assert_eq!(env["TTM_PROJECT"], "p1");
        assert_eq!(c["mcpServers"]["ttm"]["args"][1], "factory,notee");
    }

    fn local() -> McpSection {
        toml::from_str::<RunnerConfig>(
            r#"
            [mcp.local.github-local]
            command = "npx"
            env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }
            allow = ["dev"]
            [mcp.local.notes]
            command = "notes-mcp"
            allow = ["dev"]
            team_jobs = true
            [mcp.local.secretless]
            command = "x"
            env = { K = "env:MISSING" }
            allow = ["dev"]
            "#,
        )
        .unwrap()
        .mcp
    }

    fn v2_job(team_job: bool) -> JobSpec {
        serde_json::from_value(json!({
            "run_id": "r1", "task_key": "AUR-12", "project_id": "p1", "branch": "aur-12", "prompt": "x",
            "agent": {"id": "a1", "handle": "dev", "name": "DEV"}, "team_job": team_job,
            "mcp": {"token": "ttm_job_abc", "toolsets": [],
                    "remote": [{"name": "linear", "url": "https://h/api/v1/mcp/gw/linear"}, {"name": "ttm", "url": "https://evil"}],
                    "local": ["github-local", "notes", "secretless", "linear", "unknown"]}
        }))
        .unwrap()
    }

    #[test]
    fn remote_and_local_mcp_follow_the_rules() {
        let env = |v: &str| (v == "GITHUB_TOKEN").then(|| "ghp_x".to_string());
        let (c, warnings) = mcp_config(
            &v2_job(false),
            "https://h",
            Path::new("/bin/ttm-mcp"),
            &local(),
            &env,
        );
        let s = &c["mcpServers"];
        assert_eq!(s["linear"]["type"], "http");
        assert_eq!(s["linear"]["headers"]["Authorization"], "Bearer ttm_job_abc");
        assert_eq!(
            s["ttm"]["command"], "/bin/ttm-mcp",
            "the job's ttm server is never replaced"
        );
        assert_eq!(s["github-local"]["env"]["GITHUB_TOKEN"], "ghp_x");
        assert!(s["notes"].is_object());
        assert!(s["secretless"].is_null(), "missing env var drops the server");
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        // A teammate's job only gets local servers with team_jobs.
        let (c, _) = mcp_config(
            &v2_job(true),
            "https://h",
            Path::new("/bin/ttm-mcp"),
            &local(),
            &env,
        );
        assert!(c["mcpServers"]["github-local"].is_null());
        assert!(c["mcpServers"]["notes"].is_object());
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
