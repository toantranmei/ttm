//! Workspace tools: `workspace-get-self`, `workspace-list`, `workspace-get-users`.

use crate::Backend;
use crate::common::{ToolOutput, ToolResult, parse, read_only, to_value};
use rmcp::handler::server::common::{schema_for_empty_input, schema_for_type};
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};

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

/// Runs a workspace tool; `None` when `name` is not one.
pub async fn call<B: Backend + ?Sized>(
    backend: &B,
    name: &str,
    args: Option<Map<String, Value>>,
) -> Option<ToolResult> {
    Some(match name {
        GET_SELF => get_self(backend).await,
        LIST => backend
            .list_workspaces()
            .await
            .map(|w| ToolOutput::json(json!({ "workspaces": w })))
            .map_err(Into::into),
        GET_USERS => get_users(backend, args).await,
        _ => return None,
    })
}

async fn get_self<B: Backend + ?Sized>(backend: &B) -> ToolResult {
    Ok(ToolOutput::json(to_value(&backend.get_self().await?)))
}

async fn get_users<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: GetUsersArgs = parse(args)?;
    let members = backend.get_users(args.workspace_id.as_deref()).await?;
    let query = args
        .query
        .as_deref()
        .map(str::to_lowercase)
        .filter(|q| !q.is_empty());
    let members: Vec<_> = members
        .into_iter()
        .filter(|m| {
            query.as_deref().is_none_or(|q| {
                m.user.name.to_lowercase().contains(q) || m.user.email.to_lowercase().contains(q)
            })
        })
        .collect();
    Ok(ToolOutput::json(json!({ "users": members })))
}
