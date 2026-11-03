//! Workspace tools: `workspace-get-self`, `workspace-list`, `workspace-get-users`.

use crate::{Backend, BackendError};
use rmcp::handler::server::common::{schema_for_empty_input, schema_for_type};
use rmcp::model::{Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

pub const GET_SELF: &str = "workspace-get-self";
pub const LIST: &str = "workspace-list";
pub const GET_USERS: &str = "workspace-get-users";

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetUsersArgs {
    /// Workspace id; defaults to the current workspace.
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Case-insensitive filter on name or e-mail.
    #[serde(default)]
    pub query: Option<String>,
}

fn read_only(title: &str) -> ToolAnnotations {
    ToolAnnotations::with_title(title)
        .read_only(true)
        .destructive(false)
        .idempotent(true)
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool::new(
            GET_SELF,
            "The signed-in user (id, name, e-mail, plan) and the current workspace. Call this first to know who you act for.",
            schema_for_empty_input(),
        )
        .annotate(read_only("Who am I")),
        Tool::new(
            LIST,
            "Workspaces the user belongs to, with kind (personal/team) and the user's role in each.",
            schema_for_empty_input(),
        )
        .annotate(read_only("List workspaces")),
        Tool::new(
            GET_USERS,
            "Members of a workspace (people; agents join later) with their role. Use to find someone to mention or assign.",
            schema_for_type::<GetUsersArgs>(),
        )
        .annotate(read_only("Workspace members")),
    ]
}

/// Runs a workspace tool. `Ok(None)` means the name is not a workspace tool.
pub async fn call<B: Backend>(
    backend: &B,
    name: &str,
    args: Option<serde_json::Map<String, Value>>,
) -> Option<Result<Value, ToolFailure>> {
    let result = match name {
        GET_SELF => backend
            .get_self()
            .await
            .map(|s| json!(s))
            .map_err(ToolFailure::Backend),
        LIST => backend
            .list_workspaces()
            .await
            .map(|w| json!({ "workspaces": w }))
            .map_err(ToolFailure::Backend),
        GET_USERS => match parse::<GetUsersArgs>(args) {
            Ok(args) => match backend.get_users(args.workspace_id.as_deref()).await {
                Ok(members) => {
                    let query = args
                        .query
                        .as_deref()
                        .map(str::to_lowercase)
                        .filter(|q| !q.is_empty());
                    let members: Vec<_> = members
                        .into_iter()
                        .filter(|m| {
                            query.as_deref().is_none_or(|q| {
                                m.user.name.to_lowercase().contains(q)
                                    || m.user.email.to_lowercase().contains(q)
                            })
                        })
                        .collect();
                    Ok(json!({ "users": members }))
                }
                Err(err) => Err(ToolFailure::Backend(err)),
            },
            Err(err) => Err(err),
        },
        _ => return None,
    };
    Some(result)
}

/// Why a tool call failed.
#[derive(Debug)]
pub enum ToolFailure {
    /// Bad arguments: a protocol error (`invalid_params`).
    InvalidArgs(String),
    /// The backend refused or failed: a tool result with `isError: true`, so the model can react.
    Backend(BackendError),
}

pub fn parse<T: for<'de> Deserialize<'de> + Default>(
    args: Option<serde_json::Map<String, Value>>,
) -> Result<T, ToolFailure> {
    match args {
        None => Ok(T::default()),
        Some(map) => {
            serde_json::from_value(Value::Object(map)).map_err(|e| ToolFailure::InvalidArgs(e.to_string()))
        }
    }
}
