//! `factory-confirm` and `factory-confirm-cancel` (toolset `admin`): apply or drop a change staged by an admin
//! tool. The server replays the staged request with the caller's own authorization; a request answered with
//! `X-Confirmation: applied` belongs to the action (e.g. 409 `draft-stale`), any other error to the confirmation step
//! (`confirm_expired`, `confirm_used`, `confirm_cancelled`, `confirm_tampered`, …).

use crate::Backend;
use crate::backend::{BackendError, ProblemDetail};
use crate::common::{ToolFailure, ToolOutput, ToolResult, parse_required, writes};
use crate::factory_read::{enc, str_of};
use crate::md::Md;
use crate::out::AppliedOut;
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use ttm_api::confirmations::ApplyOutcome;

pub const CONFIRM: &str = "factory-confirm";
pub const CANCEL: &str = "factory-confirm-cancel";

/// Discovery feature of servers with `/confirmations`.
pub const FEATURE: &str = "confirmations";

/// Codes after which the user must start over with the admin tool (a confirmation runs at most once).
pub const RETRY_CODES: &[&str] = &[
    "confirm_expired",
    "confirm_used",
    "confirm_cancelled",
    "confirm_tampered",
    "draft-stale",
    "upgrade-preview-stale",
    "stale",
];

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfirmArgs {
    /// `confirm_id` returned by an admin tool.
    pub confirm_id: String,
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool::new(
            CONFIRM,
            "Apply a change staged by an admin tool (factory-workflow-publish, factory-agent-draft, …). Call it only after \
             the user explicitly agreed to the summary and preview you showed. A confirmation runs once and expires after \
             10 minutes.",
            schema_for_type::<ConfirmArgs>(),
        )
        .with_output_schema::<AppliedOut>()
        .annotate(writes("Confirm change", true)),
        Tool::new(
            CANCEL,
            "Drop a change staged by an admin tool (the user said no).",
            schema_for_type::<ConfirmArgs>(),
        )
        .with_output_schema::<AppliedOut>()
        .annotate(writes("Cancel change", false)),
    ]
}

pub async fn call<B: Backend + ?Sized>(
    b: &B,
    name: &str,
    args: Option<Map<String, Value>>,
) -> Option<ToolResult> {
    let args: ConfirmArgs = match parse_required(args) {
        Ok(a) => a,
        Err(e) => return Some(Err(e)),
    };
    Some(match name {
        CONFIRM => apply(b, &args.confirm_id).await,
        CANCEL => cancel(b, &args.confirm_id).await,
        _ => return None,
    })
}

pub(crate) fn require_feature<B: Backend + ?Sized>(b: &B) -> Result<(), ToolFailure> {
    if b.supports(FEATURE) {
        Ok(())
    } else {
        Err(ToolFailure::Backend(BackendError::new(
            "unsupported_feature",
            "this server does not support confirmations yet (no `confirmations` in its features)",
        )))
    }
}

/// The error of an apply that did not succeed.
pub fn apply_error(outcome: &ApplyOutcome) -> BackendError {
    let body = &outcome.body;
    let code = str_of(body, "code").unwrap_or("error").to_string();
    let rule = str_of(body, "rule").map(String::from);
    let title = str_of(body, "title").unwrap_or("").to_string();
    let message = if outcome.applied {
        format!("The confirmed change was refused by the server: {title}")
    } else {
        format!("The confirmation could not be applied: {title}")
    };
    BackendError {
        code,
        message,
        problem: Some(Box::new(ProblemDetail {
            status: Some(outcome.status),
            rule,
            field: str_of(body, "field").map(String::from),
            params: body.get("params").and_then(Value::as_object).cloned(),
        })),
    }
}

pub async fn apply<B: Backend + ?Sized>(b: &B, id: &str) -> ToolResult {
    require_feature(b)?;
    let summary = b
        .api("GET", &format!("/confirmations/{}", enc(id)), &[], None)
        .await
        .ok()
        .and_then(|c| str_of(&c, "summary").map(String::from));
    let outcome = b.confirm_apply(id).await?;
    if !outcome.ok() {
        return Err(ToolFailure::Backend(apply_error(&outcome)));
    }
    let out = AppliedOut {
        confirm_id: id.to_string(),
        status: "applied".into(),
        result: Some(outcome.body.clone()),
    };
    let text = Md::title(
        "✓",
        format!("Applied: {}", summary.unwrap_or_else(|| id.to_string())),
    )
    .kv(&[("status", outcome.status.to_string())])
    .build();
    Ok(ToolOutput::with_text(json!(out_value(&out)), text))
}

pub async fn cancel<B: Backend + ?Sized>(b: &B, id: &str) -> ToolResult {
    require_feature(b)?;
    let v = b
        .api("POST", &format!("/confirmations/{}/cancel", enc(id)), &[], None)
        .await?;
    let out = AppliedOut {
        confirm_id: id.to_string(),
        status: str_of(&v, "status").unwrap_or("cancelled").to_string(),
        result: None,
    };
    let text = Md::title("✓", format!("Cancelled {id}: nothing was changed")).build();
    Ok(ToolOutput::with_text(out_value(&out), text))
}

fn out_value(out: &AppliedOut) -> Value {
    crate::common::to_value(out)
}
