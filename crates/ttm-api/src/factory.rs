//! Factory endpoints: projects, plan export and change requests (sync), rules, tasks.

use crate::workspace::encode;
use crate::{ApiError, Client, Result};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// A project: list items of `GET /factory/projects` (camelCase) or a single project (snake_case); both parse.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    #[serde(default, alias = "workspaceId")]
    pub workspace_id: Option<String>,
    /// Task key prefix, e.g. `AUR`.
    pub key: String,
    pub name: String,
    #[serde(default, alias = "repoProvider")]
    pub repo_provider: Option<String>,
    #[serde(default, alias = "repoUrl")]
    pub repo_url: Option<String>,
    #[serde(default, alias = "defaultBranch")]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub rev: i64,
}

/// `POST /factory/projects` body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewProject {
    pub key: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// `github`, `gitlab` or `local`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
}

/// `GET /factory/projects/{id}/export` → `{rev, project_id, key, files: {"plan/index.md": …, "plan/tasks/<KEY>.md":
/// …}}`; task files have a front matter whose values are JSON. The CLI stores files without the leading `plan/`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanBundle {
    pub rev: i64,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

impl PlanBundle {
    /// Files relative to the plan directory (`plan/tasks/A.md` → `tasks/A.md`).
    pub fn plan_files(&self) -> BTreeMap<String, String> {
        self.files
            .iter()
            .map(|(k, v)| (k.strip_prefix("plan/").unwrap_or(k).to_string(), v.clone()))
            .collect()
    }
}

/// Plan-relative path (`tasks/A.md`) → bundle path used by change requests (`plan/tasks/A.md`).
pub fn bundle_path(plan_relative: &str) -> String {
    if plan_relative.starts_with("plan/") {
        plan_relative.to_string()
    } else {
        format!("plan/{plan_relative}")
    }
}

/// A field changed on both sides (`409` of a change request).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conflict {
    pub task: String,
    pub field: String,
    #[serde(default)]
    pub base: Value,
    #[serde(default)]
    pub local: Value,
    #[serde(default)]
    pub server: Value,
}

/// A change request. `status`: `pending`, `applied`, `rejected`, or `empty` (nothing changed; nothing stored, `id`
/// is empty). `skipped` lists ignored files with the reason (deletions, `plan/index.md`, unreadable files).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeRequest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub base_rev: Option<i64>,
    /// `[{key, op, fields, server}]`.
    #[serde(default)]
    pub changes: Vec<Value>,
    /// Project rev after applying.
    #[serde(default)]
    pub rev: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub skipped: Vec<String>,
}

impl ChangeRequest {
    /// Nothing changed: the server stored nothing.
    pub fn is_empty(&self) -> bool {
        self.status == "empty" || self.id.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeRequestOutcome {
    Created(ChangeRequest),
    Conflicts(Vec<Conflict>),
}

/// `GET /factory/projects/{id}/rules`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleSet {
    #[serde(default)]
    pub rev: Option<i64>,
    /// Markdown placed between `<!-- ttm:begin -->` and `<!-- ttm:end -->`.
    #[serde(default)]
    pub managed_markdown: String,
    #[serde(default)]
    pub hooks: Value,
    #[serde(default)]
    pub checks: Vec<RuleCheck>,
}

/// One local rule (assumed `{id, description?, command?, enabled}`). Known ids: `no_push_main`, `no_verify`,
/// `conventional_commits`, `tests_before_done` (`command` = test command).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleCheck {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default = "yes")]
    pub enabled: bool,
}

fn yes() -> bool {
    true
}

/// Result of one rule evaluation, reported as a `rule_check` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleCheckReport {
    pub rule: String,
    pub passed: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

/// The member doing a task (`assigneeMember`, `createdBy`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberRef {
    /// `user` or `agent`.
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Mei-er membership of a person (`{tier}`), when the server sends it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership: Option<MembershipRef>,
    /// Mei-er membership of the owner of an agent's workspace.
    #[serde(default, alias = "ownerMembership", skip_serializing_if = "Option::is_none")]
    pub owner_membership: Option<MembershipRef>,
}

