//! MCP tools of ttm, written once over the [`Backend`] trait.
//!
//! - `ttm-mcp` (stdio, on the user's machine) uses [`HttpBackend`], which calls `/api/v1` with the user's token.
//! - The hosted MCP server of toantranmei.com implements [`Backend`] directly on its services, so both expose the same
//!   tools with the same names, schemas and outputs.
//!
//! [`ToolSet`] implements [`rmcp::ServerHandler`]: it lists the tools of the enabled [`Toolsets`] and dispatches calls.

pub mod backend;
pub mod common;
pub mod notee;
pub mod notee_write;
pub mod toolset;
pub mod workspace;

pub use backend::{Backend, BackendError, BackendResult, Fetched, HttpBackend, SelfInfo};
pub use common::{CallContext, ToolFailure, ToolOutput};
pub use toolset::{ToolSet, Toolset, Toolsets};
