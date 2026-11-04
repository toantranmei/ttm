//! Public template library (S5): search the catalog, export and publish, install previews and installs.

use crate::Result;
use crate::client::Client;
use crate::workspace::encode;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `GET /catalog[]` item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tagline: Option<String>,
    #[serde(default)]
    pub price: Option<Value>,
    #[serde(default)]
    pub author: Option<Value>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `POST /catalog/items/{id}/install-preview` answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InstallPreview {
    #[serde(default)]
    pub creates: InstallCreates,
    #[serde(default)]
    pub inputs: Vec<Value>,
    #[serde(default)]
    pub requires_purchase: bool,
    #[serde(default)]
    pub owned: bool,
    #[serde(default)]
    pub runs_code: bool,
    #[serde(default)]
    pub checked: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InstallCreates {
    #[serde(default)]
    pub workflow: Option<Value>,
    #[serde(default)]
    pub rules: Vec<Value>,
    #[serde(default)]
    pub agents: Vec<Value>,
    #[serde(default)]
    pub skills: Vec<Value>,
}

/// `{kind, id}` source of an export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateSource {
    pub kind: String,
    pub id: String,
}

/// `POST /factory/templates/export-preview` answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExportPreview {
    #[serde(default)]
    pub kind_suggested: Option<String>,
    #[serde(default)]
    pub files: Vec<Value>,
    #[serde(default)]
    pub removed: Vec<Value>,
    #[serde(default)]
    pub inputs: Vec<Value>,
    #[serde(default)]
    pub runs_code: bool,
    #[serde(default)]
    pub digest: String,
}

impl Client {
    /// `GET /catalog?kind=a,b&q`.
    pub async fn catalog(&self, kinds: &[&str], query: Option<&str>) -> Result<Vec<Template>> {
        let mut q = Vec::new();
        if !kinds.is_empty() {
            q.push(("kind", kinds.join(",")));
        }
        if let Some(v) = query {
            q.push(("q", v.to_string()));
        }
        let v: Value = self.get_query("/catalog", &q).await?;
        // Either a bare array or `{items: […]}`.
        let items = v.get("items").cloned().unwrap_or(v);
        serde_json::from_value(items).map_err(|e| crate::ApiError::Decode {
            status: 200,
            message: e.to_string(),
        })
    }

    pub async fn template_export_preview(
        &self,
        workspace_id: &str,
        sources: &[TemplateSource],
    ) -> Result<ExportPreview> {
        self.post(
            "/factory/templates/export-preview",
            &serde_json::json!({ "workspace_id": workspace_id, "sources": sources }),
        )
        .await
    }

    /// `POST /factory/templates/publish` → `{item_id, version, url}`.
    pub async fn template_publish(
        &self,
        workspace_id: &str,
        sources: &[TemplateSource],
        digest: &str,
        item: &Value,
        notes: Option<&str>,
    ) -> Result<Value> {
        self.post(
            "/factory/templates/publish",
            &serde_json::json!({ "workspace_id": workspace_id, "sources": sources, "digest": digest,
                                  "item": item, "notes": notes }),
        )
        .await
    }

    pub async fn template_install_preview(
        &self,
        item_id: &str,
        workspace_id: &str,
        version: Option<i64>,
    ) -> Result<InstallPreview> {
        self.post(
            &format!("/catalog/items/{}/install-preview", encode(item_id)),
            &serde_json::json!({ "workspace_id": workspace_id, "version": version }),
        )
        .await
    }

    pub async fn template_install(
        &self,
        item_id: &str,
        workspace_id: &str,
        version: Option<i64>,
        inputs: &Value,
        agents: &Value,
    ) -> Result<Value> {
        self.post(
            &format!("/catalog/items/{}/install", encode(item_id)),
            &serde_json::json!({ "workspace_id": workspace_id, "version": version, "inputs": inputs, "agents": agents }),
        )
        .await
    }
}