/// `membership: {tier}` on people and `owner_membership: {tier}` on agents (`supporter`, `mei-er`, `sponsor`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MembershipRef {
    pub tier: String,
}

impl MemberRef {
    /// The tier to show next to this member: its own, else its owner's.
    pub fn tier(&self) -> Option<&str> {
        self.membership
            .as_ref()
            .or(self.owner_membership.as_ref())
            .map(|m| m.tier.as_str())
    }
}

/// A task as the server shows it (camelCase). `id` is the task KEY (`AUR-12`); the database id is `uuid`. `assignee`
/// is the work ROLE; the person or agent doing it is `assignee_member`. Task routes accept the key or the uuid.
/// Fields not listed here are kept in `extra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub uuid: Option<String>,
    pub title: String,
    #[serde(default)]
    pub state: Option<String>,
    /// One of the 9 board statuses.
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    /// The work role (same as `role`).
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub assignee_member: Option<MemberRef>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub estimate: Option<f64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub rev: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Task {
    /// `AUR-12`.
    pub fn key(&self) -> &str {
        if self.key.is_empty() { &self.id } else { &self.key }
    }

    /// Name of the person or agent doing it.
    pub fn assignee_name(&self) -> Option<String> {
        self.assignee_member
            .as_ref()
            .map(|m| m.name.clone())
            .filter(|n| !n.is_empty())
    }

    pub fn assignee_id(&self) -> Option<&str> {
        self.assignee_member.as_ref().map(|m| m.id.as_str())
    }

    /// Work role: `role`, else `assignee`.
    pub fn work_role(&self) -> Option<&str> {
        self.role
            .as_deref()
            .or(self.assignee.as_deref())
            .filter(|r| !r.is_empty())
    }
}

/// A merge queue entry (`GET /factory/projects/{id}/merge-queue`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeEntry {
    pub id: String,
    pub run_id: String,
    pub task_key: String,
    pub branch: String,
    #[serde(default)]
    pub pr_url: Option<String>,
    /// `waiting`, `ready`, `merged`, `conflict`, `cancelled`.
    pub status: String,
    /// Head of the queue: merge this one now.
    #[serde(default)]
    pub can_merge: bool,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub enqueued_at: Option<String>,
}

impl Client {
    pub async fn projects(&self) -> Result<Vec<Project>> {
        self.get("/factory/projects").await
    }

    /// `GET /factory/projects/{id}` (snake_case, with repository settings).
    pub async fn project(&self, id: &str) -> Result<Project> {
        self.get(&format!("/factory/projects/{}", encode(id))).await
    }

    pub async fn create_project(&self, project: &NewProject) -> Result<Project> {
        self.post("/factory/projects", project).await
    }

    /// Project by id or key (from the project list).
    pub async fn find_project(&self, id_or_key: &str) -> Result<Option<Project>> {
        Ok(self
            .projects()
            .await?
            .into_iter()
            .find(|p| p.id == id_or_key || p.key.eq_ignore_ascii_case(id_or_key)))
    }

    pub async fn export_plan(&self, project_id: &str, rev: Option<i64>) -> Result<PlanBundle> {
        let params: Vec<(&str, String)> = rev.map(|r| vec![("rev", r.to_string())]).unwrap_or_default();
        self.get_query(
            &format!("/factory/projects/{}/export", encode(project_id)),
            &params,
        )
        .await
    }

    /// `POST /factory/projects/{id}/change-requests {base_rev, files}`; `files` values `null` mean deleted.
    pub async fn create_change_request(
        &self,
        project_id: &str,
        base_rev: i64,
        files: &BTreeMap<String, Option<String>>,
    ) -> Result<ChangeRequestOutcome> {
        let body = serde_json::json!({ "base_rev": base_rev, "files": files });
        let path = format!("/factory/projects/{}/change-requests", encode(project_id));
        let (status, value) = self.json_raw(Method::POST, &path, &body, None).await?;
        match status {
            200..=299 => serde_json::from_value(value)
                .map(ChangeRequestOutcome::Created)
                .map_err(|e| ApiError::Decode {
                    status,
                    message: e.to_string(),
                }),
            409 if value.get("conflicts").is_some() => {
                let conflicts =
                    serde_json::from_value(value["conflicts"].clone()).map_err(|e| ApiError::Decode {
                        status,
                        message: e.to_string(),
                    })?;
                Ok(ChangeRequestOutcome::Conflicts(conflicts))
            }
            _ => Err(ApiError::from_body(
                status,
                "application/json",
                &value.to_string(),
            )),
        }
    }

