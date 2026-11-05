//! A backend answering every `/api/v1` call from a fixed table, recording calls.

#![allow(dead_code)]

use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Mutex;
use ttm_api::{MeUser, Member, MemberUser, Membership, TierRef, WorkspaceSummary};
use ttm_mcp_tools::{Backend, BackendError, FactoryScope, SelfInfo};

#[derive(Default)]
pub struct Fake {
    pub run_id: Option<String>,
    pub calls: Mutex<Vec<String>>,
    /// Bodies sent, with their `METHOD path`.
    pub bodies: Mutex<Vec<(String, Value)>>,
}

impl Fake {
    pub fn job() -> Self {
        Self {
            run_id: Some("run1".into()),
            ..Self::default()
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    /// The body of the last `call` (`METHOD path`).
    pub fn body(&self, call: &str) -> Option<Value> {
        self.bodies
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(c, _)| c == call)
            .map(|(_, b)| b.clone())
    }
}

fn ws() -> WorkspaceSummary {
    WorkspaceSummary {
        id: "w1".into(),
        name: "Team Aurora".into(),
        kind: "team".into(),
        role: "owner".into(),
        slug: Some("aurora".into()),
        owner_membership: Some(TierRef {
            tier: "mei-er".into(),
        }),
    }
}

pub fn answer(method: &str, path: &str) -> Option<Value> {
    Some(match (method, path) {
        ("GET", "/factory/projects") => json!([{"id": "p1", "key": "AUR", "name": "Aurora"}]),
        ("GET", "/factory/projects/p1") => {
            json!({"id": "p1", "key": "AUR", "name": "Aurora", "workflow_id": "wf1"})
        }
        ("GET", "/factory/workflows/wf1") => json!({
            "id": "wf1", "name": "Professional", "latest_version": 2, "version": 2, "has_draft": true,
            "definition": {"states": [
                {"id": "ready", "name": "Ready", "category": "ready", "role": "PM"},
                {"id": "doing", "name": "Doing", "category": "in-progress", "role": "DEV", "auto_assign": true, "wip": 3},
                {"id": "qc", "name": "QC", "category": "qc", "role": "QC"}
            ], "transitions": [{"id": "t1", "from": ["ready"], "to": "doing", "rules": [{"slug": "r"}]}]},
            "draft": {"definition": {"states": [{"id": "done", "name": "Done", "category": "done"}], "transitions": []}}
        }),
        ("GET", "/factory/workflows/wf1/diff") => {
            json!({"states": {"added": [{"id": "qc2", "name": "QC 2"}], "removed": [], "changed": []},
            "affected_rules": [{"id": "r9", "slug": "qc-notify", "name": "QC → notify", "scope": "project", "project_key": "AUR",
                "field": "trigger.to", "state": "qc", "reason": "state_renamed", "new_state": "qc2"}]})
        }
        ("GET", "/factory/projects/p1/workflow-upgrade") => json!({
            "workflow_id": "wf1", "name": "Professional", "from_version": 2, "to_version": 3, "preview_hash": "h1",
            "diff": {"states": {"added": [], "removed": [{"id": "qc", "name": "QC"}], "changed": []}},
            "remap": [], "counts": {"tasks_total": 0, "tasks_moved": 0}, "affected_rules": []
        }),
        ("POST", "/confirmations") => json!({"confirm_id": "c1", "expires_at": "2026-11-05T10:10:00Z"}),
        ("GET", "/confirmations/c1") => {
            json!({"confirm_id": "c1", "summary": "Publish workflow Professional v3", "status": "pending"})
        }
        ("POST", "/confirmations/c1/apply") => json!({"version": 3, "notified_projects": 1}),
        ("POST", "/confirmations/c1/cancel") => json!({"confirm_id": "c1", "status": "cancelled"}),
        ("GET", "/workspaces/w1/agents") => json!([
            {"id": "a1", "name": "DEV · Code", "handle": "dev", "role": "DEV", "model": "sonnet", "status": "active",
             "skills": [{"skill_id": "s1", "slug": "ttm-tdd", "state": "ok"}, {"skill_id": "s2", "slug": "ttm-pr", "state": "pending"}],
             "profile": {"mcp": [{"server": "local:github-local", "tools": ["*"]}]}, "owner_membership": {"tier": "sponsor"}}
        ]),
        ("GET", "/factory/automation/rules") => json!([
            {"id": "r1", "slug": "auto-assign", "name": "Auto assign", "enabled": true, "trigger": {"type": "task.moved", "to": "doing"}, "actions": [{"type": "assign", "role": "DEV"}], "runs_7d": 4}
        ]),
        ("GET", "/factory/skills/ttm-tdd") => {
            json!({"id": "s1", "slug": "ttm-tdd", "name": "TDD", "current_version": 3,
            "versions": [{"version": 3, "review_status": "approved"}, {"version": 4, "review_status": "pending"}]})
        }
        ("GET", "/factory/skills/s1/versions/3/files/SKILL.md") => {
            json!({"text": "---\nname: TDD\n---\nWrite the test first."})
        }
        ("GET", "/notifications") => {
            json!([{"id": "n1", "kind": "factory", "text": "Skill ttm-tdd v4 waiting for review", "created_at": "2026-11-05T10:00:00Z", "data": {"reason": "skill_review"}}])
        }
        ("GET", "/catalog") => {
            json!({"items": [{"id": "i1", "slug": "web-todo", "kind": "workflow", "name": "Web todo"}]})
        }
        ("GET", "/factory/tasks/AUR-12") => {
            json!({"id": "AUR-12", "key": "AUR-12", "title": "Login", "state": "Approval", "category": "approval"})
        }
        _ => return None,
    })
}

#[async_trait]
impl Backend for Fake {
    async fn get_self(&self) -> Result<SelfInfo, BackendError> {
        Ok(SelfInfo {
            user: MeUser {
                id: "u1".into(),
                email: "an@example.com".into(),
                name: "An".into(),
                avatar_url: None,
                locale: None,
                plan: Some("free".into()),
                membership: Some(Membership {
                    tier: "mei-er".into(),
                    status: Some("active".into()),
                    since: Some("2025-11-05T00:00:00Z".into()),
                    until: Some("2026-12-05T00:00:00Z".into()),
                    badge: Some("✦".into()),
                }),
            },
            workspace: Some(ws()),
        })
    }

