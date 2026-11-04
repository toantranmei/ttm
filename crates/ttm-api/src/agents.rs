//! Agents v2 (S3): profile with handle, bio, git identity, skills and MCP; presets (S0).

use crate::Result;
use crate::client::Client;
use crate::types::TierRef;
use crate::workspace::encode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A skill attached to an agent: `state` is `ok`, `pending` (version waiting for review) or `none`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentSkill {
    #[serde(default)]
    pub skill_id: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub pinned_version: Option<i64>,
    #[serde(default)]
    pub effective_version: Option<i64>,
    #[serde(default)]
    pub state: Option<String>,
}

/// One MCP grant in `profile.mcp`: `server` is a server id or `local:<name>`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentMcp {
    pub server: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
}

/// `GET /workspaces/{id}/agents[]`. Unknown fields are kept in `extra`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// `active`, `paused`, `archived`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub git_name: Option<String>,
    #[serde(default)]
    pub git_email: Option<String>,
    #[serde(default)]
    pub git_email_custom: Option<bool>,
    #[serde(default)]
    pub skills: Vec<AgentSkill>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub profile: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_membership: Option<TierRef>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Agent {
    /// `@handle`, else the name.
    pub fn label(&self) -> String {
        match self.handle.as_deref().filter(|h| !h.is_empty()) {
            Some(h) => format!("@{h}"),
            None => self.name.clone(),
        }
    }

    /// Whether `needle` is this agent's id, handle (with or without `@`) or name.
    pub fn matches(&self, needle: &str) -> bool {
        let n = needle.trim().trim_start_matches('@');
        self.id == n
            || self.handle.as_deref().is_some_and(|h| h.eq_ignore_ascii_case(n))
            || self.name.eq_ignore_ascii_case(n)
    }

    /// MCP grants from `profile.mcp`.
    pub fn mcp(&self) -> Vec<AgentMcp> {
        self.profile
            .as_ref()
            .and_then(|p| p.get("mcp"))
            .and_then(|m| serde_json::from_value(m.clone()).ok())
            .unwrap_or_default()
    }
}

/// `POST /workspaces/{id}/agents` and `PATCH …/{agent}` body (only set fields are sent).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_budget_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_budget_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<String>,
    /// Profile keys to merge (`mcp`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<Value>,
}

/// `PUT /workspaces/{id}/agents/{agent}/skills` item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillPin {
    pub skill_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_version: Option<i64>,
}

/// `GET /factory/agent-presets[]` (S0).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentPreset {
    pub code: String,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub handle: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub project_roles: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub model_upgrade: Option<Value>,
    #[serde(default)]
    pub max_budget_tokens: Option<i64>,
    #[serde(default)]
    pub daily_budget_tokens: Option<i64>,
    #[serde(default)]
    pub max_turns: Option<i64>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// `GET /workspaces/{id}/handles/{handle}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HandleCheck {
    #[serde(default)]
    pub available: bool,
    #[serde(default)]
    pub suggestion: Option<String>,
}

impl Client {
    pub async fn agents(&self, workspace_id: &str) -> Result<Vec<Agent>> {
        self.get(&format!("/workspaces/{}/agents", encode(workspace_id)))
            .await
    }

    pub async fn create_agent(&self, workspace_id: &str, agent: &AgentInput) -> Result<Agent> {
        self.post(&format!("/workspaces/{}/agents", encode(workspace_id)), agent)
            .await
    }

    pub async fn update_agent(
        &self,
        workspace_id: &str,
        agent_id: &str,
        changes: &AgentInput,
    ) -> Result<Agent> {
        self.patch(
            &format!("/workspaces/{}/agents/{}", encode(workspace_id), encode(agent_id)),
            changes,
        )
        .await
    }

    /// Replaces the whole skill list of an agent (≤ 30).
    pub async fn put_agent_skills(
        &self,
        workspace_id: &str,
        agent_id: &str,
        skills: &[SkillPin],
    ) -> Result<Value> {
        self.put(
            &format!(
                "/workspaces/{}/agents/{}/skills",
                encode(workspace_id),
                encode(agent_id)
            ),
            &serde_json::json!({ "skills": skills }),
        )
        .await
    }

    pub async fn handle_check(&self, workspace_id: &str, handle: &str) -> Result<HandleCheck> {
        self.get(&format!(
            "/workspaces/{}/handles/{}",
            encode(workspace_id),
            encode(handle)
        ))
        .await
    }

    /// `GET /workspaces/{id}/agent-options` (MCP servers, local MCP of runners, commit identity).
    pub async fn agent_options(&self, workspace_id: &str) -> Result<Value> {
        self.get(&format!("/workspaces/{}/agent-options", encode(workspace_id)))
            .await
    }

    /// `GET /factory/agent-presets?template=`.
    pub async fn agent_presets(&self, template: Option<&str>) -> Result<Vec<AgentPreset>> {
        let query: Vec<(&str, String)> = template
            .map(|t| vec![("template", t.to_string())])
            .unwrap_or_default();
        self.get_query("/factory/agent-presets", &query).await
    }
}
