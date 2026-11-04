//! The local runner of ttm: takes jobs from the server, runs a coding agent (a *harness*, Claude Code first) in a git
//! worktree with the user's own subscription or API key, streams its activity back as run events, and pushes the
//! branch. Scheduling, model choice, context packing and prompts stay on the server; a job carries the final prompt.
//!
//! Connection: long-poll `GET /factory/runners/{id}/jobs?wait=25` plus a heartbeat every 20 s (outbound only, works
//! behind NAT). The WebSocket channel of the sync server can replace the long-poll later without changing the
//! executor.

pub mod capacity;
pub mod config;
pub mod connection;
pub mod executor;
pub mod git;
pub mod guard;
pub mod harness;
pub mod identity;
pub mod local_mcp;
pub mod merge;
pub mod say;
pub mod service;
pub mod shared;
pub mod skills;
pub mod worktree;

/// Features of this runner, announced to the server; `team_isolation` is added when a team Claude token is set
/// (see [`features`]).
pub const FEATURES: [&str; 4] = ["git_identity", "skills", "mcp_gateway", "mcp_local"];
/// Isolated configuration for teammates' jobs (needs `ttm runner config claude-token`).
pub const TEAM_ISOLATION: &str = "team_isolation";

/// The features this runner has.
pub fn features(team_isolation: bool) -> Vec<String> {
    let mut out: Vec<String> = FEATURES.iter().map(|f| f.to_string()).collect();
    if team_isolation {
        out.push(TEAM_ISOLATION.to_string());
    }
    out
}

pub use config::RunnerConfig;
pub use connection::Runner;
pub use executor::{Executor, JobOutcome};
pub use shared::SharedClient;
