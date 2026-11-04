//! Workflow v2 (S2): versions, drafts, validation, simulation, diffs, publishing and project upgrades.

use crate::Result;
use crate::client::Client;
use crate::workspace::encode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `GET /factory/workflows/{id}[/versions/{v}]`: `definition` holds `states[]` and `transitions[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub latest_version: Option<i64>,
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub has_draft: bool,
    #[serde(default)]
    pub draft_updated_at: Option<String>,
    #[serde(default)]
    pub source: Option<Value>,
    #[serde(default)]
    pub projects: Vec<Value>,
    #[serde(default)]
    pub definition: Value,
    #[serde(default)]
    pub draft: Option<WorkflowDraft>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDraft {
    #[serde(default)]
    pub definition: Value,
    #[serde(default)]
    pub rules: Vec<Value>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub updated_by: Option<Value>,
}

/// A state of a workflow definition (fields the CLI reads; the rest stays in `extra`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkflowState {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub auto_assign: Option<Value>,
    #[serde(default)]
    pub wip: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkflowTransition {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub from: Value,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub guard: Option<Value>,
    #[serde(default)]
    pub rules: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Workflow {
    /// The definition shown: the draft when `draft` is asked and there is one.
    pub fn states(definition: &Value) -> Vec<WorkflowState> {
        definition
            .get("states")
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or_default()
    }

    pub fn transitions(definition: &Value) -> Vec<WorkflowTransition> {
        definition
            .get("transitions")
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or_default()
    }
}

/// One validation issue (`POST /factory/workflows/{id}/validate`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidationIssue {
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Validation {
    #[serde(default)]
    pub issues: Vec<ValidationIssue>,
}

/// `POST /factory/ascode/validate` (S5) answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AscodeValidation {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub errors: Vec<AscodeError>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AscodeError {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub line: Option<i64>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub rule: Option<String>,
    #[serde(default)]
    pub message: String,
}

/// `POST /factory/workflows/{id}/simulate` body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimulateInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    pub actor: SimActor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facts: Option<Value>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimActor {
    pub kind: String,
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Simulation {
    #[serde(default)]
    pub steps: Vec<SimStep>,
    #[serde(default)]
    pub end_state: Option<String>,
    #[serde(default)]
    pub options: Vec<Value>,
    #[serde(default)]
    pub reachable: Vec<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimStep {
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub transition_id: Option<String>,
    #[serde(default)]
    pub allowed: bool,
    #[serde(default)]
    pub blocked: Option<SimBlocked>,
    #[serde(default)]
    pub overrides: Option<Value>,
    #[serde(default)]
    pub approval_gate: Option<Value>,
    #[serde(default)]
    pub rules: Vec<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SimBlocked {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// `{added, removed, changed}` of one part of a workflow diff.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiffPart {
    #[serde(default)]
    pub added: Vec<Value>,
    #[serde(default)]
    pub removed: Vec<Value>,
    #[serde(default)]
    pub changed: Vec<Value>,
}

/// `GET /factory/workflows/{id}/diff?from&to`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDiff {
    #[serde(default)]
    pub from: Value,
    #[serde(default)]
    pub to: Value,
    #[serde(default)]
    pub states: DiffPart,
    #[serde(default)]
    pub transitions: DiffPart,
    #[serde(default)]
    pub rules: DiffPart,
    #[serde(default)]
    pub layout_only: bool,
}

/// `GET /factory/projects/{id}/workflow-upgrade`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpgradePreview {
    #[serde(default)]
    pub workflow_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub from_version: i64,
    #[serde(default)]
    pub to_version: i64,
    #[serde(default)]
    pub diff: WorkflowDiff,
    #[serde(default)]
    pub remap: Vec<Remap>,
    #[serde(default)]
    pub counts: Value,
    #[serde(default)]
    pub preview_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Remap {
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub from_state: String,
    #[serde(default)]
    pub to_state: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// A label for a diff item (`{id, name}` objects or strings).
pub fn item_label(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Object(o) => o
            .get("name")
            .or_else(|| o.get("id"))
            .or_else(|| o.get("slug"))
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| v.to_string()),
        other => other.to_string(),
    }
}

impl Client {
    pub async fn workflow(&self, id: &str) -> Result<Workflow> {
        self.get(&format!("/factory/workflows/{}", encode(id))).await
    }

    pub async fn workflow_version(&self, id: &str, version: i64) -> Result<Workflow> {
        self.get(&format!("/factory/workflows/{}/versions/{version}", encode(id)))
            .await
    }

    pub async fn workflow_validate(&self, id: &str, definition: &Value) -> Result<Validation> {
        self.post(
            &format!("/factory/workflows/{}/validate", encode(id)),
            &serde_json::json!({ "definition": definition }),
        )
        .await
    }

    /// `POST /factory/ascode/validate {workspace_id, files}` (S5).
    pub async fn ascode_validate(
        &self,
        workspace_id: &str,
        files: &std::collections::BTreeMap<String, String>,
    ) -> Result<AscodeValidation> {
        self.post(
            "/factory/ascode/validate",
            &serde_json::json!({ "workspace_id": workspace_id, "files": files }),
        )
        .await
    }

    pub async fn workflow_simulate(&self, id: &str, input: &SimulateInput) -> Result<Simulation> {
        self.post(&format!("/factory/workflows/{}/simulate", encode(id)), input)
            .await
    }

    /// `to` is a version number or `draft`.
    pub async fn workflow_diff(&self, id: &str, from: &str, to: &str) -> Result<WorkflowDiff> {
        self.get_query(
            &format!("/factory/workflows/{}/diff", encode(id)),
            &[("from", from.to_string()), ("to", to.to_string())],
        )
        .await
    }

    /// `PUT …/draft {name?, definition, base_version}` → `{draft, issues}`; 409 `draft-stale`.
    pub async fn workflow_put_draft(
        &self,
        id: &str,
        name: Option<&str>,
        definition: &Value,
        base_version: i64,
    ) -> Result<Value> {
        self.put(
            &format!("/factory/workflows/{}/draft", encode(id)),
            &serde_json::json!({ "name": name, "definition": definition, "base_version": base_version }),
        )
        .await
    }

    /// `POST …/publish {note, base_version}` → `{version, notified_projects}`.
    pub async fn workflow_publish(&self, id: &str, note: &str, base_version: i64) -> Result<Value> {
        self.post(
            &format!("/factory/workflows/{}/publish", encode(id)),
            &serde_json::json!({ "note": note, "base_version": base_version }),
        )
        .await
    }

    pub async fn workflow_copy(&self, id: &str, name: Option<&str>) -> Result<Workflow> {
        self.post(
            &format!("/factory/workflows/{}/copy", encode(id)),
            &serde_json::json!({ "name": name }),
        )
        .await
    }

    pub async fn workflow_upgrade_preview(
        &self,
        project_id: &str,
        to: Option<i64>,
    ) -> Result<UpgradePreview> {
        let q: Vec<(&str, String)> = to.map(|t| vec![("to", t.to_string())]).unwrap_or_default();
        self.get_query(
            &format!("/factory/projects/{}/workflow-upgrade", encode(project_id)),
            &q,
        )
        .await
    }

    /// `POST …/workflow-upgrade {to_version, preview_hash, mapping?}` → `{version, moved}`.
    pub async fn workflow_upgrade(
        &self,
        project_id: &str,
        to_version: i64,
        preview_hash: &str,
        mapping: Option<&Value>,
    ) -> Result<Value> {
        self.post(
            &format!("/factory/projects/{}/workflow-upgrade", encode(project_id)),
            &serde_json::json!({ "to_version": to_version, "preview_hash": preview_hash, "mapping": mapping }),
        )
        .await
    }

    /// `POST /factory/workflow-templates/{slug}/preview`.
    pub async fn workflow_template_preview(&self, slug: &str) -> Result<Value> {
        self.post(
            &format!("/factory/workflow-templates/{}/preview", encode(slug)),
            &serde_json::json!({}),
        )
        .await
    }
}
