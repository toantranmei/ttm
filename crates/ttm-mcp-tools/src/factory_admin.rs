//! Admin tools (toolset `admin`, scope `factory:admin`; `factory-task-approve`: `factory:write`). Never offered to an
//! agent running a job.
//!
//! Every tool works in two steps and never applies a change itself:
//! 1. it reads what the change would do (validation, diff, upgrade or install preview);
//! 2. it stages the request with `POST /confirmations {method, path, body, summary, preview}` and returns the
//!    `confirm_id` (valid 10 minutes). The change is applied by `factory-confirm` once the user agrees (or right away
//!    when the client answers an elicitation, see `ToolSet`).

use crate::Backend;
use crate::backend::BackendError;
use crate::common::{ToolFailure, ToolOutput, ToolResult, parse_required, writes};
use crate::factory_read::{
    affected_rules, affected_rules_md, diff_rows, enc, find_agent, find_rule, find_skill, get, install_md,
    project, rule_md, str_of, upgrade_md, workflow_id, workspace,
};
use crate::md::{Md, glyph};
use crate::out::{ActionOut, ConfirmOut};
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};

pub const WORKFLOW_DRAFT_UPDATE: &str = "factory-workflow-draft-update";
pub const WORKFLOW_PUBLISH: &str = "factory-workflow-publish";
pub const WORKFLOW_UPGRADE: &str = "factory-workflow-upgrade";
pub const AUTOMATION_DRAFT: &str = "factory-automation-draft";
pub const AUTOMATION_SET_ENABLED: &str = "factory-automation-set-enabled";
pub const AGENT_DRAFT: &str = "factory-agent-draft";
pub const AGENT_SET_STATUS: &str = "factory-agent-set-status";
pub const SKILL_REVIEW: &str = "factory-skill-review";
pub const TEMPLATE_INSTALL: &str = "factory-template-install";
pub const TASK_APPROVE: &str = "factory-task-approve";

