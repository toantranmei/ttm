//! Factory read tools for people and agents (toolset `factory`): workflows, Automation rules, agents, skills,
//! templates and the inbox. Agents running a job only see [`JOB_TOOLS`] of them.
//!
//! Without a project argument they use the project of the run (`TTM_PROJECT`); without a workspace, the current one.

use crate::Backend;
use crate::backend::{BackendError, BackendResult};
use crate::common::{ToolFailure, ToolOutput, ToolResult, parse, parse_required, read_only};
use crate::md::{Md, badge, glyph, who};
use crate::out::{ItemsOut, ObjectOut};
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use ttm_api::automation::summarize;
use ttm_api::workflows::{Workflow, item_label};

pub const WORKFLOW_GET: &str = "factory-workflow-get";
pub const WORKFLOW_VALIDATE: &str = "factory-workflow-validate";
pub const WORKFLOW_SIMULATE: &str = "factory-workflow-simulate";
pub const WORKFLOW_UPGRADE_PREVIEW: &str = "factory-workflow-upgrade-preview";
pub const AUTOMATION_LIST: &str = "factory-automation-list";
pub const AUTOMATION_GET: &str = "factory-automation-get";
pub const AUTOMATION_TEST: &str = "factory-automation-test";
pub const AGENT_LIST: &str = "factory-agent-list";
pub const AGENT_GET: &str = "factory-agent-get";
pub const SKILL_LIST: &str = "factory-skill-list";
pub const SKILL_GET: &str = "factory-skill-get";
pub const TEMPLATE_SEARCH: &str = "factory-template-search";
pub const TEMPLATE_INSTALL_PREVIEW: &str = "factory-template-install-preview";
pub const INBOX: &str = "factory-inbox";

/// The read tools an agent running a job may use (besides the 8 job tools of `factory.rs`).
pub const JOB_TOOLS: [&str; 3] = [WORKFLOW_GET, SKILL_GET, AUTOMATION_LIST];

// ---- shared lookups -------------------------------------------------------------------------------------------

pub(crate) async fn get<B: Backend + ?Sized>(
    b: &B,
    path: &str,
    query: &[(&str, String)],
) -> BackendResult<Value> {
    let q: Vec<(String, String)> = query.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
    b.api("GET", path, &q, None).await
}

pub(crate) fn enc(segment: &str) -> String {
    segment
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `given`, else the current workspace of the signed-in user.
pub(crate) async fn workspace<B: Backend + ?Sized>(b: &B, given: Option<&str>) -> BackendResult<String> {
    if let Some(w) = given.filter(|w| !w.is_empty()) {
        return Ok(w.to_string());
    }
    b.get_self()
        .await?
        .workspace
        .map(|w| w.id)
        .ok_or_else(|| BackendError::new("no_workspace", "give `workspace_id` (no current workspace)"))
}

/// The project named by `given` (key or id), else the run's project; answered as the single project object.
pub(crate) async fn project<B: Backend + ?Sized>(b: &B, given: Option<&str>) -> BackendResult<Value> {
    let scope = b.factory_scope();
    let needle = given
        .filter(|p| !p.is_empty())
        .map(String::from)
        .or(scope.project)
        .ok_or_else(|| {
            BackendError::new("no_project", "give `project` (key or id; no project in this run)")
        })?;
    let list = get(b, "/factory/projects", &[]).await.unwrap_or(Value::Null);
    let id = list
        .as_array()
        .into_iter()
        .flatten()
        .find(|p| {
            p.get("id").and_then(Value::as_str) == Some(needle.as_str())
                || p.get("key")
                    .and_then(Value::as_str)
                    .is_some_and(|k| k.eq_ignore_ascii_case(&needle))
        })
        .and_then(|p| p.get("id").and_then(Value::as_str))
        .unwrap_or(&needle)
        .to_string();
    get(b, &format!("/factory/projects/{}", enc(&id)), &[]).await
}

pub(crate) fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// The workflow id: `workflow_id`, else the workflow of the project.
pub(crate) async fn workflow_id<B: Backend + ?Sized>(
    b: &B,
    workflow: Option<&str>,
    project_arg: Option<&str>,
) -> BackendResult<(String, Option<String>)> {
    if let Some(w) = workflow.filter(|w| !w.is_empty()) {
        return Ok((w.to_string(), None));
    }
    let p = project(b, project_arg).await?;
    let id = str_of(&p, "workflow_id")
        .or_else(|| str_of(&p, "workflowId"))
        .ok_or_else(|| BackendError::new("no_workflow", "this project has no v2 workflow"))?
        .to_string();
    Ok((id, str_of(&p, "id").map(String::from)))
}

fn items(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a.clone(),
        Value::Object(o) => o
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => vec![],
    }
}

fn list_out(list: Vec<Value>, text: String) -> ToolOutput {
    ToolOutput::with_text(json!({ "items": list }), text)
}

fn field(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

fn tier_of(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|m| m.get("tier"))
        .and_then(Value::as_str)
        .map(String::from)
}

