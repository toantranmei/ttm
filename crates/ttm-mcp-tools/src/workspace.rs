//! Workspace tools: `workspace-get-self`, `workspace-list`, `workspace-get-users`.

use crate::Backend;
use crate::common::{ToolOutput, ToolResult, parse, read_only, to_value};
use crate::md::{Md, badge};
use crate::out::{SelfOut, UsersOut, WorkspacesOut};
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
        .with_output_schema::<SelfOut>()
        .annotate(read_only("Who am I")),
        Tool::new(
            LIST,
            "Workspaces the user belongs to, with kind (personal/team) and the user's role in each.",
            schema_for_empty_input(),
        )
        .with_output_schema::<WorkspacesOut>()
        .annotate(read_only("List workspaces")),
        Tool::new(
            GET_USERS,
            "Members of a workspace (people; agents join later) with their role. Use to find someone to mention or assign.",
            schema_for_type::<GetUsersArgs>(),
        )
        .with_output_schema::<UsersOut>()
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
        LIST => list(backend).await,
        GET_USERS => get_users(backend, args).await,
        _ => return None,
    })
}

async fn get_self<B: Backend + ?Sized>(backend: &B) -> ToolResult {
    let me = backend.get_self().await?;
    let membership = me
        .user
        .membership
        .as_ref()
        .map(|m| json!({ "tier": m.tier, "since": m.since, "until": m.until }));
    let mut value = to_value(&me);
    value["membership"] = membership.clone().unwrap_or(Value::Null);
    let tier = me.user.membership.as_ref().map(|m| m.tier.as_str());
    let mut kv = vec![
        ("email", me.user.email.clone()),
        ("id", me.user.id.clone()),
        ("plan", me.user.plan.clone().unwrap_or_default()),
    ];
    if let Some(m) = &me.user.membership {
        kv.push((
            "membership",
            format!(
                "{}{}",
                m.tier,
                m.since
                    .as_deref()
                    .map(|s| format!(" since {}", &s[..s.len().min(7)]))
                    .unwrap_or_default()
            ),
        ));
    }
    if let Some(w) = &me.workspace {
        kv.push((
            "workspace",
            format!("{} ({}, {}, id {})", w.name, w.kind, w.role, w.id),
        ));
    }
    let text = Md::title("👤", format!("{}{}", me.user.name, badge(tier)))
        .kv(&kv)
        .next(&[
            ("workspace-list", "other workspaces"),
            ("notee-search", "find pages"),
        ])
        .build();
    Ok(ToolOutput::with_text(value, text))
}

async fn list<B: Backend + ?Sized>(backend: &B) -> ToolResult {
    let workspaces = backend.list_workspaces().await?;
    let rows: Vec<Vec<String>> = workspaces
        .iter()
        .map(|w| {
            vec![
                format!(
                    "{}{}",
                    w.name,
                    badge(w.owner_membership.as_ref().map(|m| m.tier.as_str()))
                ),
                w.kind.clone(),
                w.role.clone(),
                w.id.clone(),
            ]
        })
        .collect();
    let text = Md::title("○", format!("{} workspace(s)", workspaces.len()))
        .table(&["Name", "Kind", "Role", "Id"], &rows)
        .next(&[("workspace-get-users", "members of one")])
        .build();
    Ok(ToolOutput::with_text(json!({ "workspaces": workspaces }), text))
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
    let rows: Vec<Vec<String>> = members
        .iter()
        .map(|m| {
            let tier = m
                .membership
                .as_ref()
                .or(m.user.membership.as_ref())
                .map(|t| t.tier.as_str());
            vec![
                format!("👤 {}{}", m.user.name, badge(tier)),
                m.user.email.clone(),
                m.role.clone(),
                m.user.id.clone(),
            ]
        })
        .collect();
    let text = Md::title("👤", format!("{} member(s)", members.len()))
        .table(&["Name", "E-mail", "Role", "Id"], &rows)
        .build();
    Ok(ToolOutput::with_text(json!({ "users": members }), text))
}
