//! Factory tools, used by an agent while it works on a task: `factory-get-task`, `factory-get-context`,
//! `factory-report-progress`, `factory-request-approval`, `factory-create-subtasks`, `factory-attach-artifact`,
//! `factory-get-rules`, `factory-renew-lease`. Without arguments they act on the current run and task of the job.

use crate::Backend;
use crate::common::{
    ToolFailure, ToolOutput, ToolResult, parse, parse_required, read_only, to_value, writes,
};
use crate::md::{Md, glyph, who};
use crate::out::{ObjectOut, object};
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};

pub const GET_TASK: &str = "factory-get-task";
pub const GET_CONTEXT: &str = "factory-get-context";
pub const REPORT_PROGRESS: &str = "factory-report-progress";
pub const REQUEST_APPROVAL: &str = "factory-request-approval";
pub const CREATE_SUBTASKS: &str = "factory-create-subtasks";
pub const ATTACH_ARTIFACT: &str = "factory-attach-artifact";
pub const GET_RULES: &str = "factory-get-rules";
pub const RENEW_LEASE: &str = "factory-renew-lease";

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskArg {
    /// Task key (AUR-12) or id; default: the task of this run.
    #[serde(default)]
    pub task: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgressArgs {
    /// What was done or is being done, one or two sentences.
    pub message: String,
    /// 0-100.
    #[serde(default)]
    pub percent: Option<u8>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApprovalArgs {
    /// What you need approved and why.
    pub reason: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubtaskArg {
    pub title: String,
    /// Work role: PM, BA, UI/UX, DEV, GUARDIAN, QC or a workspace role.
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Whole number (story points or hours, as the project uses).
    #[serde(default)]
    pub estimate: Option<i32>,
    /// `feature`, `bug`, `chore`… as the project uses.
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    /// Keys of existing tasks, or `#<n>` for the n-th (0-based) subtask of this call.
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubtasksArgs {
    /// Parent task key; default: the task of this run.
    #[serde(default)]
    pub parent: Option<String>,
    /// At most 10 per run; depth at most 2 below the root task.
    pub tasks: Vec<SubtaskArg>,
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Pull request (`url`): recorded as the run's PR.
    Pr,
    /// Test run: set `passed`.
    TestResult,
    /// CI result: set `passed`.
    Ci,
    Screenshot,
    Link,
    Note,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactArgs {
    pub kind: ArtifactKind,
    #[serde(default)]
    pub url: Option<String>,
    /// Text content (test output, note).
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    /// For `test_result` and `ci`: whether it passed.
    #[serde(default)]
    pub passed: Option<bool>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RulesArgs {
    /// Project key or id; default: the project of this run.
    #[serde(default)]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LeaseArgs {
    /// Glob paths of the code you are changing, e.g. `src/auth/**`.
    pub paths: Vec<String>,
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool::new(GET_TASK, "The Factory task you work on (or another by key): title, state, role, assignee, parent.", schema_for_type::<TaskArg>())
            .with_output_schema::<ObjectOut>()
            .annotate(read_only("Get task")),
        Tool::new(
            GET_CONTEXT,
            "Context prepared by the server for this task: description, acceptance criteria, related Notee pages, sibling \
             tasks, suggested files. Read it before you start.",
            schema_for_type::<TaskArg>(),
        )
        .with_output_schema::<ObjectOut>()
            .annotate(read_only("Get context")),
        Tool::new(REPORT_PROGRESS, "Report progress on the Execution Map (shown live to the team).", schema_for_type::<ProgressArgs>())
            .with_output_schema::<ObjectOut>()
            .annotate(writes("Report progress", false)),
        Tool::new(
            REQUEST_APPROVAL,
            "Ask a person (or the role the workflow names) to approve something before you continue.",
            schema_for_type::<ApprovalArgs>(),
        )
        .with_output_schema::<ObjectOut>()
            .annotate(writes("Request approval", false)),
        Tool::new(
            CREATE_SUBTASKS,
            "Split the task into subtasks. Limits (defaults; the workflow may change them): 10 per run, 30 per root task, \
             depth 2. Over a limit the tool fails with code `conflict` and rule `split-limit`; its params give the limits \
             and current counts (max_depth, per_run, per_root, depth, run_count, root_count), so split again with fewer \
             or shallower subtasks.",
            schema_for_type::<SubtasksArgs>(),
        )
        .with_output_schema::<ObjectOut>()
            .annotate(writes("Create subtasks", false)),
        Tool::new(ATTACH_ARTIFACT, "Attach a result to the run: pull request, test output, screenshot, link or note.", schema_for_type::<ArtifactArgs>())
            .with_output_schema::<ObjectOut>()
            .annotate(writes("Attach artifact", false)),
        Tool::new(GET_RULES, "The project's rules for agents (branching, commits, tests before done).", schema_for_type::<RulesArgs>())
            .with_output_schema::<ObjectOut>()
            .annotate(read_only("Get rules")),
        Tool::new(
            RENEW_LEASE,
            "Claim or extend a lease on code paths (globs) so other agents do not edit them at the same time. A lease held \
             by another run comes back as granted=false with held_by.",
            schema_for_type::<LeaseArgs>(),
        )
        .with_output_schema::<ObjectOut>()
            .annotate(writes("Renew lease", false)),
    ]
}

pub async fn call<B: Backend + ?Sized>(
    backend: &B,
    name: &str,
    args: Option<Map<String, Value>>,
) -> Option<ToolResult> {
    Some(match name {
        GET_TASK => get_task(backend, args).await,
        GET_CONTEXT => get_context(backend, args).await,
        REPORT_PROGRESS => progress(backend, args).await,
        REQUEST_APPROVAL => approval(backend, args).await,
        CREATE_SUBTASKS => subtasks(backend, args).await,
        ATTACH_ARTIFACT => artifact(backend, args).await,
        GET_RULES => rules(backend, args).await,
        RENEW_LEASE => lease(backend, args).await,
        _ => return None,
    })
}

async fn get_task<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: TaskArg = parse(args)?;
    let task = backend.factory_get_task(args.task.as_deref()).await?;
    Ok(ToolOutput::with_text(to_value(&task), task_md(&task)))
}

async fn get_context<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: TaskArg = parse(args)?;
    let context = backend.factory_get_context(args.task.as_deref()).await?;
    match context.get("markdown").and_then(Value::as_str) {
        Some(md) => {
            let text = md.to_string();
            Ok(ToolOutput::with_text(context, text))
        }
        None => {
            let text = Md::title("○", "Task context")
                .text(format!(
                    "```json\n{}\n```",
                    serde_json::to_string_pretty(&context).unwrap_or_default()
                ))
                .build();
            Ok(ToolOutput::with_text(object(context), text))
        }
    }
}

async fn progress<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: ProgressArgs = parse_required(args)?;
    if args.percent.is_some_and(|p| p > 100) {
        return Err(ToolFailure::InvalidArgs("percent must be 0-100".into()));
    }
    backend
        .factory_report_progress(&args.message, args.percent)
        .await?;
    let text = Md::title("✓", "Progress reported")
        .kv(&[(
            "percent",
            args.percent.map(|p| format!("{p}%")).unwrap_or_default(),
        )])
        .build();
    Ok(ToolOutput::with_text(json!({ "ok": true }), text))
}

async fn approval<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: ApprovalArgs = parse_required(args)?;
    let result = backend.factory_request_approval(&args.reason).await?;
    let text = Md::title("◐", "Approval requested")
        .text("Wait for the decision before doing what needs approval.")
        .next(&[(GET_TASK, "check the decision later")])
        .build();
    Ok(ToolOutput::with_text(object(result), text))
}

async fn subtasks<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: SubtasksArgs = parse_required(args)?;
    if args.tasks.is_empty() {
        return Err(ToolFailure::InvalidArgs("tasks is empty".into()));
    }
    let tasks: Vec<Value> = args.tasks.into_iter().map(subtask_body).collect();
    let result = backend
        .factory_create_subtasks(args.parent.as_deref(), &tasks)
        .await?;
    let created: Vec<Vec<String>> = result
        .get("tasks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|t| {
            vec![
                t.get("key")
                    .or_else(|| t.get("id"))
                    .and_then(Value::as_str)
                    .unwrap_or("-")
                    .to_string(),
                t.get("title").and_then(Value::as_str).unwrap_or("").to_string(),
            ]
        })
        .collect();
    let text = Md::title(
        "✓",
        format!("{} subtask(s) created", created.len().max(tasks.len())),
    )
    .table(&["Key", "Title"], &created)
    .build();
    Ok(ToolOutput::with_text(object(result), text))
}

