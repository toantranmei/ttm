//! Harness adapters: how a coding agent is started and how its output becomes run events.

pub mod claude_code;

use async_trait::async_trait;
use std::path::Path;
use tokio::sync::{mpsc, watch};
use ttm_api::factory::{JobSpec, RunEvent};

/// How a harness run ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HarnessOutcome {
    /// `succeeded`, `failed`, `cancelled`, `paused`.
    pub status: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    /// Final message of the agent.
    pub summary: Option<String>,
    /// Usage limit reached: retry after this RFC 3339 time.
    pub paused_until: Option<String>,
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
        events: mpsc::UnboundedSender<RunEvent>,
        cancel: watch::Receiver<bool>,
    ) -> anyhow::Result<HarnessOutcome>;
}

/// Every harness this build knows.
pub fn all() -> Vec<Box<dyn Harness>> {
    vec![Box::new(claude_code::ClaudeCode::default())]
}

/// Names of the installed harnesses.
pub fn installed() -> Vec<String> {
    all()
        .into_iter()
        .filter(|h| h.available())
        .map(|h| h.name().to_string())
        .collect()
}

pub fn by_name(name: &str) -> Option<Box<dyn Harness>> {
    all().into_iter().find(|h| h.name() == name)
}
