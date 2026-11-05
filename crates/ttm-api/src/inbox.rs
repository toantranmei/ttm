//! Notifications (S0 §8): filtered by kind, unread and `since`, for `ttm inbox` and `factory-inbox`.

use crate::Result;
use crate::client::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `GET /notifications[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub read_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub data: Value,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Notification {
    pub fn unread(&self) -> bool {
        self.read_at.is_none() && !self.extra.get("read").and_then(Value::as_bool).unwrap_or(false)
    }

    /// `data.reason` of Factory notifications (`skill_review`, `approval_request`, …), else the kind.
    pub fn reason(&self) -> &str {
        self.data
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or(&self.kind)
    }

    pub fn message(&self) -> &str {
        self.text.as_deref().or(self.title.as_deref()).unwrap_or("")
    }
}

/// Filter of `GET /notifications`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InboxQuery {
    pub kinds: Vec<String>,
    pub unread: bool,
    /// RFC 3339: only `created_at > since`.
    pub since: Option<String>,
}

impl Client {
    pub async fn notifications(&self, query: &InboxQuery) -> Result<Vec<Notification>> {
        let mut q = Vec::new();
        if !query.kinds.is_empty() {
            q.push(("kind", query.kinds.join(",")));
        }
        if query.unread {
            q.push(("unread", "1".to_string()));
        }
        if let Some(since) = &query.since {
            q.push(("since", since.clone()));
        }
        let v: Value = self.get_query("/notifications", &q).await?;
        let items = v
            .get("items")
            .or_else(|| v.get("notifications"))
            .cloned()
            .unwrap_or(v);
        serde_json::from_value(items).map_err(|e| crate::ApiError::Decode {
            status: 200,
            message: e.to_string(),
        })
    }

    pub async fn unread_count(&self, kinds: &[&str]) -> Result<i64> {
        let q: Vec<(&str, String)> = if kinds.is_empty() {
            vec![]
        } else {
            vec![("kind", kinds.join(","))]
        };
        let v: Value = self.get_query("/notifications/unread-count", &q).await?;
        Ok(v.get("count").and_then(Value::as_i64).unwrap_or(0))
    }

    /// Marks notifications read (`ids` empty = all).
    pub async fn notifications_read(&self, ids: &[String]) -> Result<Value> {
        let body = if ids.is_empty() {
            serde_json::json!({ "all": true })
        } else {
            serde_json::json!({ "ids": ids })
        };
        self.post("/notifications/read", &body).await
    }
}