async fn artifact<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: ArtifactArgs = parse_required(args)?;
    if args.url.is_none() && args.text.is_none() && args.passed.is_none() {
        return Err(ToolFailure::InvalidArgs("give `url`, `text` or `passed`".into()));
    }
    let kind = serde_json::to_value(args.kind).unwrap_or(Value::Null);
    let body = json!({ "kind": kind, "url": args.url, "text": args.text, "title": args.title, "passed": args.passed });
    let result = backend.factory_attach_artifact(&body).await?;
    let text = Md::title(
        "✓",
        format!(
            "Artifact attached ({kind})",
            kind = body["kind"].as_str().unwrap_or("?")
        ),
    )
    .build();
    Ok(ToolOutput::with_text(object(result), text))
}

async fn rules<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: RulesArgs = parse(args)?;
    let rules = backend.factory_get_rules(args.project.as_deref()).await?;
    let checks: Vec<Vec<String>> = rules
        .checks
        .iter()
        .map(|c| {
            vec![
                c.id.clone(),
                if c.enabled { "on".into() } else { "off".into() },
                c.description.clone().unwrap_or_default(),
            ]
        })
        .collect();
    let text = Md::title("○", "Project rules for agents")
        .text(rules.managed_markdown.clone())
        .table(&["Check", "State", "Description"], &checks)
        .build();
    Ok(ToolOutput::with_text(to_value(&rules), text))
}