    /// `GET /factory/change-requests/{id}` .
    pub async fn change_request(&self, id: &str) -> Result<ChangeRequest> {
        self.get(&format!("/factory/change-requests/{}", encode(id)))
            .await
    }

    pub async fn rules(&self, project_id: &str) -> Result<RuleSet> {
        self.get(&format!("/factory/projects/{}/rules", encode(project_id)))
            .await
    }

    /// `GET /factory/projects/{id}/resolve-prompt?rev=` → `{prompt}` .
    pub async fn resolve_prompt(&self, project_id: &str, rev: i64) -> Result<String> {
        let v: Value = self
            .get_query(
                &format!("/factory/projects/{}/resolve-prompt", encode(project_id)),
                &[("rev", rev.to_string())],
            )
            .await?;
        Ok(v.get("prompt")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    /// `POST /factory/projects/{id}/rule-checks` .
    pub async fn report_rule_check(&self, project_id: &str, report: &RuleCheckReport) -> Result<()> {
        self.post::<_, Value>(
            &format!("/factory/projects/{}/rule-checks", encode(project_id)),
            report,
        )
        .await
        .map(drop)
    }

    /// Tasks of the project view (`GET /factory/projects/{id}/view`, field `tasks`).
    pub async fn tasks(&self, project_id: &str) -> Result<Vec<Task>> {
        let view: Value = self
            .get(&format!("/factory/projects/{}/view", encode(project_id)))
            .await?;
        let tasks = view.get("tasks").cloned().unwrap_or(Value::Array(vec![]));
        serde_json::from_value(tasks).map_err(|e| ApiError::Decode {
            status: 200,
            message: e.to_string(),
        })
    }

    /// `GET /factory/tasks/{id-or-key}` (assumed to accept a key like `AUR-12`).
    pub async fn task(&self, id_or_key: &str) -> Result<Task> {
        self.get(&format!("/factory/tasks/{}", encode(id_or_key))).await
    }

    /// `PATCH /factory/tasks/{id}` with `If-Match: <rev>`; 412 → `rev-conflict` problem.
    pub async fn update_task(&self, task: &Task, changes: &Value) -> Result<Task> {
        let rev = task.rev.map(|r| r.to_string());
        let path = format!("/factory/tasks/{}", encode(task.key()));
        let (status, value) = self
            .json_raw(Method::PATCH, &path, changes, rev.as_deref())
            .await?;
        if (200..300).contains(&status) {
            return serde_json::from_value(value).map_err(|e| ApiError::Decode {
                status,
                message: e.to_string(),
            });
        }
        Err(ApiError::from_body(
            status,
            "application/json",
            &value.to_string(),
        ))
    }

    pub async fn transition_task(&self, id: &str, to: &str) -> Result<Task> {
        self.post(
            &format!("/factory/tasks/{}/transition", encode(id)),
            &serde_json::json!({ "to": to }),
        )
        .await
    }

    /// `decision`: `approve` or `reject`.
    pub async fn approve_task(&self, id: &str, decision: &str) -> Result<Task> {
        self.post(
            &format!("/factory/tasks/{}/approve", encode(id)),
            &serde_json::json!({ "decision": decision }),
        )
        .await
    }

    /// `POST /factory/tasks/{id}/subtasks {tasks, run_id?}`.
    /// Each task: `{title, type?, priority?, estimate? (integer), role?, description?, labels?, acceptanceCriteria?:
    /// [string], dependencies?: [key | "#<n>"]}` → `{tasks, warnings}`.
    pub async fn create_subtasks(&self, id: &str, tasks: &[Value], run_id: Option<&str>) -> Result<Value> {
        let mut body = serde_json::json!({ "tasks": tasks });
        if let Some(run) = run_id {
            body["run_id"] = Value::String(run.to_string());
        }
        self.post(&format!("/factory/tasks/{}/subtasks", encode(id)), &body)
            .await
    }
}

/// The keys of a subtask the server reads (`SubtaskInput`, camelCase).
pub const SUBTASK_KEYS: [&str; 9] = [
    "title",
    "type",
    "priority",
    "estimate",
    "role",
    "description",
    "labels",
    "acceptanceCriteria",
    "dependencies",
];

/// Maps one subtask written in either style to the server's keys: snake_case (as the `factory-create-subtasks` MCP
/// tool takes it: `acceptance_criteria`, `depends_on`) or camelCase (`acceptanceCriteria`, `dependencies`); `kind`
/// is `type`. Returns the normalised object and the keys that were dropped because the server does not read them.
pub fn normalize_subtask(task: Value) -> (Value, Vec<String>) {
    let Value::Object(map) = task else {
        return (task, Vec::new());
    };
    let mut out = Map::new();
    let mut unknown = Vec::new();
    for (key, value) in map {
        let target = match key.as_str() {
            "acceptance_criteria" => "acceptanceCriteria",
            "depends_on" | "dependsOn" => "dependencies",
            "kind" => "type",
            other => match SUBTASK_KEYS.iter().find(|k| **k == other) {
                Some(k) => k,
                None => {
                    unknown.push(key);
                    continue;
                }
            },
        };
        // An explicit server key wins over its alias.
        if key == target || !out.contains_key(target) {
            out.insert(target.to_string(), value);
        }
    }
    (Value::Object(out), unknown)
}

// Runner, runs and jobs (P8).

/// `POST /factory/runners/register` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisterRunner {
    pub name: String,
    pub os: String,
    pub arch: String,
    pub version: String,
    pub harnesses: Vec<String>,
    pub max_parallel: usize,
    pub accept_team_jobs: bool,
}

/// `POST /factory/runners/register` answer: the runner id and a `ttm_job_` token for the runner channel.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct RunnerRegistration {
    pub runner_id: String,
    pub token: String,
    /// RFC 3339; the token (scope `factory:run`) lives 12 hours. Registering again under the same name keeps the
    /// id and revokes the previous token.
    #[serde(default)]
    pub expires_at: Option<String>,
}

