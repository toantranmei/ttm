//! Shared plumbing for the `ttm` CLI and the `ttm-mcp` server: where files live, the config file and its profiles,
//! which host to talk to, the discovery document of that host, and where the token is kept.
//!
//! Host resolution order: `--host` > `TTM_HOST` > profile > [`DEFAULT_HOST`] (set at build time with the
//! `TTM_DEFAULT_HOST` environment variable). Token resolution order: `TTM_TOKEN` > OS keyring > `credentials.toml`.

pub mod config;
pub mod context;
pub mod credentials;
pub mod discovery;
pub mod merge3;
pub mod paths;
pub mod profile;
pub mod project_file;
pub mod skill_zip;
pub mod worktree;

pub use config::Config;
pub use context::{Context, Overrides};
pub use credentials::{CredentialStore, StoredToken, TokenSource};
pub use discovery::Discovery;
pub use paths::Paths;
pub use profile::Profile;

/// The server used when nothing else is configured. Override at build time: `TTM_DEFAULT_HOST=https://… cargo build`.
pub const DEFAULT_HOST: &str = match option_env!("TTM_DEFAULT_HOST") {
    Some(host) => host,
    None => "https://toantranmei.com",
};

/// Version of this crate, shared by both binaries (one release train).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Name of the profile used when neither `--profile`, `TTM_PROFILE` nor `default_profile` picks one.
pub const DEFAULT_PROFILE: &str = "default";

/// Errors of this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid host `{0}`: expected an http:// or https:// URL")]
    InvalidHost(String),
    #[error("could not read {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("invalid TOML in {path}: {message}")]
    Toml {
        path: std::path::PathBuf,
        message: String,
    },
    #[error("discovery failed for {host}: {message}")]
    Discovery { host: String, message: String },
    #[error("no home directory found; set TTM_CONFIG_DIR and TTM_CACHE_DIR")]
    NoHome,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
