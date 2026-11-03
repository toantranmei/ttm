//! Shared plumbing of the tool modules.

use crate::BackendError;
use rmcp::model::ToolAnnotations;
use serde::Deserialize;
use serde_json::{Map, Value};

/// What a tool returns: `value` becomes `structuredContent`; `text` (Markdown for the model) becomes the text content,
/// else the JSON of `value` is used.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolOutput {
    pub value: Value,
    pub text: Option<String>,
}

impl ToolOutput {
    pub fn json(value: Value) -> Self {
        Self { value, text: None }
    }

    pub fn with_text(value: Value, text: impl Into<String>) -> Self {
        Self {
            value,
            text: Some(text.into()),
        }
    }
}

/// Why a tool call failed.
#[derive(Debug)]
pub enum ToolFailure {
    /// Bad arguments: a protocol error (`invalid_params`).
    InvalidArgs(String),
    /// The backend refused or failed: a tool result with `isError: true`, so the model can react.
    Backend(BackendError),
}

impl From<BackendError> for ToolFailure {
    fn from(err: BackendError) -> Self {
        ToolFailure::Backend(err)
    }
}

pub type ToolResult = Result<ToolOutput, ToolFailure>;

/// Parses the arguments object; missing arguments are the default value.
pub fn parse<T: for<'de> Deserialize<'de> + Default>(
    args: Option<Map<String, Value>>,
) -> Result<T, ToolFailure> {
    match args {
        None => Ok(T::default()),
        Some(map) => {
            serde_json::from_value(Value::Object(map)).map_err(|e| ToolFailure::InvalidArgs(e.to_string()))
        }
    }
}

/// Like [`parse`] for argument types without a sensible default (required fields).
pub fn parse_required<T: for<'de> Deserialize<'de>>(
    args: Option<Map<String, Value>>,
) -> Result<T, ToolFailure> {
    serde_json::from_value(Value::Object(args.unwrap_or_default()))
        .map_err(|e| ToolFailure::InvalidArgs(e.to_string()))
}

pub fn read_only(title: &str) -> ToolAnnotations {
    ToolAnnotations::with_title(title)
        .read_only(true)
        .destructive(false)
        .idempotent(true)
        .open_world(false)
}

pub fn writes(title: &str, destructive: bool) -> ToolAnnotations {
    ToolAnnotations::with_title(title)
        .read_only(false)
        .destructive(destructive)
        .idempotent(false)
        .open_world(false)
}

pub fn to_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}