impl std::fmt::Debug for RunnerRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunnerRegistration")
            .field("runner_id", &self.runner_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JobRepo {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub base_branch: Option<String>,
    /// `github`, `gitlab`, `local`.
    #[serde(default)]
    pub provider: Option<String>,
}

/// MCP access of the job: a short-lived token (`factory:run notee:read notee:comment` …) and toolsets.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JobMcp {
    pub token: String,
    #[serde(default)]
    pub toolsets: Vec<String>,
}

impl std::fmt::Debug for JobMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobMcp")
            .field("toolsets", &self.toolsets)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JobLimits {
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub budget_tokens: Option<u64>,
    #[serde(default)]
    pub timeout_min: Option<u64>,
}

/// What the server asks a runner to do. The prompt is final; the logic that produced it stays on the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSpec {
    pub run_id: String,
    pub task_key: String,
    #[serde(default)]
    pub task_id: Option<String>,
    /// `task`, or `resolve` (fix a merge conflict).
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub project_key: Option<String>,
    #[serde(default)]
    pub repo: JobRepo,
    pub branch: String,
    #[serde(default)]
    pub worktree_hint: Option<String>,
    #[serde(default = "claude_code")]
    pub harness: String,
    #[serde(default)]
    pub model: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub mcp: JobMcp,
    #[serde(default)]
    pub limits: JobLimits,
    #[serde(default)]
    pub rules_rev: Option<i64>,
}

fn claude_code() -> String {
    "claude-code".into()
}

impl JobSpec {
    /// Project key: `project_key`, else the prefix of `task_key` (`AUR-12` → `AUR`).
    pub fn project_key(&self) -> String {
        self.project_key.clone().unwrap_or_else(|| {
            self.task_key
                .rsplit_once('-')
                .map(|(k, _)| k.to_string())
                .unwrap_or_default()
        })
    }
}

