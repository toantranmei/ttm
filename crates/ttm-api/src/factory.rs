//! Factory endpoints: projects, plan export and change requests (sync), rules, tasks.

use crate::workspace::encode;
use crate::{ApiError, Client, Result};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// `GET /factory/projects[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// Task key prefix, e.g. `AUR`.
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub repo_provider: Option<String>,
    #[serde(default)]
    pub repo_url: Option<String>,
    #[serde(default)]
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

/// `GET /factory/projects/{id}/export` (assumed `{rev, files: {"plan/index.md": "…"}}`; paths are relative to
/// `.ttm/`, so `plan/…`). The CLI strips the leading `plan/`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanBundle {
    pub rev: i64,
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

/// A created change request (assumed `{id, status, changes[]}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeRequest {
    pub id: String,
    /// `pending`, `approved`, `applied`, `rejected`.
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub changes: Vec<Value>,
    /// Project rev after applying (when applied at once).
    #[serde(default)]
    pub rev: Option<i64>,
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

/// A task (fields the CLI and tools use; the rest is kept in `extra`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub key: String,
    pub title: String,
    #[serde(default)]
    pub state: Option<String>,
    /// One of the 9 board statuses.
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    /// `{kind: user|agent, id, name}`.
    #[serde(default)]
    pub assignee: Option<Value>,
    #[serde(default, alias = "parentId")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub estimate: Option<f64>,
    #[serde(default)]
    pub rev: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Task {
    pub fn assignee_name(&self) -> Option<String> {
        let a = self.assignee.as_ref()?;
        a.get("name")
            .and_then(Value::as_str)
            .or_else(|| a.as_str())
            .map(String::from)
    }
}

impl Client {
    pub async fn projects(&self) -> Result<Vec<Project>> {
        self.get("/factory/projects").await
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

    /// `GET /factory/change-requests/{id}` (assumed).
    pub async fn change_request(&self, id: &str) -> Result<ChangeRequest> {
        self.get(&format!("/factory/change-requests/{}", encode(id)))
            .await
    }

    pub async fn rules(&self, project_id: &str) -> Result<RuleSet> {
        self.get(&format!("/factory/projects/{}/rules", encode(project_id)))
            .await
    }

    /// `GET /factory/projects/{id}/resolve-prompt?rev=` → `{prompt}` (assumed).
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

    /// `POST /factory/projects/{id}/rule-checks` (assumed).
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
        let path = format!("/factory/tasks/{}", encode(&task.id));
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

    pub async fn transition_task(&self, id: &str, to: &str) -> Result<Value> {
        self.post(
            &format!("/factory/tasks/{}/transition", encode(id)),
            &serde_json::json!({ "to": to }),
        )
        .await
    }

    /// `decision`: `approve` or `reject`.
    pub async fn approve_task(&self, id: &str, decision: &str) -> Result<Value> {
        self.post(
            &format!("/factory/tasks/{}/approve", encode(id)),
            &serde_json::json!({ "decision": decision }),
        )
        .await
    }

    /// `POST /factory/tasks/{id}/subtasks {tasks, run_id?}`.
    pub async fn create_subtasks(&self, id: &str, tasks: &[Value], run_id: Option<&str>) -> Result<Value> {
        let mut body = serde_json::json!({ "tasks": tasks });
        if let Some(run) = run_id {
            body["run_id"] = Value::String(run.to_string());
        }
        self.post(&format!("/factory/tasks/{}/subtasks", encode(id)), &body)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_strips_plan_prefix() {
        let mut files = BTreeMap::new();
        files.insert("plan/index.md".to_string(), "x".to_string());
        files.insert("plan/tasks/AUR-1.md".to_string(), "y".to_string());
        let b = PlanBundle { rev: 3, files };
        let plan = b.plan_files();
        assert!(plan.contains_key("index.md") && plan.contains_key("tasks/AUR-1.md"));
    }

    #[test]
    fn task_keeps_unknown_fields() {
        let t: Task = serde_json::from_value(serde_json::json!({
            "id": "t1", "key": "AUR-1", "title": "Login", "state": "doing", "assignee": {"kind": "agent", "id": "a1", "name": "Dev bot"},
            "parentId": "t0", "labels": ["auth"], "rev": 4
        }))
        .unwrap();
        assert_eq!(t.parent_id.as_deref(), Some("t0"));
        assert_eq!(t.assignee_name().as_deref(), Some("Dev bot"));
        assert!(t.extra.contains_key("labels"));
    }

    #[test]
    fn rule_check_defaults_to_enabled() {
        let r: RuleCheck = serde_json::from_value(serde_json::json!({"id": "no_push_main"})).unwrap();
        assert!(r.enabled);
    }
}
