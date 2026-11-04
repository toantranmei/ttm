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

/// `POST /confirmations` answer (201 `{confirm_id, expires_at}`) / `GET /confirmations/{id}` / cancel answer
/// (`{confirm_id, status}`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Confirmation {
    #[serde(default)]
    pub confirm_id: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub preview: Value,
    /// `pending`, `applied`, `cancelled`, `expired`.
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub used_at: Option<String>,
    /// HTTP status the applied action answered.
    #[serde(default)]
    pub result_status: Option<u16>,
}

/// Header the server puts on the replayed answer of `POST /confirmations/{id}/apply`.
pub const APPLIED_HEADER: &str = "x-confirmation";

/// Errors of the confirmation step itself (not of the confirmed action).
pub const CONFIRM_ERRORS: &[&str] = &[
    "not_found",
    "confirm_expired",
    "confirm_used",
    "confirm_cancelled",
    "confirm_tampered",
    "bearer_required",
    "job_token_not_allowed",
    "not_confirmable",
    "payload_too_large",
    "token_not_allowed",
    "insufficient_scope",
    "rate_limited",
];

/// Answer of `POST /confirmations/{id}/apply`.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyOutcome {
    pub status: u16,
    pub body: Value,
    /// `X-Confirmation: applied`: the status and body are those of the confirmed action (e.g. 409 `draft-stale`);
    /// without it an error belongs to the confirmation step (`confirm_expired`, `confirm_used`, …).
    pub applied: bool,
}

impl ApplyOutcome {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// `code` (or `rule`) of an error body.
    pub fn code(&self) -> Option<&str> {
        self.body
            .get("rule")
            .or_else(|| self.body.get("code"))
            .and_then(Value::as_str)
    }
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

    /// Applies a confirmation. Error statuses come back as data: with `applied` they belong to the action, else to
    /// the confirmation step.
    pub async fn confirmation_apply(&self, id: &str) -> Result<ApplyOutcome> {
        let (status, body, headers) = self
            .json_raw_headers(
                reqwest::Method::POST,
                &format!("/confirmations/{}/apply", encode(id)),
                &serde_json::json!({}),
            )
            .await?;
        let applied = headers
            .get(APPLIED_HEADER)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("applied"));
        Ok(ApplyOutcome {
            status,
            body,
            applied,
        })
    }

    pub async fn confirmation_cancel(&self, id: &str) -> Result<Confirmation> {
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