    async fn list_workspaces(&self) -> Result<Vec<WorkspaceSummary>, BackendError> {
        Ok(vec![ws()])
    }

    async fn get_users(&self, _workspace_id: Option<&str>) -> Result<Vec<Member>, BackendError> {
        Ok(vec![Member {
            user: MemberUser {
                id: "u1".into(),
                name: "An".into(),
                email: "an@example.com".into(),
                avatar_url: None,
                membership: None,
            },
            role: "owner".into(),
            membership: Some(TierRef {
                tier: "sponsor".into(),
            }),
        }])
    }

    async fn factory_get_task(&self, _task: Option<&str>) -> Result<ttm_api::factory::Task, BackendError> {
        Ok(serde_json::from_value(json!({
            "id": "AUR-12", "key": "AUR-12", "title": "Login with Google", "state": "Doing", "category": "in-progress",
            "role": "DEV", "estimate": 3,
            "assigneeMember": {"kind": "agent", "id": "a1", "name": "DEV · Code", "ownerMembership": {"tier": "mei-er"}}
        }))
        .unwrap())
    }

    async fn api(
        &self,
        method: &str,
        path: &str,
        _query: &[(String, String)],
        body: Option<Value>,
    ) -> Result<Value, BackendError> {
        self.calls.lock().unwrap().push(format!("{method} {path}"));
        if let Some(body) = body {
            self.bodies
                .lock()
                .unwrap()
                .push((format!("{method} {path}"), body));
        }
        answer(method, path).ok_or_else(|| BackendError::new("not_found", format!("{method} {path}")))
    }

    fn factory_scope(&self) -> FactoryScope {
        FactoryScope {
            run_id: self.run_id.clone(),
            task: None,
            project: Some("AUR".into()),
        }
    }
}

pub fn args(v: Value) -> Option<serde_json::Map<String, Value>> {
    v.as_object().cloned()
}

pub fn text(result: &rmcp::model::CallToolResult) -> String {
    result.content[0]
        .as_text()
        .map(|t| t.text.clone())
        .unwrap_or_default()
}