// ---- arguments ------------------------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowArgs {
    /// Workflow id; default: the workflow of `project`.
    #[serde(default)]
    pub workflow_id: Option<String>,
    /// Project key or id; default: the project of this run.
    #[serde(default)]
    pub project: Option<String>,
    /// A published version number, or `draft`; default: the latest published version.
    #[serde(default)]
    pub version: Option<Value>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValidateArgs {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    /// Definition to check (`{states, transitions}`); default: the current draft.
    #[serde(default)]
    pub definition: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SimulateArgs {
    /// Task key (AUR-12).
    pub task: String,
    /// Status ids to move through, in order (e.g. `["qc", "done"]`).
    pub to: Vec<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    /// Version to simulate (number or `draft`); default: the project's version.
    #[serde(default)]
    pub version: Option<Value>,
    /// Roles of the actor (default: none, a plain user).
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpgradePreviewArgs {
    #[serde(default)]
    pub project: Option<String>,
    /// Target version; default: the latest.
    #[serde(default)]
    pub to: Option<i64>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectArg {
    /// Project key or id; default: the project of this run.
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleArg {
    /// Rule id, slug or name.
    pub rule: String,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleTestArgs {
    /// Rule id, slug or name.
    pub rule: String,
    /// Task key to test against.
    pub task: String,
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceArg {
    /// Workspace id; default: the current workspace.
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentArg {
    /// Agent handle (`dev`, `@dev`), name or id.
    pub agent: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillListArgs {
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Words in the name or description.
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    /// Include the built-in skills.
    #[serde(default)]
    pub builtin: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillArg {
    /// Skill slug or id.
    pub skill: String,
    /// Version; default: the version in effect (latest approved).
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemplateSearchArgs {
    /// `workflow`, `agent`, `skill`, `rule_pack`, `bundle` (several separated by commas).
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TemplateArg {
    /// Catalog item id (from factory-template-search).
    pub item: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub version: Option<i64>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InboxArgs {
    /// `factory`, `mention`, `comment`… (comma separated); default: all.
    #[serde(default)]
    pub kind: Option<String>,
    /// Only unread notifications.
    #[serde(default)]
    pub unread: bool,
    /// RFC 3339: only newer notifications.
    #[serde(default)]
    pub since: Option<String>,
}

// ---- tools ----------------------------------------------------------------------------------------------------

pub fn tools() -> Vec<Tool> {
    let t = |name: &'static str, description: &'static str, schema, title: &'static str, list: bool| {
        let tool = Tool::new(name, description, schema).annotate(read_only(title));
        if list {
            tool.with_output_schema::<ItemsOut>()
        } else {
            tool.with_output_schema::<ObjectOut>()
        }
    };
    vec![
        t(
            WORKFLOW_GET,
            "A project's workflow (or one by id): statuses with their category, role and auto-assign, and transitions with their guards. `version` picks a published version or the draft.",
            schema_for_type::<WorkflowArgs>(),
            "Get workflow",
            false,
        ),
        t(
            WORKFLOW_VALIDATE,
            "Check a workflow definition (default: the current draft) and list every issue with its field and code.",
            schema_for_type::<ValidateArgs>(),
            "Validate workflow",
            false,
        ),
        t(
            WORKFLOW_SIMULATE,
            "Simulate moving a task through statuses: for each step whether it is allowed, and if not the blocking rule; approval gates and Automation rules that would fire.",
            schema_for_type::<SimulateArgs>(),
            "Simulate workflow",
            false,
        ),
        t(
            WORKFLOW_UPGRADE_PREVIEW,
            "What upgrading a project to a newer workflow version changes: the diff, where each task would move and the project's Automation rules naming states the version removes or changes (they stop firing).",
            schema_for_type::<UpgradePreviewArgs>(),
            "Preview workflow upgrade",
            false,
        ),
        t(
            AUTOMATION_LIST,
            "Automation rules of the project (when … if … then …), with whether each is on. Agents read them to know what happens automatically.",
            schema_for_type::<ProjectArg>(),
            "List Automation rules",
            true,
        ),
        t(
            AUTOMATION_GET,
            "One Automation rule: trigger, conditions and actions.",
            schema_for_type::<RuleArg>(),
            "Get Automation rule",
            false,
        ),
        t(
            AUTOMATION_TEST,
            "Dry run of an Automation rule on a task: whether it matches and what each action would do. Changes nothing.",
            schema_for_type::<RuleTestArgs>(),
            "Test Automation rule",
            false,
        ),
        t(
            AGENT_LIST,
            "Agents of the workspace: handle, role, model, skills (pending review marked), MCP servers, status.",
            schema_for_type::<WorkspaceArg>(),
            "List agents",
            true,
        ),
        t(
            AGENT_GET,
            "One agent: profile, git identity, skills, MCP servers.",
            schema_for_type::<AgentArg>(),
            "Get agent",
            false,
        ),
        t(
            SKILL_LIST,
            "Skills of the workspace (and built-in ones with builtin=true): version in effect, versions waiting for review.",
            schema_for_type::<SkillListArgs>(),
            "List skills",
            true,
        ),
        t(
            SKILL_GET,
            "One skill: versions with review state and the SKILL.md of the version in effect (or the one asked).",
            schema_for_type::<SkillArg>(),
            "Get skill",
            false,
        ),
        t(
            TEMPLATE_SEARCH,
            "Search the public template library (workflows, agents, skills, rule packs, bundles).",
            schema_for_type::<TemplateSearchArgs>(),
            "Search templates",
            true,
        ),
        t(
            TEMPLATE_INSTALL_PREVIEW,
            "What installing a template would create in a workspace (statuses, rules, agents, skills), whether it runs code and whether it must be bought first.",
            schema_for_type::<TemplateArg>(),
            "Preview template install",
            false,
        ),
        t(
            INBOX,
            "The user's notifications (approvals, change requests, skill reviews, run results…), newest first.",
            schema_for_type::<InboxArgs>(),
            "Inbox",
            true,
        ),
    ]
}

pub async fn call<B: Backend + ?Sized>(
    b: &B,
    name: &str,
    args: Option<Map<String, Value>>,
) -> Option<ToolResult> {
    Some(match name {
        WORKFLOW_GET => match parse(args) {
            Ok(a) => workflow_get(b, a).await,
            Err(e) => Err(e),
        },
        WORKFLOW_VALIDATE => match parse(args) {
            Ok(a) => workflow_validate(b, a).await,
            Err(e) => Err(e),
        },
        WORKFLOW_SIMULATE => match parse_required(args) {
            Ok(a) => workflow_simulate(b, a).await,
            Err(e) => Err(e),
        },
        WORKFLOW_UPGRADE_PREVIEW => match parse(args) {
            Ok(a) => upgrade_preview(b, a).await,
            Err(e) => Err(e),
        },
        AUTOMATION_LIST => match parse(args) {
            Ok(a) => automation_list(b, a).await,
            Err(e) => Err(e),
        },
        AUTOMATION_GET => match parse_required::<RuleArg>(args) {
            Ok(a) => automation_get(b, a).await,
            Err(e) => Err(e),
        },
        AUTOMATION_TEST => match parse_required(args) {
            Ok(a) => automation_test(b, a).await,
            Err(e) => Err(e),
        },
        AGENT_LIST => match parse(args) {
            Ok(a) => agent_list(b, a).await,
            Err(e) => Err(e),
        },
        AGENT_GET => match parse_required(args) {
            Ok(a) => agent_get(b, a).await,
            Err(e) => Err(e),
        },
        SKILL_LIST => match parse(args) {
            Ok(a) => skill_list(b, a).await,
            Err(e) => Err(e),
        },
        SKILL_GET => match parse_required(args) {
            Ok(a) => skill_get(b, a).await,
            Err(e) => Err(e),
        },
        TEMPLATE_SEARCH => match parse(args) {
            Ok(a) => template_search(b, a).await,
            Err(e) => Err(e),
        },
        TEMPLATE_INSTALL_PREVIEW => match parse_required(args) {
            Ok(a) => install_preview(b, a).await,
            Err(e) => Err(e),
        },
        INBOX => match parse(args) {
            Ok(a) => inbox(b, a).await,
            Err(e) => Err(e),
        },
        _ => return None,
    })
}

/// `version` as a path suffix: a number → `/versions/N`, `draft` → the draft of the workflow.
fn version_of(v: &Option<Value>) -> Result<Option<VersionPick>, ToolFailure> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_i64()
            .map(|n| Some(VersionPick::Number(n)))
            .ok_or_else(|| ToolFailure::InvalidArgs("version must be a whole number or `draft`".into())),
        Some(Value::String(s)) if s == "draft" => Ok(Some(VersionPick::Draft)),
        Some(Value::String(s)) => s
            .parse::<i64>()
            .map(|n| Some(VersionPick::Number(n)))
            .map_err(|_| ToolFailure::InvalidArgs("version must be a whole number or `draft`".into())),
        _ => Err(ToolFailure::InvalidArgs(
            "version must be a whole number or `draft`".into(),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VersionPick {
    Number(i64),
    Draft,
}

/// Markdown of a workflow definition: statuses and transitions.
pub fn workflow_md(name: &str, label: &str, definition: &Value) -> Md {
    let states = Workflow::states(definition);
    let transitions = Workflow::transitions(definition);
    let state_rows: Vec<Vec<String>> = states
        .iter()
        .map(|s| {
            let cat = s.category.clone().unwrap_or_default();
            let auto = match &s.auto_assign {
                Some(Value::Bool(true)) => "🤖".to_string(),
                Some(Value::Bool(false)) | None => "👤".to_string(),
                Some(other) => summarize(other),
            };
            vec![
                format!("{} {}", glyph(&cat), s.name),
                s.id.clone(),
                cat,
                s.role.clone().unwrap_or_default(),
                auto,
                s.wip.map(|w| w.to_string()).unwrap_or_default(),
            ]
        })
        .collect();
    let transition_rows: Vec<Vec<String>> = transitions
        .iter()
        .map(|t| {
            let from = match &t.from {
                Value::Array(a) => a.iter().map(item_label).collect::<Vec<_>>().join(", "),
                Value::Null => "*".into(),
                other => item_label(other),
            };
            let rules = match &t.rules {
                Some(Value::Array(a)) if !a.is_empty() => format!("⚡ {}", a.len()),
                Some(Value::Number(n)) => format!("⚡ {n}"),
                _ => String::new(),
            };
            vec![
                format!("{from} → {}", t.to),
                t.guard.as_ref().map(summarize).unwrap_or_default(),
                rules,
            ]
        })
        .collect();
    Md::title("○", format!("Workflow {name} · {label}"))
        .table(
            &["Status", "Id", "Category", "Role", "Assign", "WIP"],
            &state_rows,
        )
        .table(&["Transition", "Guard", "Rules"], &transition_rows)
}

async fn workflow_get<B: Backend + ?Sized>(b: &B, args: WorkflowArgs) -> ToolResult {
    let pick = version_of(&args.version)?;
    let (id, _) = workflow_id(b, args.workflow_id.as_deref(), args.project.as_deref()).await?;
    let (mut value, label, definition) = match pick {
        Some(VersionPick::Number(n)) => {
            let v = get(b, &format!("/factory/workflows/{}/versions/{n}", enc(&id)), &[]).await?;
            let d = v.get("definition").cloned().unwrap_or(Value::Null);
            (v, format!("v{n}"), d)
        }
        _ => {
            let v = get(b, &format!("/factory/workflows/{}", enc(&id)), &[]).await?;
            let draft = pick == Some(VersionPick::Draft);
            let d = if draft {
                v.pointer("/draft/definition").cloned().ok_or_else(|| {
                    ToolFailure::Backend(BackendError::new("no_draft", "this workflow has no draft"))
                })?
            } else {
                v.get("definition").cloned().unwrap_or(Value::Null)
            };
            let label = if draft {
                "draft".to_string()
            } else {
                format!(
                    "v{}{}",
                    v.get("version")
                        .or_else(|| v.get("latest_version"))
                        .and_then(Value::as_i64)
                        .unwrap_or(0),
                    if v.get("has_draft").and_then(Value::as_bool).unwrap_or(false) {
                        " (draft pending)"
                    } else {
                        ""
                    }
                )
            };
            (v, label, d)
        }
    };
    let name = field(&value, "name");
    let md =
        workflow_md(&name, &label, &definition).next(&[(WORKFLOW_SIMULATE, "check a move before doing it")]);
    if let Value::Object(o) = &mut value {
        o.insert("shown".into(), json!(label));
    }
    Ok(ToolOutput::with_text(value, md.build()))
}

async fn workflow_validate<B: Backend + ?Sized>(b: &B, args: ValidateArgs) -> ToolResult {
    let (id, _) = workflow_id(b, args.workflow_id.as_deref(), args.project.as_deref()).await?;
    let definition = match args.definition {
        Some(d) => d,
        None => {
            let w = get(b, &format!("/factory/workflows/{}", enc(&id)), &[]).await?;
            w.pointer("/draft/definition")
                .or_else(|| w.get("definition"))
                .cloned()
                .unwrap_or(Value::Null)
        }
    };
    let result = b
        .api(
            "POST",
            &format!("/factory/workflows/{}/validate", enc(&id)),
            &[],
            Some(json!({ "definition": definition })),
        )
        .await?;
    let issues = result
        .get("issues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = issues
        .iter()
        .map(|i| vec![field(i, "code"), field(i, "field"), field(i, "message")])
        .collect();
    let md = if issues.is_empty() {
        Md::title("✓", "The workflow is valid")
    } else {
        Md::title("✕", format!("{} issue(s)", issues.len())).table(&["Code", "Field", "Message"], &rows)
    };
    Ok(ToolOutput::with_text(
        json!({ "ok": issues.is_empty(), "issues": issues }),
        md.build(),
    ))
}

async fn workflow_simulate<B: Backend + ?Sized>(b: &B, args: SimulateArgs) -> ToolResult {
    if args.to.is_empty() {
        return Err(ToolFailure::InvalidArgs("`to` is empty".into()));
    }
    let pick = version_of(&args.version)?;
    let p = project(b, args.project.as_deref()).await?;
    let project_id = str_of(&p, "id").unwrap_or_default().to_string();
    let id = match args.workflow_id {
        Some(w) => w,
        None => str_of(&p, "workflow_id")
            .or_else(|| str_of(&p, "workflowId"))
            .ok_or_else(|| {
                ToolFailure::Backend(BackendError::new(
                    "no_workflow",
                    "this project has no v2 workflow",
                ))
            })?
            .to_string(),
    };
    let mut body = json!({
        "project_id": project_id, "task": args.task, "steps": args.to,
        "actor": {"kind": "user", "roles": args.roles}
    });
    match pick {
        Some(VersionPick::Number(n)) => body["version"] = json!(n),
        Some(VersionPick::Draft) => {
            let w = get(b, &format!("/factory/workflows/{}", enc(&id)), &[]).await?;
            body["definition"] = w.pointer("/draft/definition").cloned().unwrap_or(Value::Null);
        }
        None => {}
    }
    let sim = b
        .api(
            "POST",
            &format!("/factory/workflows/{}/simulate", enc(&id)),
            &[],
            Some(body),
        )
        .await?;
    let steps = sim
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = steps
        .iter()
        .map(|s| {
            let allowed = s.get("allowed").and_then(Value::as_bool).unwrap_or(false);
            vec![
                format!(
                    "{} {} → {}",
                    if allowed { "✓" } else { "✕" },
                    field(s, "from"),
                    field(s, "to")
                ),
                s.get("blocked")
                    .map(|b| format!("{}: {}", field(b, "code"), field(b, "message")))
                    .unwrap_or_default(),
                if s.get("approval_gate").is_some_and(|g| !g.is_null()) {
                    "◐ approval".into()
                } else {
                    String::new()
                },
                s.get("rules")
                    .and_then(Value::as_array)
                    .map(|r| r.len().to_string())
                    .unwrap_or_default(),
            ]
        })
        .collect();
    let all = steps
        .iter()
        .all(|s| s.get("allowed").and_then(Value::as_bool).unwrap_or(false));
    let md = Md::title(
        if all { "✓" } else { "✕" },
        format!("Simulation ends in {}", field(&sim, "end_state")),
    )
    .table(&["Step", "Blocked by", "Gate", "Rules"], &rows);
    Ok(ToolOutput::with_text(sim, md.build()))
}

async fn upgrade_preview<B: Backend + ?Sized>(b: &B, args: UpgradePreviewArgs) -> ToolResult {
    let p = project(b, args.project.as_deref()).await?;
    let id = str_of(&p, "id").unwrap_or_default().to_string();
    let q: Vec<(&str, String)> = args.to.map(|t| vec![("to", t.to_string())]).unwrap_or_default();
    let preview = get(b, &format!("/factory/projects/{}/workflow-upgrade", enc(&id)), &q).await?;
    Ok(ToolOutput::with_text(
        preview.clone(),
        upgrade_md(&preview).build(),
    ))
}

/// Diff of a workflow (`{states, transitions, rules: {added, removed, changed}}`) as a table.
pub fn diff_rows(diff: &Value) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    for part in ["states", "transitions", "rules"] {
        for (kind, sign) in [("added", "+"), ("removed", "-"), ("changed", "~")] {
            for item in diff
                .pointer(&format!("/{part}/{kind}"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                rows.push(vec![format!("{sign} {part}"), item_label(item)]);
            }
        }
    }
    rows
}

/// The Automation rules a new workflow version leaves behind (`affected_rules` of a diff or an upgrade preview).
pub fn affected_rules(v: &Value) -> Vec<Value> {
    v.get("affected_rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// Rules are not versioned with the workflow: a rule naming a state the new version removes or changes stops firing.
/// Adds the warning and the table (nothing when none).
pub fn affected_rules_md(md: Md, rules: &[Value]) -> Md {
    if rules.is_empty() {
        return md;
    }
    let rows: Vec<Vec<String>> = rules
        .iter()
        .filter_map(|r| serde_json::from_value::<ttm_api::workflows::AffectedRule>(r.clone()).ok())
        .map(|r| vec![r.label(), r.slug.clone(), r.field.clone(), r.why()])
        .collect();
    md.text(format!(
        "⚠ {} Automation rule(s) name states this version removes or changes; they stop firing until they are fixed (rules are not versioned with the workflow). Show them to the user: confirming acknowledges them.",
        rows.len()
    ))
    .table(&["Rule", "Slug", "Where", "Why"], &rows)
}

pub fn upgrade_md(preview: &Value) -> Md {
    let remap: Vec<Vec<String>> = preview
        .get("remap")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|r| {
            vec![
                field(r, "key"),
                field(r, "title"),
                format!("{} → {}", field(r, "from_state"), field(r, "to_state")),
                field(r, "reason"),
            ]
        })
        .collect();
    let md = Md::title(
        "◐",
        format!(
            "Upgrade {} v{} → v{}",
            field(preview, "name"),
            field(preview, "from_version"),
            field(preview, "to_version")
        ),
    )
    .table(
        &["Change", "Item"],
        &diff_rows(preview.get("diff").unwrap_or(&Value::Null)),
    )
    .table(&["Task", "Title", "Moves", "Why"], &remap);
    affected_rules_md(md, &affected_rules(preview))
}

async fn rules_of<B: Backend + ?Sized>(b: &B, project_arg: Option<&str>) -> BackendResult<Vec<Value>> {
    let p = project(b, project_arg).await?;
    let id = str_of(&p, "id").unwrap_or_default().to_string();
    Ok(items(
        &get(b, "/factory/automation/rules", &[("project_id", id)]).await?,
    ))
}

pub(crate) async fn find_rule<B: Backend + ?Sized>(
    b: &B,
    rule: &str,
    project_arg: Option<&str>,
) -> BackendResult<Value> {
    if let Ok(v) = get(b, &format!("/factory/automation/rules/{}", enc(rule)), &[]).await {
        return Ok(v);
    }
    rules_of(b, project_arg)
        .await?
        .into_iter()
        .find(|r| {
            str_of(r, "slug") == Some(rule) || str_of(r, "name").is_some_and(|n| n.eq_ignore_ascii_case(rule))
        })
        .ok_or_else(|| BackendError::new("not_found", format!("no Automation rule `{rule}`")))
}

pub fn rule_md(rule: &Value) -> Md {
    let enabled = rule.get("enabled").and_then(Value::as_bool).unwrap_or(false);
    let conditions = rule
        .pointer("/conditions/filters")
        .and_then(Value::as_array)
        .map(|f| f.iter().map(summarize).collect::<Vec<_>>().join("; "))
        .unwrap_or_else(|| summarize(rule.get("conditions").unwrap_or(&Value::Null)));
    let actions = rule
        .get("actions")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(summarize).collect::<Vec<_>>().join("; "))
        .unwrap_or_default();
    Md::title(
        if enabled { "●" } else { "⏸" },
        format!("Rule {}", field(rule, "name")),
    )
    .kv(&[
        ("when", summarize(rule.get("trigger").unwrap_or(&Value::Null))),
        ("if", conditions),
        ("then", actions),
        ("enabled", enabled.to_string()),
        ("id", field(rule, "id")),
    ])
}

async fn automation_list<B: Backend + ?Sized>(b: &B, args: ProjectArg) -> ToolResult {
    let rules = rules_of(b, args.project.as_deref()).await?;
    let rows: Vec<Vec<String>> = rules
        .iter()
        .map(|r| {
            vec![
                format!(
                    "{} {}",
                    if r.get("enabled").and_then(Value::as_bool).unwrap_or(false) {
                        "●"
                    } else {
                        "⏸"
                    },
                    field(r, "name")
                ),
                summarize(r.get("trigger").unwrap_or(&Value::Null)),
                field(r, "runs_7d"),
                field(r, "id"),
            ]
        })
        .collect();
    let text = Md::title("○", format!("{} Automation rule(s)", rules.len()))
        .table(&["Rule", "When", "Runs (7 days)", "Id"], &rows)
        .next(&[(AUTOMATION_GET, "details of one")])
        .build();
    Ok(list_out(rules, text))
}

async fn automation_get<B: Backend + ?Sized>(b: &B, args: RuleArg) -> ToolResult {
    let rule = find_rule(b, &args.rule, args.project.as_deref()).await?;
    let md = rule_md(&rule).next(&[(AUTOMATION_TEST, "dry run on a task")]);
    Ok(ToolOutput::with_text(rule, md.build()))
}

async fn automation_test<B: Backend + ?Sized>(b: &B, args: RuleTestArgs) -> ToolResult {
    let rule = find_rule(b, &args.rule, args.project.as_deref()).await?;
    let p = project(b, args.project.as_deref()).await?;
    let body = json!({ "rule_id": field(&rule, "id"), "project_id": str_of(&p, "id"), "task": args.task });
    let dry = b
        .api("POST", "/factory/automation/rules/dry-run", &[], Some(body))
        .await?;
    let matched = dry.get("matched").and_then(Value::as_bool).unwrap_or(false);
    let rows: Vec<Vec<String>> = dry
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|a| {
            let runs = a
                .get("will_run")
                .or_else(|| a.get("ok"))
                .and_then(Value::as_bool)
                .unwrap_or(matched);
            vec![
                format!("{} {}", if runs { "✓" } else { "✕" }, summarize(a)),
                field(a, "reason"),
            ]
        })
        .collect();
    let md = Md::title(
        if matched { "✓" } else { "○" },
        format!(
            "Rule {} {} on {}",
            field(&rule, "name"),
            if matched { "matches" } else { "does not match" },
            args.task
        ),
    )
    .table(&["Action", "Why"], &rows);
    Ok(ToolOutput::with_text(dry, md.build()))
}

pub(crate) async fn agents_of<B: Backend + ?Sized>(b: &B, ws: &str) -> BackendResult<Vec<Value>> {
    Ok(items(
        &get(b, &format!("/workspaces/{}/agents", enc(ws)), &[]).await?,
    ))
}

pub(crate) async fn find_agent<B: Backend + ?Sized>(b: &B, ws: &str, needle: &str) -> BackendResult<Value> {
    let n = needle.trim().trim_start_matches('@');
    agents_of(b, ws)
        .await?
        .into_iter()
        .find(|a| {
            str_of(a, "id") == Some(n)
                || str_of(a, "handle").is_some_and(|h| h.eq_ignore_ascii_case(n))
                || str_of(a, "name").is_some_and(|h| h.eq_ignore_ascii_case(n))
        })
        .ok_or_else(|| BackendError::new("not_found", format!("no agent `{needle}`")))
}

fn agent_skills(a: &Value) -> String {
    a.get("skills")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|s| {
            let pending = str_of(s, "state") == Some("pending");
            format!("{}{}", field(s, "slug"), if pending { " ◐" } else { "" })
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn agent_mcp(a: &Value) -> String {
    a.pointer("/profile/mcp")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|m| field(m, "server"))
        .collect::<Vec<_>>()
        .join(", ")
}

async fn agent_list<B: Backend + ?Sized>(b: &B, args: WorkspaceArg) -> ToolResult {
    let ws = workspace(b, args.workspace_id.as_deref()).await?;
    let agents = agents_of(b, &ws).await?;
    let rows: Vec<Vec<String>> = agents
        .iter()
        .map(|a| {
            vec![
                who(
                    "agent",
                    &field(a, "name"),
                    tier_of(a, "owner_membership").as_deref(),
                ),
                str_of(a, "handle").map(|h| format!("@{h}")).unwrap_or_default(),
                field(a, "role"),
                field(a, "model"),
                agent_skills(a),
                agent_mcp(a),
                field(a, "status"),
            ]
        })
        .collect();
    let text = Md::title("🤖", format!("{} agent(s)", agents.len()))
        .table(
            &["Agent", "Handle", "Role", "Model", "Skills", "MCP", "Status"],
            &rows,
        )
        .next(&[(AGENT_GET, "one agent's profile")])
        .build();
    Ok(list_out(agents, text))
}

async fn agent_get<B: Backend + ?Sized>(b: &B, args: AgentArg) -> ToolResult {
    let ws = workspace(b, args.workspace_id.as_deref()).await?;
    let a = find_agent(b, &ws, &args.agent).await?;
    let md = Md::title(
        "🤖",
        format!(
            "{}{}",
            field(&a, "name"),
            badge(tier_of(&a, "owner_membership").as_deref())
        ),
    )
    .kv(&[
        (
            "handle",
            str_of(&a, "handle").map(|h| format!("@{h}")).unwrap_or_default(),
        ),
        ("role", field(&a, "role")),
        ("model", field(&a, "model")),
        ("status", field(&a, "status")),
        ("bio", field(&a, "bio")),
        (
            "git",
            format!("{} <{}>", field(&a, "git_name"), field(&a, "git_email")),
        ),
        ("skills", agent_skills(&a)),
        ("mcp", agent_mcp(&a)),
        ("id", field(&a, "id")),
    ]);
    Ok(ToolOutput::with_text(a, md.build()))
}

async fn skill_list<B: Backend + ?Sized>(b: &B, args: SkillListArgs) -> ToolResult {
    let ws = workspace(b, args.workspace_id.as_deref()).await?;
    let mut q = vec![("workspace_id", ws)];
    if let Some(v) = args.query {
        q.push(("q", v));
    }
    if let Some(v) = args.tag {
        q.push(("tag", v));
    }
    if args.builtin {
        q.push(("builtin", "1".into()));
    }
    let skills = items(&get(b, "/factory/skills", &q).await?);
    let rows: Vec<Vec<String>> = skills
        .iter()
        .map(|s| {
            let pending = match s.get("pending_versions") {
                Some(Value::Array(a)) => a.len(),
                Some(Value::Number(n)) => n.as_u64().unwrap_or(0) as usize,
                _ => 0,
            };
            vec![
                field(s, "slug"),
                field(s, "name"),
                field(s, "current_version"),
                if pending > 0 {
                    format!("◐ {pending}")
                } else {
                    String::new()
                },
                if s.get("builtin").and_then(Value::as_bool).unwrap_or(false) {
                    "built-in".into()
                } else {
                    String::new()
                },
            ]
        })
        .collect();
    let text = Md::title("○", format!("{} skill(s)", skills.len()))
        .table(&["Slug", "Name", "Version", "Pending", "Source"], &rows)
        .next(&[(SKILL_GET, "read a skill")])
        .build();
    Ok(list_out(skills, text))
}

pub(crate) async fn find_skill<B: Backend + ?Sized>(
    b: &B,
    needle: &str,
    ws: Option<&str>,
) -> BackendResult<Value> {
    if let Ok(s) = get(b, &format!("/factory/skills/{}", enc(needle)), &[]).await {
        return Ok(s);
    }
    let ws = workspace(b, ws).await?;
    let found = items(
        &get(
            b,
            "/factory/skills",
            &[("workspace_id", ws), ("builtin", "1".into())],
        )
        .await?,
    )
    .into_iter()
    .find(|s| str_of(s, "slug") == Some(needle))
    .ok_or_else(|| BackendError::new("not_found", format!("no skill `{needle}`")))?;
    let id = field(&found, "id");
    get(b, &format!("/factory/skills/{}", enc(&id)), &[]).await
}

async fn skill_get<B: Backend + ?Sized>(b: &B, args: SkillArg) -> ToolResult {
    let mut skill = find_skill(b, &args.skill, args.workspace_id.as_deref()).await?;
    let id = field(&skill, "id");
    let version = args
        .version
        .or_else(|| skill.get("current_version").and_then(Value::as_i64));
    let skill_md = match version {
        Some(v) => b
            .api(
                "GET",
                &format!("/factory/skills/{}/versions/{v}/files/SKILL.md", enc(&id)),
                &[],
                None,
            )
            .await
            .ok()
            .map(|v| match v {
                Value::String(s) => s,
                other => other
                    .get("text")
                    .or_else(|| other.get("content"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            }),
        None => None,
    };
    let versions: Vec<Vec<String>> = skill
        .get("versions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|v| {
            let status = field(v, "review_status");
            let mark = match status.as_str() {
                "approved" | "exempt" => "✓",
                "pending" => "◐",
                "rejected" => "✕",
                _ => "○",
            };
            vec![
                format!("v{}", field(v, "version")),
                format!("{mark} {status}"),
                field(v, "note"),
            ]
        })
        .collect();
    let mut md = Md::title(
        "○",
        format!(
            "Skill {} · v{}",
            field(&skill, "slug"),
            version.map(|v| v.to_string()).unwrap_or("-".into())
        ),
    )
    .kv(&[
        ("name", field(&skill, "name")),
        ("description", field(&skill, "description")),
    ])
    .table(&["Version", "Review", "Note"], &versions);
    if let Some(text) = &skill_md {
        md = md.text(format!("SKILL.md:\n\n{text}"));
    }
    if let Value::Object(o) = &mut skill {
        o.insert("skill_md".into(), json!(skill_md));
        o.insert("shown_version".into(), json!(version));
    }
    Ok(ToolOutput::with_text(skill, md.build()))
}

async fn template_search<B: Backend + ?Sized>(b: &B, args: TemplateSearchArgs) -> ToolResult {
    let mut q = Vec::new();
    if let Some(k) = args.kind {
        q.push(("kind", k));
    }
    if let Some(v) = args.query {
        q.push(("q", v));
    }
    let list = items(&get(b, "/catalog", &q).await?);
    let rows: Vec<Vec<String>> = list
        .iter()
        .map(|t| {
            vec![
                field(t, "name"),
                field(t, "kind"),
                field(t, "tagline"),
                field(t, "id"),
            ]
        })
        .collect();
    let text = Md::title("○", format!("{} template(s)", list.len()))
        .table(&["Name", "Kind", "Tagline", "Id"], &rows)
        .next(&[(TEMPLATE_INSTALL_PREVIEW, "see what one would create")])
        .build();
    Ok(list_out(list, text))
}

pub fn install_md(item: &str, p: &Value) -> Md {
    let mut rows = Vec::new();
    if let Some(w) = p.pointer("/creates/workflow").filter(|w| !w.is_null()) {
        rows.push(vec!["workflow".to_string(), item_label(w)]);
    }
    for kind in ["rules", "agents", "skills"] {
        for v in p
            .pointer(&format!("/creates/{kind}"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            rows.push(vec![kind.trim_end_matches('s').to_string(), item_label(v)]);
        }
    }
    Md::title("◐", format!("Installing {item} creates {} item(s)", rows.len()))
        .table(&["Kind", "Name"], &rows)
        .kv(&[
            (
                "runs code",
                if p.get("runs_code").and_then(Value::as_bool).unwrap_or(false) {
                    "yes (skills with scripts)".into()
                } else {
                    String::new()
                },
            ),
            (
                "purchase",
                if p.get("requires_purchase")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "required first".into()
                } else {
                    String::new()
                },
            ),
        ])
}

async fn install_preview<B: Backend + ?Sized>(b: &B, args: TemplateArg) -> ToolResult {
    let ws = workspace(b, args.workspace_id.as_deref()).await?;
    let p = b
        .api(
            "POST",
            &format!("/catalog/items/{}/install-preview", enc(&args.item)),
            &[],
            Some(json!({ "workspace_id": ws, "version": args.version })),
        )
        .await?;
    Ok(ToolOutput::with_text(
        p.clone(),
        install_md(&args.item, &p).build(),
    ))
}

async fn inbox<B: Backend + ?Sized>(b: &B, args: InboxArgs) -> ToolResult {
    let mut q = Vec::new();
    if let Some(k) = args.kind {
        q.push(("kind", k));
    }
    if args.unread {
        q.push(("unread", "1".into()));
    }
    if let Some(s) = args.since {
        q.push(("since", s));
    }
    let list = items(&get(b, "/notifications", &q).await?);
    let rows: Vec<Vec<String>> = list
        .iter()
        .map(|n| {
            let unread = n.get("read_at").is_none_or(Value::is_null);
            vec![
                format!(
                    "{} {}",
                    if unread { "●" } else { "○" },
                    n.pointer("/data/reason")
                        .and_then(Value::as_str)
                        .unwrap_or(&field(n, "kind"))
                ),
                n.get("text")
                    .or_else(|| n.get("title"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                field(n, "created_at"),
                field(n, "url"),
            ]
        })
        .collect();
    let text = Md::title("○", format!("{} notification(s)", list.len()))
        .table(&["What", "Text", "When", "Link"], &rows)
        .build();
    Ok(list_out(list, text))
}