/// One event of a run's activity stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunEvent {
    /// Set by the server when reading events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// `thought`, `action`, `plan`, `approval_request`, `response`, `log`, `rule_check`, `usage`, `progress`.
    pub kind: String,
    #[serde(default)]
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// RFC 3339.
    #[serde(default)]
    pub at: Option<String>,
}

impl RunEvent {
    pub fn now(kind: &str, message: impl Into<String>, data: Option<Value>) -> Self {
        let at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .ok();
        Self {
            id: None,
            kind: kind.into(),
            message: message.into(),
            data,
            at,
        }
    }
}

/// `POST /factory/runners/{id}/heartbeat` body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Heartbeat {
    pub running: Vec<String>,
    pub capacity: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused_until: Option<String>,
}

/// Heartbeat answer : runs to stop (kill switch) and whether to shut down (turned off from the web).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatReply {
    #[serde(default)]
    pub stop_runs: Vec<String>,
    #[serde(default)]
    pub shutdown: bool,
}

/// `POST /factory/runs/{id}/finish` body .
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RunFinish {
    /// `succeeded`, `failed`, `cancelled`, `paused`.
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_url: Option<String>,
    #[serde(default)]
    pub tokens_in: u64,
    #[serde(default)]
    pub tokens_out: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paused_until: Option<String>,
    /// The agent ran the project's test command: `true` passed, `false` failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests_pass: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci_green: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Lease answer `{granted, paths, expires_at}`; a 409 `{held_by: {run_id, task, paths}}` comes back as `granted:
/// false`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    #[serde(default)]
    pub granted: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub held_by: Option<Value>,
    #[serde(default)]
    pub paths: Vec<String>,
}

impl Client {
    pub async fn register_runner(&self, body: &RegisterRunner) -> Result<RunnerRegistration> {
        self.post("/factory/runners/register", body).await
    }

    /// Long-poll `GET /factory/runners/{id}/jobs?wait=<s>`: `None` on 204 / empty.
    pub async fn poll_job(&self, runner_id: &str, wait_secs: u64) -> Result<Option<JobSpec>> {
        let path = format!("/factory/runners/{}/jobs", encode(runner_id));
        let response = self
            .send(Method::GET, &path, &[("wait", wait_secs.to_string())], None)
            .await?;
        let status = response.status().as_u16();
        let bytes = response.bytes().await?;
        if status == 204 || bytes.is_empty() {
            return Ok(None);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|e| ApiError::Decode {
            status,
            message: e.to_string(),
        })?;
        let job = match value {
            Value::Null => return Ok(None),
            Value::Object(ref map) if map.contains_key("job") => map["job"].clone(),
            other => other,
        };
        if job.is_null() {
            return Ok(None);
        }
        serde_json::from_value(job)
            .map(Some)
            .map_err(|e| ApiError::Decode {
                status,
                message: e.to_string(),
            })
    }

    pub async fn accept_job(&self, run_id: &str) -> Result<()> {
        self.post::<_, Value>(
            &format!("/factory/runs/{}/accept", encode(run_id)),
            &serde_json::json!({}),
        )
        .await
        .map(drop)
    }

    pub async fn heartbeat(&self, runner_id: &str, beat: &Heartbeat) -> Result<HeartbeatReply> {
        self.post(&format!("/factory/runners/{}/heartbeat", encode(runner_id)), beat)
            .await
    }

    /// `POST /factory/runs/{id}/events` (batch).
    pub async fn post_run_events(&self, run_id: &str, events: &[RunEvent]) -> Result<()> {
        self.post::<_, Value>(
            &format!("/factory/runs/{}/events", encode(run_id)),
            &serde_json::json!({ "events": events }),
        )
        .await
        .map(drop)
    }

    /// Events with an id greater than `since`.
    pub async fn run_events(&self, run_id: &str, since: Option<i64>) -> Result<Vec<RunEvent>> {
        let params: Vec<(&str, String)> = since.map(|s| vec![("since", s.to_string())]).unwrap_or_default();
        self.get_query(&format!("/factory/runs/{}/events", encode(run_id)), &params)
            .await
    }

    pub async fn finish_run(&self, run_id: &str, finish: &RunFinish) -> Result<()> {
        self.post::<_, Value>(&format!("/factory/runs/{}/finish", encode(run_id)), finish)
            .await
            .map(drop)
    }

