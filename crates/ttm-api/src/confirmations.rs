//! Two-step confirmations (S7 Task 16): a configuration change is staged with a preview, then applied by
//! `confirm_id` once the person agrees. The server replays the staged request with the caller's own authorization.

use crate::Result;
use crate::client::Client;
use crate::workspace::encode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `POST /confirmations` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewConfirmation {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub body: Value,
    pub summary: String,
    #[serde(default)]
    pub preview: Value,
}

/// `POST /confirmations` answer / `GET /confirmations/{id}`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Confirmation {
    #[serde(default)]
    pub confirm_id: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub preview: Value,
    /// `pending`, `applied`, `cancelled`, `expired`.
    #[serde(default)]
    pub status: Option<String>,
}

/// Routes that can be confirmed (`CONFIRMABLE` of the server), as `(method, path pattern)` with `{}` segments.
pub const CONFIRMABLE: &[(&str, &str)] = &[
    ("PUT", "/factory/workflows/{}/draft"),
    ("POST", "/factory/workflows/{}/publish"),
    ("POST", "/factory/workflows/{}/copy"),
    ("POST", "/factory/projects/{}/workflow-upgrade"),
    ("POST", "/factory/automation/rules"),
    ("PATCH", "/factory/automation/rules/{}"),
    ("POST", "/factory/automation/rules/{}/enabled"),
    ("POST", "/workspaces/{}/agents"),
    ("PATCH", "/workspaces/{}/agents/{}"),
    ("PUT", "/workspaces/{}/agents/{}/skills"),
    ("POST", "/factory/skills/{}/versions/{}/review"),
    ("POST", "/catalog/items/{}/install"),
    ("POST", "/factory/tasks/{}/approve"),
];

/// Whether `(method, path)` matches one of [`CONFIRMABLE`].
pub fn is_confirmable(method: &str, path: &str) -> bool {
    let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
    CONFIRMABLE.iter().any(|(m, pattern)| {
        let pat: Vec<&str> = pattern.trim_matches('/').split('/').collect();
        m.eq_ignore_ascii_case(method)
            && pat.len() == parts.len()
            && pat
                .iter()
                .zip(&parts)
                .all(|(p, s)| *p == "{}" && !s.is_empty() || p == s)
    })
}

impl Client {
    pub async fn confirmation_create(&self, c: &NewConfirmation) -> Result<Confirmation> {
        self.post("/confirmations", c).await
    }

    pub async fn confirmation(&self, id: &str) -> Result<Confirmation> {
        self.get(&format!("/confirmations/{}", encode(id))).await
    }

    /// Applies a confirmation; answers the status and body of the confirmed request (a non-2xx replayed status is
    /// returned as `(status, body)`, not as an error).
    pub async fn confirmation_apply(&self, id: &str) -> Result<(u16, Value)> {
        self.json_raw(
            reqwest::Method::POST,
            &format!("/confirmations/{}/apply", encode(id)),
            &serde_json::json!({}),
            None,
        )
        .await
    }

    pub async fn confirmation_cancel(&self, id: &str) -> Result<Value> {
        self.post(
            &format!("/confirmations/{}/cancel", encode(id)),
            &serde_json::json!({}),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmable_routes() {
        assert!(is_confirmable("POST", "/factory/workflows/w1/publish"));
        assert!(is_confirmable("put", "/workspaces/w/agents/a/skills"));
        assert!(!is_confirmable("DELETE", "/factory/workflows/w1"));
        assert!(!is_confirmable("POST", "/factory/workflows//publish"));
    }
}
