//! Typed HTTP client for the `/api/v1` of toantranmei.com (or any compatible host).
//!
//! Types are written by hand after the server's `openapi/v1.json`; the contract test of the release pipeline checks
//! that they still match. Behaviour shared by every call lives in [`Client`]: `User-Agent: ttm/<ver> (<os>; <arch>)`,
//! 30 s timeout, up to 3 retries on 429/5xx honouring `Retry-After`, a once-a-day warning when the server marks an
//! endpoint with `Deprecation`, and [`ApiError::UpgradeRequired`] when the server answers 426.

pub mod client;
pub mod error;
pub mod factory;
pub mod notee;
pub mod oauth;
pub mod types;
pub mod vocab;
pub mod workspace;

pub use client::{Client, user_agent};
pub use error::{ApiError, Problem};
pub use types::*;

pub type Result<T, E = ApiError> = std::result::Result<T, E>;

/// Version of the client, sent in `User-Agent`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