    pub async fn stop_run(&self, run_id: &str) -> Result<()> {
        self.post::<_, Value>(
            &format!("/factory/runs/{}/stop", encode(run_id)),
            &serde_json::json!({}),
        )
        .await
        .map(drop)
    }

    /// `GET /factory/projects/{id}/merge-queue`.
    pub async fn merge_queue(&self, project_id: &str) -> Result<Vec<MergeEntry>> {
        self.get(&format!("/factory/projects/{}/merge-queue", encode(project_id)))
            .await
    }

    /// `POST /factory/merge-queue/{id}/result {status: merged|conflict, detail?}`.
    pub async fn merge_result(
        &self,
        entry_id: &str,
        merged: bool,
        detail: Option<&str>,
    ) -> Result<MergeEntry> {
        let body =
            serde_json::json!({ "status": if merged { "merged" } else { "conflict" }, "detail": detail });
        self.post(
            &format!("/factory/merge-queue/{}/result", encode(entry_id)),
            &body,
        )
        .await
    }

    /// `POST /factory/runs/{id}/leases {paths}`; a 409 comes back as `granted: false` with `held_by`.
    pub async fn request_lease(&self, run_id: &str, paths: &[String]) -> Result<Lease> {
        let path = format!("/factory/runs/{}/leases", encode(run_id));
        let (status, value) = self
            .json_raw(Method::POST, &path, &serde_json::json!({ "paths": paths }), None)
            .await?;
        match status {
            200..=299 => {
                let mut lease: Lease = serde_json::from_value(value).unwrap_or_default();
                if lease.paths.is_empty() {
                    lease.paths = paths.to_vec();
                }
                if !lease.granted && lease.held_by.is_none() {
                    lease.granted = true;
                }
                Ok(lease)
            }
            409 => Ok(Lease {
                granted: false,
                held_by: value.get("held_by").cloned().or(Some(value.clone())),
                paths: paths.to_vec(),
                expires_at: None,
            }),
            _ => Err(ApiError::from_body(
                status,
                "application/json",
                &value.to_string(),
            )),
        }
    }

    /// `POST /factory/tasks/{key}/runs {runner_id}` → job for this machine (`ttm agent run`).
    pub async fn start_task_run(&self, task: &str, runner_id: &str) -> Result<JobSpec> {
        self.post(
            &format!("/factory/tasks/{}/runs", encode(task)),
            &serde_json::json!({ "runner_id": runner_id }),
        )
        .await
    }

    /// Context packed by the server for a run (`GET /factory/runs/{id}/context` → `{markdown}`).
    pub async fn run_context(&self, run_id: &str) -> Result<Value> {
        self.get(&format!("/factory/runs/{}/context", encode(run_id)))
            .await
    }

    /// Context of a task outside a run (`GET /factory/tasks/{id}/context`).
    pub async fn task_context(&self, task: &str) -> Result<Value> {
        self.get(&format!("/factory/tasks/{}/context", encode(task)))
            .await
    }

    /// `POST /factory/runs/{id}/approval-requests {reason}` .
    pub async fn request_approval(&self, run_id: &str, reason: &str) -> Result<Value> {
        self.post(
            &format!("/factory/runs/{}/approval-requests", encode(run_id)),
            &serde_json::json!({ "reason": reason }),
        )
        .await
    }

