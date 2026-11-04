//! Output schemas (`outputSchema` of each tool). `structuredContent` is always a JSON object: lists are wrapped as
//! `{items, next_cursor?}` (or kept under their historical key such as `workspaces`, `results`, `threads`), single
//! objects are returned as they are. Server objects embedded in an output are typed as free-form JSON objects so
//! newer servers can add fields without breaking the schema.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Any object (single resources, write results).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ObjectOut {
    #[serde(flatten)]
    pub fields: Map<String, Value>,
}

/// A list: `{items, next_cursor?}`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ItemsOut {
    #[serde(default)]
    pub items: Vec<Map<String, Value>>,
    /// Cursor of the next page, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Mei-er membership of the signed-in user.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct MembershipOut {
    /// `supporter`, `mei-er` or `sponsor`.
    pub tier: String,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
}

/// `workspace-get-self`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SelfOut {
    pub user: Map<String, Value>,
    #[serde(default)]
    pub workspace: Option<Map<String, Value>>,
    /// `null` for non-members.
    #[serde(default)]
    pub membership: Option<MembershipOut>,
}

/// `workspace-list`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct WorkspacesOut {
    #[serde(default)]
    pub workspaces: Vec<Map<String, Value>>,
}

/// `workspace-get-users`: members with `membership: {tier} | null`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct UsersOut {
    #[serde(default)]
    pub users: Vec<Map<String, Value>>,
}

/// `notee-search`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SearchOut {
    #[serde(default)]
    pub results: Vec<Map<String, Value>>,
}

/// `notee-query-database`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct QueryOut {
    #[serde(default)]
    pub columns: Vec<Value>,
    #[serde(default)]
    pub rows: Vec<Map<String, Value>>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// `notee-get-comments`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ThreadsOut {
    #[serde(default)]
    pub threads: Vec<Map<String, Value>>,
}

/// `notee-create-pages`, `notee-move-pages`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PagesOut {
    #[serde(default)]
    pub pages: Vec<Map<String, Value>>,
}

/// `notee-get-suggestions`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct SuggestionsOut {
    #[serde(default)]
    pub suggestions: Vec<Map<String, Value>>,
}

/// The request a confirmation will apply.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ActionOut {
    pub method: String,
    pub path: String,
}

/// Every admin tool: a staged change waiting for `factory-confirm`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ConfirmOut {
    /// Pass to `factory-confirm` once the user agrees.
    pub confirm_id: String,
    /// RFC 3339; 10 minutes after staging.
    #[serde(default)]
    pub expires_at: Option<String>,
    pub summary: String,
    /// What will change (diff, validation, install plan…).
    #[serde(default)]
    pub preview: Value,
    pub action: ActionOut,
    /// `pending`, or `applied`/`cancelled` when the user answered in the client right away.
    #[serde(default)]
    pub status: Option<String>,
    /// The answer of the applied request (only when applied right away).
    #[serde(default)]
    pub result: Option<Value>,
}

/// `factory-confirm`, `factory-confirm-cancel`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct AppliedOut {
    pub confirm_id: String,
    /// `applied` or `cancelled`.
    pub status: String,
    /// Answer of the confirmed request.
    #[serde(default)]
    pub result: Option<Value>,
}

/// Makes any value an object: arrays become `{items}`, scalars `{value}`.
pub fn object(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        Value::Array(items) => serde_json::json!({ "items": items }),
        Value::Null => serde_json::json!({}),
        other => serde_json::json!({ "value": other }),
    }
}
