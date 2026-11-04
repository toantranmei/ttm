//! Errors: RFC 9457 problem+json from the server, plus transport and protocol failures.

use serde::{Deserialize, Serialize};

/// A problem+json body: `{code, title, status, field?, rule?, params?}`. `code` is stable and meant for branching
/// (`unauthorized`, `insufficient_scope`, `conflict`, `too_many_blocks`, …); `rule` names the check that failed on a
/// `bad_request` / `conflict` / `precondition_failed` (`split-limit`, `rev-conflict`, `approval-required`, …), `field`
/// the input it is about and `params` the values it names (`{"per_run": 10}`, `{"required": "notee:write"}`); `title`
/// is English for humans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Problem {
    #[serde(default)]
    pub status: u16,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Map<String, serde_json::Value>>,
    /// The scope that was missing (`insufficient_scope`): `params.required`, or a top-level `required` from older
    /// servers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<String>,
}

impl Problem {
    /// One param of the rule (`params.per_run`, `params.required`, …).
    pub fn param(&self, key: &str) -> Option<&serde_json::Value> {
        self.params.as_ref()?.get(key)
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({}", self.title, self.code)?;
        if let Some(rule) = &self.rule {
            write!(f, ", rule `{rule}`")?;
        }
        if let Some(field) = &self.field {
            write!(f, ", field `{field}`")?;
        }
        write!(f, ", HTTP {})", self.status)?;
        if let Some(detail) = &self.detail {
            write!(f, ": {detail}")?;
        }
        if let Some(required) = &self.required {
            write!(f, " [requires scope {required}]")?;
        }
        let params: serde_json::Map<String, serde_json::Value> = self
            .params
            .iter()
            .flatten()
            .filter(|(k, _)| !(k.as_str() == "required" && self.required.is_some()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if !params.is_empty() {
            write!(f, " {}", serde_json::Value::Object(params))?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The server answered with an error status.
    #[error("{0}")]
    Problem(Box<Problem>),
    /// HTTP 426: this client is older than the server's `min_cli`.
    #[error("this version of ttm is too old for the server: {message} (run `ttm self-update`)")]
    UpgradeRequired { message: String },
    /// RFC 6749 error from an OAuth endpoint (`authorization_pending`, `slow_down`, `invalid_grant`, …).
    #[error("OAuth error `{error}`{}", description.as_deref().map(|d| format!(": {d}")).unwrap_or_default())]
    OAuth {
        status: u16,
        error: String,
        description: Option<String>,
    },
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected response (HTTP {status}): {message}")]
    Decode { status: u16, message: String },
    #[error("invalid URL `{0}`")]
    InvalidUrl(String),
}

impl ApiError {
    pub fn status(&self) -> Option<u16> {
        match self {
            ApiError::Problem(p) => Some(p.status),
            ApiError::UpgradeRequired { .. } => Some(426),
            ApiError::OAuth { status, .. } | ApiError::Decode { status, .. } => Some(*status),
            ApiError::Http(e) => e.status().map(|s| s.as_u16()),
            ApiError::InvalidUrl(_) => None,
        }
    }

    /// Stable machine code (`problem.code`, the OAuth `error`, or a local one).
    pub fn code(&self) -> &str {
        match self {
            ApiError::Problem(p) => &p.code,
            ApiError::UpgradeRequired { .. } => "upgrade_required",
            ApiError::OAuth { error, .. } => error,
            ApiError::Http(_) => "network",
            ApiError::Decode { .. } => "decode",
            ApiError::InvalidUrl(_) => "invalid_url",
        }
    }

    /// The problem+json body, when the server answered with one.
    pub fn problem(&self) -> Option<&Problem> {
        match self {
            ApiError::Problem(p) => Some(p),
            _ => None,
        }
    }

    /// The `rule` of the problem (`split-limit`, `rev-conflict`, `approval-required`, …).
    pub fn rule(&self) -> Option<&str> {
        self.problem()?.rule.as_deref()
    }

    /// The input the problem is about (`tasks`, `links.repo`, …).
    pub fn field(&self) -> Option<&str> {
        self.problem()?.field.as_deref()
    }

    /// The values the rule names.
    pub fn params(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.problem()?.params.as_ref()
    }

    /// The scope a token lacks (`insufficient_scope`).
    pub fn required_scope(&self) -> Option<&str> {
        self.problem()?.required.as_deref()
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status() == Some(401)
    }

    /// Builds the error for a non-success response body.
    pub(crate) fn from_body(status: u16, content_type: &str, body: &str) -> Self {
        if status == 426 {
            let message = serde_json::from_str::<Problem>(body)
                .ok()
                .and_then(|p| p.detail.or(Some(p.title)).filter(|t| !t.is_empty()))
                .unwrap_or_else(|| short(body).unwrap_or_else(|| "upgrade required".into()));
            return ApiError::UpgradeRequired { message };
        }
        if (content_type.contains("json") || body.trim_start().starts_with('{'))
            && let Ok(mut problem) = serde_json::from_str::<Problem>(body)
            && (!problem.code.is_empty() || !problem.title.is_empty())
        {
            if problem.status == 0 {
                problem.status = status;
            }
            if problem.code.is_empty() {
                problem.code = format!("http_{status}");
            }
            if problem.required.is_none() {
                problem.required = problem
                    .param("required")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned);
            }
            return ApiError::Problem(Box::new(problem));
        }
        let reason = reqwest::StatusCode::from_u16(status)
            .ok()
            .and_then(|s| s.canonical_reason())
            .unwrap_or("Error")
            .to_string();
        ApiError::Problem(Box::new(Problem {
            status,
            code: format!("http_{status}"),
            title: reason,
            detail: short(body),
            field: None,
            rule: None,
            params: None,
            required: None,
        }))
    }
}

fn short(body: &str) -> Option<String> {
    let body = body.trim();
    (!body.is_empty()).then(|| body.chars().take(300).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_problem_json() {
        let body =
            r#"{"code":"insufficient_scope","title":"Missing scope","status":403,"required":"notee:write"}"#;
        let err = ApiError::from_body(403, "application/problem+json", body);
        let ApiError::Problem(p) = &err else {
            panic!("{err:?}")
        };
        assert_eq!(
            (p.code.as_str(), p.status, p.required.as_deref()),
            ("insufficient_scope", 403, Some("notee:write"))
        );
        assert_eq!(err.code(), "insufficient_scope");
        assert!(err.to_string().contains("notee:write"));
    }

    #[test]
    fn insufficient_scope_reads_params_required() {
        let body = r#"{"code":"insufficient_scope","title":"this token lacks the scope factory:write","status":403,"params":{"required":"factory:write"}}"#;
        let err = ApiError::from_body(403, "application/problem+json", body);
        assert_eq!(err.required_scope(), Some("factory:write"));
        assert_eq!(err.params().unwrap()["required"], "factory:write");
        let text = err.to_string();
        assert!(text.contains("[requires scope factory:write]"), "{text}");
        assert!(!text.contains(r#"{"required""#), "printed once: {text}");
    }

    #[test]
    fn rule_field_and_params_are_kept() {
        let body = r#"{"code":"conflict","title":"At most 10 subtasks per run (10 already).","status":409,"rule":"split-limit","params":{"per_run":10,"run_count":10}}"#;
        let err = ApiError::from_body(409, "application/problem+json", body);
        assert_eq!(
            (err.code(), err.rule(), err.field()),
            ("conflict", Some("split-limit"), None)
        );
        assert_eq!(err.params().unwrap()["per_run"], 10);
        let text = err.to_string();
        assert!(
            text.contains("rule `split-limit`") && text.contains("\"per_run\":10"),
            "{text}"
        );

        let body = r#"{"code":"bad_request","title":"tags may have at most 32 characters","status":400,"field":"tags","rule":"too_long","params":{"max":32}}"#;
        let err = ApiError::from_body(400, "application/problem+json", body);
        assert_eq!((err.rule(), err.field()), (Some("too_long"), Some("tags")));
        assert!(err.to_string().contains("field `tags`"), "{err}");
    }

    #[test]
    fn plain_text_becomes_generic_problem() {
        let err = ApiError::from_body(502, "text/html", "<html>bad gateway</html>");
        assert_eq!(err.code(), "http_502");
        assert_eq!(err.status(), Some(502));
    }

    #[test]
    fn upgrade_required_keeps_server_message() {
        let body =
            r#"{"code":"upgrade_required","title":"Upgrade","status":426,"detail":"ttm >= 0.4.0 required"}"#;
        match ApiError::from_body(426, "application/problem+json", body) {
            ApiError::UpgradeRequired { message } => assert_eq!(message, "ttm >= 0.4.0 required"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn unauthorized() {
        let err = ApiError::from_body(
            401,
            "application/json",
            r#"{"code":"unauthorized","title":"x","status":401}"#,
        );
        assert!(err.is_unauthorized());
    }
}