    /// `POST /factory/runs/{id}/artifacts {kind, url?, text?, title?}` .
    pub async fn attach_artifact(&self, run_id: &str, artifact: &Value) -> Result<Value> {
        self.post(&format!("/factory/runs/{}/artifacts", encode(run_id)), artifact)
            .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn subtasks_accept_both_key_styles() {
        let (snake, unknown) = normalize_subtask(serde_json::json!({
            "title": "Write tests", "acceptance_criteria": ["covers login"], "depends_on": ["#0"], "kind": "chore",
            "estimate": 3, "colour": "red"
        }));
        assert_eq!(
            snake,
            serde_json::json!({"title": "Write tests", "acceptanceCriteria": ["covers login"], "dependencies": ["#0"],
                "type": "chore", "estimate": 3})
        );
        assert_eq!(unknown, ["colour"]);
        let camel = serde_json::json!({"title": "A", "acceptanceCriteria": ["x"], "dependencies": ["AUR-1"], "type": "bug"});
        assert_eq!(normalize_subtask(camel.clone()), (camel, vec![]));
        let (both, _) = normalize_subtask(
            serde_json::json!({"title": "A", "dependencies": ["AUR-1"], "depends_on": ["#0"]}),
        );
        assert_eq!(both["dependencies"], serde_json::json!(["AUR-1"]));
    }

    use super::*;

    #[test]
    fn bundle_strips_plan_prefix() {
        let mut files = BTreeMap::new();
        files.insert("plan/index.md".to_string(), "x".to_string());
        files.insert("plan/tasks/AUR-1.md".to_string(), "y".to_string());
        let b = PlanBundle {
            rev: 3,
            files,
            ..PlanBundle::default()
        };
        let plan = b.plan_files();
        assert!(plan.contains_key("index.md") && plan.contains_key("tasks/AUR-1.md"));
    }

    #[test]
    fn task_keeps_unknown_fields() {
        let t: Task = serde_json::from_value(serde_json::json!({
            "id": "AUR-1", "key": "AUR-1", "uuid": "0b7c", "title": "Login", "state": "doing", "category": "in-progress",
            "assignee": "DEV", "role": "DEV", "assigneeMember": {"kind": "agent", "id": "a1", "name": "Dev bot"},
            "parentId": "AUR-0", "labels": ["auth"], "rev": 4
        }))
        .unwrap();
        assert_eq!(t.key(), "AUR-1");
        assert_eq!(t.work_role(), Some("DEV"));
        assert_eq!(t.parent_id.as_deref(), Some("AUR-0"));
        assert_eq!(t.assignee_name().as_deref(), Some("Dev bot"));
        assert!(t.extra.contains_key("labels"));
    }

    #[test]
    fn rule_check_defaults_to_enabled() {
        let r: RuleCheck = serde_json::from_value(serde_json::json!({"id": "no_push_main"})).unwrap();
        assert!(r.enabled);
    }

    #[test]
    fn job_spec_parses_and_hides_token() {
        let job: JobSpec = serde_json::from_value(serde_json::json!({
            "run_id": "r1", "task_key": "AUR-12", "repo": {"url": "git@github.com:a/b.git", "base_branch": "main"},
            "branch": "aur-12-login", "harness": "claude-code", "model": "sonnet", "prompt": "Do it",
            "allowed_tools": ["Read", "Edit"], "mcp": {"token": "ttm_job_SECRET", "toolsets": ["factory", "notee"]},
            "limits": {"max_turns": 40, "timeout_min": 90}
        }))
        .unwrap();
        assert_eq!(job.project_key(), "AUR");
        assert!(!format!("{job:?}").contains("SECRET"));
    }

    #[test]
    fn project_list_and_single_shapes() {
        let item: Project = serde_json::from_value(serde_json::json!({
            "id": "p1", "workspaceId": "w1", "name": "Aurora", "code": "AUR", "key": "AUR", "health": "ok", "rev": 3
        }))
        .unwrap();
        assert_eq!(item.workspace_id.as_deref(), Some("w1"));
        let single: Project = serde_json::from_value(serde_json::json!({
            "id": "p1", "workspace_id": "w1", "key": "AUR", "name": "Aurora", "repo_provider": "github",
            "default_branch": "main", "rev": 3
        }))
        .unwrap();
        assert_eq!(single.default_branch.as_deref(), Some("main"));
    }

    #[test]
    fn empty_change_request_and_paths() {
        let cr: ChangeRequest = serde_json::from_value(serde_json::json!({
            "id": "", "status": "empty", "base_rev": 4, "changes": [],
            "skipped": ["plan/tasks/X.md: deleting tasks from a local copy is not supported"]
        }))
        .unwrap();
        assert!(cr.is_empty());
        assert_eq!(cr.skipped.len(), 1);
        assert_eq!(bundle_path("tasks/A.md"), "plan/tasks/A.md");
        assert_eq!(bundle_path("plan/index.md"), "plan/index.md");
    }
}
