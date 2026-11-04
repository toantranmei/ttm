//! Automation rules (S2): "when … if … then …", dry runs and run history.

use crate::Result;
use crate::client::Client;
use crate::workspace::encode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `GET /factory/automation/rules[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: String,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub trigger: Value,
    #[serde(default)]
    pub conditions: Value,
    #[serde(default)]
    pub actions: Vec<Value>,
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub builtin: Option<bool>,
    #[serde(default)]
    pub runs_7d: Option<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl AutomationRule {
    pub fn matches(&self, needle: &str) -> bool {
        self.id == needle || self.slug.as_deref() == Some(needle) || self.name.eq_ignore_ascii_case(needle)
    }
}

/// One-line summary of a trigger/condition/action object: its `type`/`kind`/`event` plus scalar params.
pub fn summarize(v: &Value) -> String {
    match v {
        Value::Object(o) => {
            let head = ["type", "kind", "event", "action"]
                .iter()
                .find_map(|k| o.get(*k).and_then(Value::as_str))
                .unwrap_or("?")
                .to_string();
            let params: Vec<String> = o
                .iter()
                .filter(|(k, _)| !["type", "kind", "event", "action"].contains(&k.as_str()))
                .filter_map(|(k, v)| match v {
                    Value::String(s) => Some(format!("{k}={s}")),
                    Value::Number(n) => Some(format!("{k}={n}")),
                    Value::Bool(b) => Some(format!("{k}={b}")),
                    _ => None,
                })
                .collect();
            if params.is_empty() {
                head
            } else {
                format!("{head} ({})", params.join(", "))
            }
        }
        Value::String(s) => s.clone(),
        Value::Null => "-".into(),
        other => other.to_string(),
    }
}

/// `POST /factory/automation/rules/dry-run` answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DryRun {
    #[serde(default)]
    pub matched: bool,
    #[serde(default)]
    pub trigger: Value,
    #[serde(default)]
    pub conditions: Value,
    #[serde(default)]
    pub actions: Vec<Value>,
}

/// `GET /factory/automation/runs[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleRun {
    pub id: String,
    #[serde(default)]
    pub rule_id: String,
    #[serde(default)]
    pub task: Option<Value>,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub log: Value,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Client {
    /// Rules of a project (or of a workflow version with `workflow_id`).
    pub async fn automation_rules(
        &self,
        project_id: Option<&str>,
        workflow_id: Option<&str>,
    ) -> Result<Vec<AutomationRule>> {
        let mut q = Vec::new();
        if let Some(p) = project_id {
            q.push(("project_id", p.to_string()));
        }
        if let Some(w) = workflow_id {
            q.push(("workflow_id", w.to_string()));
        }
        self.get_query("/factory/automation/rules", &q).await
    }

    pub async fn automation_rule(&self, id: &str) -> Result<AutomationRule> {
        self.get(&format!("/factory/automation/rules/{}", encode(id)))
            .await
    }

    pub async fn automation_set_enabled(&self, id: &str, enabled: bool) -> Result<AutomationRule> {
        self.post(
            &format!("/factory/automation/rules/{}/enabled", encode(id)),
            &serde_json::json!({ "enabled": enabled }),
        )
        .await
    }

    /// `POST /factory/automation/rules/dry-run {rule_id, project_id, task, event?}`.
    pub async fn automation_test(&self, rule_id: &str, project_id: &str, task: &str) -> Result<DryRun> {
        self.post(
            "/factory/automation/rules/dry-run",
            &serde_json::json!({ "rule_id": rule_id, "project_id": project_id, "task": task }),
        )
        .await
    }

    pub async fn automation_runs(
        &self,
        rule_id: Option<&str>,
        project_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<RuleRun>> {
        let mut q = vec![("limit", limit.to_string())];
        if let Some(r) = rule_id {
            q.push(("rule_id", r.to_string()));
        }
        if let Some(p) = project_id {
            q.push(("project_id", p.to_string()));
        }
        self.get_query("/factory/automation/runs", &q).await
    }
}
