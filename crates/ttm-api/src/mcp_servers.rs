//! Remote MCP servers of a workspace (S3), reached by agents through the server's MCP gateway.

use crate::Result;
use crate::client::Client;
use crate::workspace::encode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `GET /factory/mcp-servers[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    /// `http` or `sse`.
    #[serde(default)]
    pub transport: Option<String>,
    /// `none`, `workspace`, `delegator`.
    #[serde(default)]
    pub auth: Option<String>,
    /// `active`, `disabled`, `error`, `needs_auth`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub tools: Vec<Value>,
    #[serde(default)]
    pub connected: McpConnected,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct McpConnected {
    #[serde(default)]
    pub workspace: bool,
    #[serde(default)]
    pub me: bool,
}

impl McpServer {
    /// Names of the cached tools (`tools[]` items are strings or `{name}`).
    pub fn tool_names(&self) -> Vec<String> {
        self.tools
            .iter()
            .filter_map(|t| match t {
                Value::String(s) => Some(s.clone()),
                Value::Object(o) => o.get("name").and_then(Value::as_str).map(String::from),
                _ => None,
            })
            .collect()
    }
}

/// `{header, value}`: the secret header sent by the gateway.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticSecret {
    pub header: String,
    pub value: String,
}

/// `POST /factory/mcp-servers` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewMcpServer {
    pub workspace_id: String,
    pub name: String,
    pub url: String,
    pub transport: String,
    pub auth: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_secret: Option<StaticSecret>,
}

impl Client {
    pub async fn mcp_servers(&self, workspace_id: &str) -> Result<Vec<McpServer>> {
        self.get_query(
            "/factory/mcp-servers",
            &[("workspace_id", workspace_id.to_string())],
        )
        .await
    }

    pub async fn create_mcp_server(&self, server: &NewMcpServer) -> Result<McpServer> {
        self.post("/factory/mcp-servers", server).await
    }

    pub async fn delete_mcp_server(&self, id: &str) -> Result<()> {
        self.delete(&format!("/factory/mcp-servers/{}", encode(id))).await
    }

    pub async fn refresh_mcp_tools(&self, id: &str) -> Result<Value> {
        self.post(
            &format!("/factory/mcp-servers/{}/refresh-tools", encode(id)),
            &serde_json::json!({}),
        )
        .await
    }

    /// `POST /factory/mcp-servers/{id}/connect {for}` → the OAuth `authorize_url` to open in a browser.
    pub async fn mcp_connect(&self, id: &str, for_: &str) -> Result<String> {
        let v: Value = self
            .post(
                &format!("/factory/mcp-servers/{}/connect", encode(id)),
                &serde_json::json!({ "for": for_ }),
            )
            .await?;
        Ok(v.get("authorize_url")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    pub async fn mcp_disconnect(&self, id: &str, for_: &str) -> Result<()> {
        self.delete_query(
            &format!("/factory/mcp-servers/{}/connection", encode(id)),
            &[("for", for_.to_string())],
        )
        .await
    }
}