/// Stages `method path body` as a confirmation and answers its `confirm_id` with the preview.
pub async fn stage<B: Backend + ?Sized>(
    b: &B,
    method: &str,
    path: &str,
    body: Value,
    summary: String,
    preview: Value,
    preview_md: Md,
) -> ToolResult {
    crate::confirm::require_feature(b)?;
    if !ttm_api::confirmations::is_confirmable(method, path) {
        return Err(ToolFailure::Backend(BackendError::new(
            "not_confirmable",
            format!("{method} {path} cannot be confirmed"),
        )));
    }
    let staged = b
        .api(
            "POST",
            "/confirmations",
            &[],
            Some(json!({ "method": method, "path": path, "body": body, "summary": summary, "preview": preview })),
        )
        .await?;
    let out = ConfirmOut {
        confirm_id: str_of(&staged, "confirm_id").unwrap_or_default().to_string(),
        expires_at: str_of(&staged, "expires_at").map(String::from),
        summary: summary.clone(),
        preview,
        action: ActionOut {
            method: method.to_string(),
            path: path.to_string(),
        },
        status: Some("pending".into()),
        result: None,
    };
    if out.confirm_id.is_empty() {
        return Err(ToolFailure::Backend(BackendError::new(
            "unsupported",
            "this server does not support confirmations yet",
        )));
    }
    let text = Md::title("!", format!("Confirm: {summary}"))
        .extend(preview_md)
        .kv(&[
            ("confirm_id", out.confirm_id.clone()),
            ("expires", out.expires_at.clone().unwrap_or_default()),
        ])
        .next(&[(
            crate::confirm::CONFIRM,
            &format!(
                "with confirm_id `{}` only after the user agrees (expires in 10 minutes); `{}` to drop it",
                out.confirm_id,
                crate::confirm::CANCEL
            ),
        )])
        .build();
    Ok(ToolOutput::with_text(crate::common::to_value(&out), text))
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DraftUpdateArgs {
    /// The new definition (`{states, transitions}`), complete.
    pub definition: Value,
    #[serde(default)]
    pub workflow_id: Option<String>,
    /// Project key or id whose workflow to edit.
    #[serde(default)]
    pub project: Option<String>,
    /// New workflow name.
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublishArgs {
    /// What changed, for the projects using it (≤ 280 characters).
    pub note: String,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpgradeArgs {
    #[serde(default)]
    pub project: Option<String>,
    /// Target version; default: the latest.
    #[serde(default)]
    pub to: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleDraftArgs {
    /// Existing rule (id, slug or name) to change; omit to create one.
    #[serde(default)]
    pub rule: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    pub name: String,
    /// `{type: "task.moved", to?: "<status>"}` …
    pub trigger: Value,
    /// `{match: "all"|"any", filters: [...]}`.
    #[serde(default)]
    pub conditions: Option<Value>,
    /// `[{type: "assign", …}]`.
    pub actions: Vec<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetEnabledArgs {
    /// Rule id, slug or name.
    pub rule: String,
    pub enabled: bool,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentDraftArgs {
    /// Existing agent (handle, name or id) to change; omit to create one.
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Preset code (from the agent presets) to start from, for a new agent.
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// Short profile (≤ 280).
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub git_name: Option<String>,
    #[serde(default)]
    pub git_email: Option<String>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub max_budget_tokens: Option<i64>,
    #[serde(default)]
    pub daily_budget_tokens: Option<i64>,
    #[serde(default)]
    pub max_turns: Option<i64>,
    /// MCP grants `[{server: "<id>"|"local:<name>", tools: ["*"], required?}]`.
    #[serde(default)]
    pub mcp: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Paused,
    Archived,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentStatusArgs {
    pub agent: String,
    pub status: AgentStatus,
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Approve,
    Reject,
}

impl Decision {
    fn as_str(self) -> &'static str {
        match self {
            Decision::Approve => "approve",
            Decision::Reject => "reject",
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillReviewArgs {
    /// Skill slug or id.
    pub skill: String,
    pub version: i64,
    pub decision: Decision,
    /// Required to reject.
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstallArgs {
    /// Catalog item id.
    pub item: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub version: Option<i64>,
    /// Values for the template inputs.
    #[serde(default)]
    pub inputs: Option<Value>,
    /// Preset code → `new` | `reuse:<agent_id>` | `skip`.
    #[serde(default)]
    pub agents: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskApproveArgs {
    /// Task key (AUR-12) waiting at a human approval gate.
    pub task: String,
    pub decision: Decision,
    #[serde(default)]
    pub note: Option<String>,
}

pub fn tools() -> Vec<Tool> {
    let t =
        |name: &'static str, description: &'static str, schema, title: &'static str, destructive: bool| {
            Tool::new(name, description, schema)
                .with_output_schema::<ConfirmOut>()
                .annotate(writes(title, destructive))
        };
    vec![
        t(
            WORKFLOW_DRAFT_UPDATE,
            "Replace the draft of a workflow. Returns the validation of the new definition and a confirm_id; nothing changes until factory-confirm.",
            schema_for_type::<DraftUpdateArgs>(),
            "Update workflow draft",
            false,
        ),
        t(
            WORKFLOW_PUBLISH,
            "Publish the draft of a workflow as a new version. Returns the diff, the Automation rules naming states the version removes or changes (they stop firing; confirming acknowledges them) and a confirm_id; projects are told to upgrade only after factory-confirm.",
            schema_for_type::<PublishArgs>(),
            "Publish workflow",
            false,
        ),
        t(
            WORKFLOW_UPGRADE,
            "Move a project to a newer workflow version. Returns where each task would move, the project's Automation rules naming states the version removes or changes (they stop firing; confirming acknowledges them) and a confirm_id.",
            schema_for_type::<UpgradeArgs>(),
            "Upgrade project workflow",
            true,
        ),
        t(
            AUTOMATION_DRAFT,
            "Create or change an Automation rule (when … if … then …). Returns the rule as it would be and a confirm_id.",
            schema_for_type::<RuleDraftArgs>(),
            "Draft Automation rule",
            false,
        ),
        t(
            AUTOMATION_SET_ENABLED,
            "Turn an Automation rule on or off. Returns a confirm_id.",
            schema_for_type::<SetEnabledArgs>(),
            "Enable or disable rule",
            false,
        ),
        t(
            AGENT_DRAFT,
            "Create an agent (from a preset or field by field) or change one: name, handle, role, model, bio, git identity, budgets, MCP. Returns the change and a confirm_id.",
            schema_for_type::<AgentDraftArgs>(),
            "Draft agent",
            false,
        ),
        t(
            AGENT_SET_STATUS,
            "Pause, resume or archive an agent. Returns a confirm_id.",
            schema_for_type::<AgentStatusArgs>(),
            "Set agent status",
            true,
        ),
        t(
            SKILL_REVIEW,
            "Approve or reject a skill version waiting for review. Returns the diff against the version in effect and a confirm_id.",
            schema_for_type::<SkillReviewArgs>(),
            "Review skill",
            false,
        ),
        t(
            TEMPLATE_INSTALL,
            "Install a template from the library into a workspace. Returns what it creates and a confirm_id.",
            schema_for_type::<InstallArgs>(),
            "Install template",
            false,
        ),
        t(
            TASK_APPROVE,
            "Approve or reject the pending human approval of a task. Returns the task and a confirm_id; the decision is recorded only after factory-confirm.",
            schema_for_type::<TaskApproveArgs>(),
            "Approve task",
            false,
        ),
    ]
}

macro_rules! run {
    ($args:expr, $f:expr) => {
        match parse_required($args) {
            Ok(a) => $f(a).await,
            Err(e) => Err(e),
        }
    };
}

pub async fn call<B: Backend + ?Sized>(
    b: &B,
    name: &str,
    args: Option<Map<String, Value>>,
) -> Option<ToolResult> {
    Some(match name {
        WORKFLOW_DRAFT_UPDATE => run!(args, |a| draft_update(b, a)),
        WORKFLOW_PUBLISH => run!(args, |a| publish(b, a)),
        WORKFLOW_UPGRADE => run!(args, |a| upgrade(b, a)),
        AUTOMATION_DRAFT => run!(args, |a| rule_draft(b, a)),
        AUTOMATION_SET_ENABLED => run!(args, |a| set_enabled(b, a)),
        AGENT_DRAFT => run!(args, |a| agent_draft(b, a)),
        AGENT_SET_STATUS => run!(args, |a| agent_status(b, a)),
        SKILL_REVIEW => run!(args, |a| skill_review(b, a)),
        TEMPLATE_INSTALL => run!(args, |a| install(b, a)),
        TASK_APPROVE => run!(args, |a| task_approve(b, a)),
        _ => return None,
    })
}

async fn workflow<B: Backend + ?Sized>(
    b: &B,
    workflow: Option<&str>,
    project_arg: Option<&str>,
) -> Result<(String, Value), ToolFailure> {
    let (id, _) = workflow_id(b, workflow, project_arg).await?;
    let w = get(b, &format!("/factory/workflows/{}", enc(&id)), &[]).await?;
    Ok((id, w))
}

fn base_version(w: &Value) -> i64 {
    w.get("latest_version")
        .or_else(|| w.get("version"))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

async fn draft_update<B: Backend + ?Sized>(b: &B, a: DraftUpdateArgs) -> ToolResult {
    let (id, w) = workflow(b, a.workflow_id.as_deref(), a.project.as_deref()).await?;
    let validation = b
        .api(
            "POST",
            &format!("/factory/workflows/{}/validate", enc(&id)),
            &[],
            Some(json!({ "definition": a.definition })),
        )
        .await?;
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = issues
        .iter()
        .map(|i| {
            vec![
                str_of(i, "code").unwrap_or_default().to_string(),
                str_of(i, "field").unwrap_or_default().to_string(),
                str_of(i, "message").unwrap_or_default().to_string(),
            ]
        })
        .collect();
    let md = crate::factory_read::workflow_md(
        a.name.as_deref().unwrap_or(str_of(&w, "name").unwrap_or("")),
        "new draft",
        &a.definition,
    )
    .text(if issues.is_empty() {
        "✓ The definition is valid.".to_string()
    } else {
        format!(
            "✕ {} issue(s): the draft is saved but cannot be published until they are fixed.",
            issues.len()
        )
    })
    .table(&["Code", "Field", "Message"], &rows);
    let body = json!({ "name": a.name, "definition": a.definition, "base_version": base_version(&w) });
    let summary = format!("Save the draft of workflow {}", str_of(&w, "name").unwrap_or(&id));
    stage(
        b,
        "PUT",
        &format!("/factory/workflows/{id}/draft"),
        body,
        summary,
        json!({ "issues": issues }),
        md,
    )
    .await
}

async fn publish<B: Backend + ?Sized>(b: &B, a: PublishArgs) -> ToolResult {
    if a.note.chars().count() > 280 {
        return Err(ToolFailure::InvalidArgs(
            "note is longer than 280 characters".into(),
        ));
    }
    let (id, w) = workflow(b, a.workflow_id.as_deref(), a.project.as_deref()).await?;
    if !w.get("has_draft").and_then(Value::as_bool).unwrap_or(true) {
        return Err(ToolFailure::Backend(BackendError::new(
            "no_draft",
            "this workflow has no draft to publish",
        )));
    }
    let base = base_version(&w);
    let diff = get(
        b,
        &format!("/factory/workflows/{}/diff", enc(&id)),
        &[("from", base.to_string()), ("to", "draft".into())],
    )
    .await?;
    let affected = affected_rules(&diff);
    let md = Md::title("◐", format!("v{base} → v{} (draft)", base + 1))
        .table(&["Change", "Item"], &diff_rows(&diff));
    let md = affected_rules_md(md, &affected).kv(&[("note", a.note.clone())]);
    let summary = format!(
        "Publish workflow {} v{}{}",
        str_of(&w, "name").unwrap_or(&id),
        base + 1,
        rules_note(&affected)
    );
    // The rules are in the preview the user confirms: confirming acknowledges them (else the server answers 409
    // `rules_affected`).
    let mut body = json!({ "note": a.note, "base_version": base });
    if !affected.is_empty() {
        body["acknowledge_rules"] = json!(true);
    }
    stage(
        b,
        "POST",
        &format!("/factory/workflows/{id}/publish"),
        body,
        summary,
        json!({ "diff": diff, "affected_rules": affected }),
        md,
    )
    .await
}

fn rules_note(affected: &[Value]) -> String {
    if affected.is_empty() {
        String::new()
    } else {
        format!(" ({} Automation rule(s) stop firing)", affected.len())
    }
}

async fn upgrade<B: Backend + ?Sized>(b: &B, a: UpgradeArgs) -> ToolResult {
    let p = project(b, a.project.as_deref()).await?;
    let id = str_of(&p, "id").unwrap_or_default().to_string();
    let q: Vec<(&str, String)> = a.to.map(|t| vec![("to", t.to_string())]).unwrap_or_default();
    let preview = get(b, &format!("/factory/projects/{}/workflow-upgrade", enc(&id)), &q).await?;
    let to = preview.get("to_version").and_then(Value::as_i64).unwrap_or(0);
    let affected = affected_rules(&preview);
    let mut body = json!({ "to_version": to, "preview_hash": str_of(&preview, "preview_hash") });
    if !affected.is_empty() {
        body["acknowledge_rules"] = json!(true);
    }
    let summary = format!(
        "Upgrade project {} to workflow v{to}{}",
        str_of(&p, "key").unwrap_or(&id),
        rules_note(&affected)
    );
    let md = upgrade_md(&preview);
    stage(
        b,
        "POST",
        &format!("/factory/projects/{id}/workflow-upgrade"),
        body,
        summary,
        preview,
        md,
    )
    .await
}

async fn rule_draft<B: Backend + ?Sized>(b: &B, a: RuleDraftArgs) -> ToolResult {
    let mut rule = json!({
        "name": a.name, "trigger": a.trigger,
        "conditions": a.conditions.unwrap_or_else(|| json!({"match": "all", "filters": []})),
        "actions": a.actions,
    });
    match &a.rule {
        Some(existing) => {
            let current = find_rule(b, existing, a.project.as_deref()).await?;
            let id = str_of(&current, "id").unwrap_or_default().to_string();
            let md = Md::title("○", "Now")
                .extend(rule_md(&current))
                .text("Becomes:")
                .extend(rule_md(&rule));
            let summary = format!(
                "Change Automation rule {}",
                str_of(&current, "name").unwrap_or(&id)
            );
            stage(
                b,
                "PATCH",
                &format!("/factory/automation/rules/{id}"),
                rule.clone(),
                summary,
                json!({ "before": current, "after": rule }),
                md,
            )
            .await
        }
        None => {
            let p = project(b, a.project.as_deref()).await?;
            rule["project_id"] = json!(str_of(&p, "id"));
            if let Some(w) = str_of(&p, "workflow_id") {
                rule["workflow_id"] = json!(w);
            }
            let md = rule_md(&rule);
            let summary = format!("Create Automation rule {}", a.name);
            stage(
                b,
                "POST",
                "/factory/automation/rules",
                rule.clone(),
                summary,
                json!({ "rule": rule }),
                md,
            )
            .await
        }
    }
}

async fn set_enabled<B: Backend + ?Sized>(b: &B, a: SetEnabledArgs) -> ToolResult {
    let rule = find_rule(b, &a.rule, a.project.as_deref()).await?;
    let id = str_of(&rule, "id").unwrap_or_default().to_string();
    let summary = format!(
        "{} Automation rule {}",
        if a.enabled { "Turn on" } else { "Turn off" },
        str_of(&rule, "name").unwrap_or(&id)
    );
    let md = rule_md(&rule);
    stage(
        b,
        "POST",
        &format!("/factory/automation/rules/{id}/enabled"),
        json!({ "enabled": a.enabled }),
        summary,
        json!({ "rule": rule, "enabled": a.enabled }),
        md,
    )
    .await
}

fn agent_body(a: &AgentDraftArgs) -> Value {
    let mut body = Map::new();
    let mut put = |k: &str, v: Option<Value>| {
        if let Some(v) = v {
            body.insert(k.to_string(), v);
        }
    };
    put("preset", a.preset.clone().map(Value::from));
    put("name", a.name.clone().map(Value::from));
    put("handle", a.handle.clone().map(Value::from));
    put("role", a.role.clone().map(Value::from));
    put("model", a.model.clone().map(Value::from));
    put("bio", a.bio.clone().map(Value::from));
    put("git_name", a.git_name.clone().map(Value::from));
    put("git_email", a.git_email.clone().map(Value::from));
    put("instructions", a.instructions.clone().map(Value::from));
    put("max_budget_tokens", a.max_budget_tokens.map(Value::from));
    put("daily_budget_tokens", a.daily_budget_tokens.map(Value::from));
    put("max_turns", a.max_turns.map(Value::from));
    put("profile", a.mcp.clone().map(|m| json!({ "mcp": m })));
    Value::Object(body)
}

fn changes_md(title: &str, before: Option<&Value>, after: &Value) -> Md {
    let rows: Vec<Vec<String>> = after
        .as_object()
        .into_iter()
        .flatten()
        .map(|(k, v)| {
            let old = before.and_then(|b| b.get(k)).map(show).unwrap_or_default();
            vec![k.clone(), old, show(v)]
        })
        .collect();
    Md::title("○", title).table(&["Field", "Now", "Becomes"], &rows)
}

fn show(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

async fn agent_draft<B: Backend + ?Sized>(b: &B, a: AgentDraftArgs) -> ToolResult {
    let ws = workspace(b, a.workspace_id.as_deref()).await?;
    let body = agent_body(&a);
    if body.as_object().is_none_or(Map::is_empty) {
        return Err(ToolFailure::InvalidArgs("nothing to set".into()));
    }
    match &a.agent {
        Some(existing) => {
            let current = find_agent(b, &ws, existing).await?;
            let id = str_of(&current, "id").unwrap_or_default().to_string();
            let name = str_of(&current, "name").unwrap_or(&id).to_string();
            let md = changes_md(&format!("Agent {name}"), Some(&current), &body);
            stage(
                b,
                "PATCH",
                &format!("/workspaces/{ws}/agents/{id}"),
                body.clone(),
                format!("Change agent {name}"),
                json!({ "before": current, "changes": body }),
                md,
            )
            .await
        }
        None => {
            if a.name.is_none() && a.preset.is_none() {
                return Err(ToolFailure::InvalidArgs(
                    "a new agent needs `name` or `preset`".into(),
                ));
            }
            let label = a.name.clone().or(a.preset.clone()).unwrap_or_default();
            let md = changes_md(&format!("New agent {label}"), None, &body);
            stage(
                b,
                "POST",
                &format!("/workspaces/{ws}/agents"),
                body.clone(),
                format!("Create agent {label}"),
                json!({ "agent": body }),
                md,
            )
            .await
        }
    }
}

async fn agent_status<B: Backend + ?Sized>(b: &B, a: AgentStatusArgs) -> ToolResult {
    let ws = workspace(b, a.workspace_id.as_deref()).await?;
    let current = find_agent(b, &ws, &a.agent).await?;
    let id = str_of(&current, "id").unwrap_or_default().to_string();
    let name = str_of(&current, "name").unwrap_or(&id).to_string();
    let status = match a.status {
        AgentStatus::Active => "active",
        AgentStatus::Paused => "paused",
        AgentStatus::Archived => "archived",
    };
    let verb = match a.status {
        AgentStatus::Active => "Resume",
        AgentStatus::Paused => "Pause",
        AgentStatus::Archived => "Archive",
    };
    let body = json!({ "status": status });
    let md = changes_md(&format!("Agent {name}"), Some(&current), &body);
    stage(
        b,
        "PATCH",
        &format!("/workspaces/{ws}/agents/{id}"),
        body,
        format!("{verb} agent {name}"),
        json!({ "agent": current, "status": status }),
        md,
    )
    .await
}

async fn skill_review<B: Backend + ?Sized>(b: &B, a: SkillReviewArgs) -> ToolResult {
    if a.decision == Decision::Reject && a.note.as_deref().is_none_or(|n| n.trim().is_empty()) {
        return Err(ToolFailure::InvalidArgs("a note is required to reject".into()));
    }
    let skill = find_skill(b, &a.skill, a.workspace_id.as_deref()).await?;
    let id = str_of(&skill, "id").unwrap_or_default().to_string();
    let slug = str_of(&skill, "slug").unwrap_or(&id).to_string();
    let diff = get(
        b,
        &format!("/factory/skills/{}/versions/{}/diff", enc(&id), a.version),
        &[],
    )
    .await
    .unwrap_or(Value::Null);
    let rows: Vec<Vec<String>> = diff
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|f| str_of(f, "status") != Some("same"))
        .map(|f| {
            vec![
                str_of(f, "status").unwrap_or("").to_string(),
                str_of(f, "path").unwrap_or("").to_string(),
            ]
        })
        .collect();
    let mut md = Md::title("◐", format!("Skill {slug} v{}", a.version)).table(&["Change", "File"], &rows);
    for f in diff.get("files").and_then(Value::as_array).into_iter().flatten() {
        if let Some(u) = str_of(f, "unified") {
            md = md.text(format!("```diff\n{u}\n```"));
        }
    }
    let summary = format!(
        "{} skill {slug} v{}",
        if a.decision == Decision::Approve {
            "Approve"
        } else {
            "Reject"
        },
        a.version
    );
    let body = json!({ "decision": a.decision.as_str(), "note": a.note });
    stage(
        b,
        "POST",
        &format!("/factory/skills/{id}/versions/{}/review", a.version),
        body,
        summary,
        json!({ "diff": diff }),
        md,
    )
    .await
}

async fn install<B: Backend + ?Sized>(b: &B, a: InstallArgs) -> ToolResult {
    let ws = workspace(b, a.workspace_id.as_deref()).await?;
    let preview = b
        .api(
            "POST",
            &format!("/catalog/items/{}/install-preview", enc(&a.item)),
            &[],
            Some(json!({ "workspace_id": ws, "version": a.version })),
        )
        .await?;
    if preview
        .get("requires_purchase")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && !preview.get("owned").and_then(Value::as_bool).unwrap_or(false)
    {
        return Err(ToolFailure::Backend(BackendError::new(
            "purchase_required",
            "buy this template on the web first",
        )));
    }
    let body = json!({
        "workspace_id": ws, "version": a.version,
        "inputs": a.inputs.unwrap_or_else(|| json!({})), "agents": a.agents.unwrap_or_else(|| json!({}))
    });
    let md = install_md(&a.item, &preview);
    stage(
        b,
        "POST",
        &format!("/catalog/items/{}/install", a.item),
        body,
        format!("Install template {}", a.item),
        preview,
        md,
    )
    .await
}

async fn task_approve<B: Backend + ?Sized>(b: &B, a: TaskApproveArgs) -> ToolResult {
    let task = get(b, &format!("/factory/tasks/{}", enc(&a.task)), &[]).await?;
    let key = str_of(&task, "key")
        .filter(|k| !k.is_empty())
        .unwrap_or(&a.task)
        .to_string();
    let category = str_of(&task, "category").unwrap_or("approval");
    let md = Md::title(
        glyph(category),
        format!("{key} {}", str_of(&task, "title").unwrap_or("")),
    )
    .kv(&[
        ("state", str_of(&task, "state").unwrap_or("").to_string()),
        ("decision", a.decision.as_str().to_string()),
        ("note", a.note.clone().unwrap_or_default()),
    ]);
    let summary = format!(
        "{} {key}",
        if a.decision == Decision::Approve {
            "Approve"
        } else {
            "Reject"
        }
    );
    let body = json!({ "decision": a.decision.as_str(), "note": a.note });
    stage(
        b,
        "POST",
        &format!("/factory/tasks/{key}/approve"),
        body,
        summary,
        json!({ "task": task }),
        md,
    )
    .await
}
