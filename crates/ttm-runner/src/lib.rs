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
pub mod harness;
pub mod service;
pub mod worktree;

pub use config::RunnerConfig;
pub use connection::Runner;
pub use executor::{Executor, JobOutcome};
