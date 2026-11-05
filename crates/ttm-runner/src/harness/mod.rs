//! Harness adapters: how a coding agent is started and how its output becomes run events.

pub mod claude_code;
/// Scripted fake harness for the end-to-end simulation; only with the `sim-harness` feature (never in releases).
#[cfg(feature = "sim-harness")]
pub mod sim;

use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use ttm_api::factory::{JobSpec, RunEvent};

/// What the runner adds around a job: git identity and hooks environment, Claude Code settings, team isolation.
#[derive(Debug, Clone, Default)]
pub struct RunContext {
    /// Extra environment of the agent process (`GIT_*`, `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_OAUTH_TOKEN`).
    pub env: Vec<(String, String)>,
    /// `--settings` file (no Claude co-author or attribution lines).
    pub settings: Option<std::path::PathBuf>,
    /// Tokens used so far, kept up to date by the harness while it runs: read when the run is dropped (job timeout),
    /// so a timed out run still reports what it spent.
    pub usage: Arc<UsageMeter>,
}

/// Running token totals of one harness run.
#[derive(Debug, Default)]
pub struct UsageMeter {
    tokens_in: AtomicU64,
    tokens_out: AtomicU64,
    cache_creation: AtomicU64,
    cache_read: AtomicU64,
}

impl UsageMeter {
    /// Sets the totals (the harness's running sum).
    pub fn set(&self, tokens_in: u64, tokens_out: u64) {
        self.tokens_in.store(tokens_in, Ordering::Relaxed);
        self.tokens_out.store(tokens_out, Ordering::Relaxed);
    }

    /// Sets the totals of an outcome, prompt cache counts included.
    pub fn set_outcome(&self, outcome: &HarnessOutcome) {
        self.set(outcome.tokens_in, outcome.tokens_out);
        self.cache_creation
            .store(outcome.cache_creation, Ordering::Relaxed);
        self.cache_read.store(outcome.cache_read, Ordering::Relaxed);
    }

    /// `(cache_creation, cache_read)` so far (both part of `tokens_in`).
    pub fn cache(&self) -> (u64, u64) {
        (
            self.cache_creation.load(Ordering::Relaxed),
            self.cache_read.load(Ordering::Relaxed),
        )
    }

    /// `(tokens_in, tokens_out)` so far.
    pub fn get(&self) -> (u64, u64) {
        (
            self.tokens_in.load(Ordering::Relaxed),
            self.tokens_out.load(Ordering::Relaxed),
        )
    }
}

/// How long the agent's process group gets between SIGTERM and SIGKILL when a run is stopped.
pub const STOP_GRACE: Duration = Duration::from_secs(3);

/// The agent process and everything it started (dev servers, test runs, MCP servers): the agent is spawned as the
/// leader of its own process group, and stopping the run signals the whole group, not only the agent. Dropping a
/// guard that was not stopped or released (the run future dropped on a job timeout) stops the group too, in the
/// background.
#[derive(Debug)]
pub struct ProcessGroup {
    pgid: Option<u32>,
}

impl ProcessGroup {
    /// Makes `command` start a new process group (Unix; elsewhere only the agent itself is stopped).
    pub fn prepare(command: &mut tokio::process::Command) {
        #[cfg(unix)]
        command.process_group(0);
        #[cfg(not(unix))]
        let _ = command;
    }

    /// Guard of the group led by `child` (spawned after [`ProcessGroup::prepare`]).
    pub fn of(child: &tokio::process::Child) -> Self {
        Self { pgid: child.id() }
    }

    /// SIGTERM to the group, up to `grace` for the agent to exit, then SIGKILL to whatever is left of the group.
    pub async fn stop(&mut self, child: &mut tokio::process::Child, grace: Duration) {
        let pgid = self.pgid.take();
        signal_group(pgid, false);
        if tokio::time::timeout(grace, child.wait()).await.is_err() {
            let _ = child.start_kill();
        }
        signal_group(pgid, true);
        let _ = child.wait().await;
    }

