//! Suggestion tools: `notee-get-suggestions`, `notee-resolve-suggestion`. Creating a suggestion is
//! `notee-update-page` with `mode: "suggest"`.

use crate::Backend;
use crate::common::{ToolFailure, ToolOutput, ToolResult, parse_required, read_only, to_value, writes};
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::fmt::Write as _;
use ttm_api::notee::resource_id;

pub const GET_SUGGESTIONS: &str = "notee-get-suggestions";
pub const RESOLVE_SUGGESTION: &str = "notee-resolve-suggestion";

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GetArgs {
    /// Page id or URL.
    pub page: String,
    /// Include accepted, rejected and stale suggestions (default: pending only).
    #[serde(default)]
    pub include_resolved: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Accept,
    Reject,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveArgs {
    /// Suggestion id.
    pub suggestion: String,
    pub decision: Decision,
    /// Accept only these changes (default: all). Ignored when rejecting.
    #[serde(default)]
    pub change_ids: Option<Vec<String>>,
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool::new(
            GET_SUGGESTIONS,
            "Suggested edits on a Notee page (tracked changes waiting for review): author, agent, and each change with its \
             quoted anchor and before/after Markdown.",
            schema_for_type::<GetArgs>(),
        )
        .annotate(read_only("Read suggestions")),
        Tool::new(
            RESOLVE_SUGGESTION,
            "Accept (all or some changes) or reject a suggestion. Needs edit access. Changes whose anchor text is gone come \
             back as stale.",
            schema_for_type::<ResolveArgs>(),
        )
        .annotate(writes("Resolve suggestion", true)),
    ]
}

pub async fn call<B: Backend + ?Sized>(
    backend: &B,
    name: &str,
    args: Option<Map<String, Value>>,
) -> Option<ToolResult> {
    Some(match name {
        GET_SUGGESTIONS => get(backend, args).await,
        RESOLVE_SUGGESTION => resolve(backend, args).await,
        _ => return None,
    })
}

async fn get<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: GetArgs = parse_required(args)?;
    let list: Vec<_> = backend
        .notee_get_suggestions(&resource_id(&args.page))
        .await?
        .into_iter()
        .filter(|s| args.include_resolved || s.status == "pending")
        .collect();
    let mut text = String::new();
    if list.is_empty() {
        text.push_str("No pending suggestions.");
    }
    for s in &list {
        let who = s.author.as_ref().map(|a| a.name.as_str()).unwrap_or("someone");
        let via = s
            .via_agent
            .as_deref()
            .map(|a| format!(" via {a}"))
            .unwrap_or_default();
        let _ = writeln!(text, "Suggestion {} by {who}{via} ({}):", s.id, s.status);
        for c in &s.changes {
            let _ = writeln!(
                text,
                "  - [{}] {}: {:?} -> {:?}",
                c.id.as_deref().unwrap_or("-"),
                c.kind,
                c.before_nfm.as_deref().unwrap_or(""),
                c.after_nfm.as_deref().unwrap_or("")
            );
        }
    }
    Ok(ToolOutput::with_text(json!({ "suggestions": list }), text))
}

async fn resolve<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: ResolveArgs = parse_required(args)?;
    let accept = matches!(args.decision, Decision::Accept);
    if !accept && args.change_ids.is_some() {
        return Err(ToolFailure::InvalidArgs(
            "change_ids only applies to accept".into(),
        ));
    }
    let result = backend
        .notee_resolve_suggestion(&args.suggestion, accept, args.change_ids.as_deref())
        .await?;
    Ok(ToolOutput::json(to_value(&result)))
}