async fn lease<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: LeaseArgs = parse_required(args)?;
    if args.paths.is_empty() {
        return Err(ToolFailure::InvalidArgs("paths is empty".into()));
    }
    let lease = backend.factory_renew_lease(&args.paths).await?;
    let value = to_value(&lease);
    let granted = value.get("granted").and_then(Value::as_bool).unwrap_or(true);
    let md = if granted {
        Md::title("✓", format!("Lease on {}", args.paths.join(", ")))
    } else {
        Md::title("✕", "Lease held by another run").kv(&[(
            "held by",
            value.get("held_by").map(|v| v.to_string()).unwrap_or_default(),
        )])
    };
    Ok(ToolOutput::with_text(object(value), md.build()))
}

/// `**● AUR-12 Title**` + state, role, assignee (with Mei-er badge), description.
pub fn task_md(task: &ttm_api::factory::Task) -> String {
    let category = task
        .category
        .as_deref()
        .or(task.state.as_deref())
        .unwrap_or("inbox");
    let assignee = task
        .assignee_member
        .as_ref()
        .map(|m| who(&m.kind, &m.name, m.tier()))
        .unwrap_or_default();
    Md::title(glyph(category), format!("{} {}", task.key(), task.title))
        .kv(&[
            (
                "state",
                format!(
                    "{} ({})",
                    task.state.as_deref().unwrap_or("-"),
                    task.category.as_deref().unwrap_or("-")
                ),
            ),
            ("role", task.work_role().unwrap_or("").to_string()),
            ("assignee", assignee),
            (
                "estimate",
                task.estimate.map(|e| e.to_string()).unwrap_or_default(),
            ),
        ])
        .text(task.description.clone().unwrap_or_default())
        .next(&[(GET_CONTEXT, "read the prepared context")])
        .build()
}

/// The server's subtask input (camelCase; `estimate` is an integer).
pub fn subtask_body(t: SubtaskArg) -> Value {
    let mut body = json!({ "title": t.title });
    let obj = body.as_object_mut().expect("object");
    let mut put = |k: &str, v: Value| {
        if !v.is_null() && v != json!([]) {
            obj.insert(k.to_string(), v);
        }
    };
    put("type", json!(t.kind));
    put("priority", json!(t.priority));
    put("estimate", json!(t.estimate));
    put("role", json!(t.role));
    put("description", json!(t.description));
    put("labels", json!(t.labels));
    put("acceptanceCriteria", json!(t.acceptance_criteria));
    put("dependencies", json!(t.depends_on));
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtask_body_is_camel_case() {
        let arg: SubtaskArg = serde_json::from_value(json!({
            "title": "Write tests", "role": "QC", "estimate": 3, "depends_on": ["#0"], "acceptance_criteria": ["covers login"]
        }))
        .unwrap();
        assert_eq!(
            subtask_body(arg),
            json!({"title": "Write tests", "role": "QC", "estimate": 3, "dependencies": ["#0"], "acceptanceCriteria": ["covers login"]})
        );
    }

    #[test]
    fn artifact_kinds_match_the_server() {
        assert_eq!(serde_json::to_value(ArtifactKind::Pr).unwrap(), "pr");
        assert_eq!(
            serde_json::to_value(ArtifactKind::TestResult).unwrap(),
            "test_result"
        );
    }
}