    /// The agent ended by itself: leave the group alone.
    pub fn release(&mut self) {
        self.pgid = None;
    }
}

impl Drop for ProcessGroup {
    fn drop(&mut self) {
        if let Some(pgid) = self.pgid.take() {
            signal_group(Some(pgid), false);
            std::thread::spawn(move || {
                std::thread::sleep(STOP_GRACE);
                signal_group(Some(pgid), true);
            });
        }
    }
}

#[cfg(unix)]
fn signal_group(pgid: Option<u32>, kill: bool) {
    use rustix::process::{Pid, Signal, kill_process_group};
    let Some(pid) = pgid.and_then(|p| Pid::from_raw(p as i32)) else {
        return;
    };
    let _ = kill_process_group(pid, if kill { Signal::KILL } else { Signal::TERM });
}

#[cfg(not(unix))]
fn signal_group(_pgid: Option<u32>, _kill: bool) {}

/// How a harness run ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HarnessOutcome {
    /// `succeeded`, `failed`, `cancelled`, `paused`.
    pub status: String,
    /// Input tokens, prompt cache writes and reads included.
    pub tokens_in: u64,
    pub tokens_out: u64,
    /// Of `tokens_in`: prompt cache writes and reads (Claude Code reports them; other harnesses leave 0).
    pub cache_creation: u64,
    pub cache_read: u64,
    /// Final message of the agent.
    pub summary: Option<String>,
    /// Usage limit reached: retry after this RFC 3339 time.
    pub paused_until: Option<String>,
    /// The agent ran a test command: whether the last run passed.
    pub tests_pass: Option<bool>,
    /// That test command.
    pub tests_command: Option<String>,
}

impl HarnessOutcome {
    /// Tokens counted against a budget: [`ttm_api::factory::weighted_tokens`] (cache by price).
    pub fn weighted_tokens(&self) -> u64 {
        ttm_api::factory::weighted_tokens(
            self.tokens_in,
            self.tokens_out,
            self.cache_creation,
            self.cache_read,
        )
    }
}

#[async_trait]
pub trait Harness: Send + Sync {
    /// Name used in job specs and `runner register` (`claude-code`).
    fn name(&self) -> &'static str;
    /// Whether the agent CLI is installed.
    fn available(&self) -> bool;
    /// Runs the job in `worktree` with the MCP config file `mcp_config`; sends events; stops when `cancel` turns true.
    async fn run(
        &self,
        job: &JobSpec,
        worktree: &Path,
        mcp_config: &Path,
        ctx: &RunContext,
        events: mpsc::UnboundedSender<RunEvent>,
        cancel: watch::Receiver<bool>,
    ) -> anyhow::Result<HarnessOutcome>;
}

/// Every harness this build knows.
pub fn all() -> Vec<Box<dyn Harness>> {
    #[cfg_attr(not(feature = "sim-harness"), allow(unused_mut))]
    let mut out: Vec<Box<dyn Harness>> = vec![Box::new(claude_code::ClaudeCode::default())];
    #[cfg(feature = "sim-harness")]
    out.push(Box::new(sim::SimHarness::default()));
    out
}

/// Names of the installed harnesses. A simulation build whose `ttm-sim` is set up announces only `ttm-sim`, so the
/// server never routes a simulated job to a real agent on the same machine.
pub fn installed() -> Vec<String> {
    let names: Vec<String> = all()
        .into_iter()
        .filter(|h| h.available())
        .map(|h| h.name().to_string())
        .collect();
    #[cfg(feature = "sim-harness")]
    if names.iter().any(|n| n == sim::NAME) {
        return vec![sim::NAME.to_string()];
    }
    names
}

pub fn by_name(name: &str) -> Option<Box<dyn Harness>> {
    all().into_iter().find(|h| h.name() == name)
}
