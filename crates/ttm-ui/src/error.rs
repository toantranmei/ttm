//! Errors for people: `✕ <summary>`, `  rule: <rule>  field: <field>`, `  Fix: <how>`, `  Help: <link>`.
//!
//! [`ErrorView::from_anyhow`] reads the [`ttm_api::ApiError`] in the chain (problem+json `rule`, `field`, `status`,
//! `params`) or a local [`UsageError`] (`missing_flag`, `confirmation_required`, `unsupported_feature`, which exit 2).

use crate::Render;
use crate::callout::next_line;
use crate::mode::Mode;
use crate::theme::{Token, paint};
use ttm_api::vocab;

/// A local error about how ttm was called: a missing flag, a confirmation that needs `--yes`, a feature the server
/// lacks. Exits with code 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError {
    pub rule: &'static str,
    pub field: Option<String>,
    pub message: String,
    pub fix: Option<String>,
    pub next: Option<String>,
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UsageError {}

/// Rules that exit with code 2 (usage) rather than 1.
pub const USAGE_RULES: [&str; 4] = [
    "missing_flag",
    "confirmation_required",
    "unsupported_feature",
    "invalid_flag",
];

impl UsageError {
    pub fn new(rule: &'static str, message: impl Into<String>) -> Self {
        Self {
            rule,
            field: None,
            message: message.into(),
            fix: None,
            next: None,
        }
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }

    pub fn next(mut self, next: impl Into<String>) -> Self {
        self.next = Some(next.into());
        self
    }

    /// `flag` has no value and nobody can be asked (`--yes`, or not a terminal).
    pub fn missing_flag(flag: &str, interactive_hint: bool) -> Self {
        let fix = if interactive_hint {
            format!("pass {flag} or run in a terminal")
        } else {
            format!("pass {flag}")
        };
        UsageError::new("missing_flag", format!("missing {flag}"))
            .field(flag)
            .fix(fix)
    }

    /// A confirmation was needed without a terminal and without `--yes`.
    pub fn confirmation_required(what: &str) -> Self {
        UsageError::new("confirmation_required", format!("{what} needs a confirmation"))
            .fix("confirm in a terminal, or pass --yes")
            .next("rerun with --yes")
    }

    /// The server does not have `feature` (from `discovery.features`).
    pub fn unsupported_feature(feature: &str, what: &str) -> Self {
        UsageError::new(
            "unsupported_feature",
            format!("This server does not support {what} yet"),
        )
        .field(feature)
        .fix("update the server, or use a host that has it (ttm status shows the host)")
    }
}

/// What to do about a `rule` (or a problem `code`), in one line.
pub fn hint(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "rev-conflict" => "someone changed it meanwhile: fetch it again (ttm sync) and redo the change",
        "draft-stale" => "the draft changed since you read it: show it again and reapply your edit",
        "upgrade-preview-stale" => "the workflow changed since the preview: run the upgrade preview again",
        "dry-run-required" => "test the rule first: ttm automation test <rule> --task <KEY>",
        "input_required" => "this needs more input: pass the missing value shown in field",
        "stale" => "the change request is out of date: sync again and send a new one",
        "secret_in_file" => "remove the secret from the file and use a secret reference instead",
        "invalid_file" => "fix the file at the path shown (ttm workflow validate --file … lists every issue)",
        "purchase_required" => "buy the template on the web first, then install it again",
        "sponsors_only" => "this template is for Sponsors: see https://toantranmei.com/mei-er",
        "insufficient_scope" => "sign in again with the missing scope: ttm login --scope '<scopes>'",
        "unauthorized" => "sign in again: ttm login",
        "forbidden" => "your role in this workspace does not allow it: ask an owner or admin",
        "not_found" => "check the id or key (list commands show them)",
        "seat_limit" => "the workspace has no free seat: free one or upgrade the plan on the web",
        "plan_limit" => "the plan limit is reached: upgrade the plan on the web",
        "handle_taken" => "pick another --handle",
        "skill_unchanged" => "nothing changed since the last version: edit the skill, then push again",
        "skill_limit" => "too many skills on this agent or workspace: remove one first",
        "self_review" => "someone else must review your own skill version",
        "secrets_disabled" => "secrets are turned off on this server: ask its operator",
        "storage_disabled" => "file storage is turned off on this server: ask its operator",
        "oauth_client_unavailable" => {
            "this MCP server cannot sign in by OAuth: use --auth workspace with a secret"
        }
        "job_token_not_allowed" => "agents running a job cannot do this: run it yourself with your own token",
        "confirm_expired" => "the confirmation expired (10 minutes): start the action again",
        "confirm_stale" => "the request changed since it was confirmed: start the action again",
        "confirm_used" => "this confirmation was already applied: start the action again for a new one",
        "confirm_cancelled" => "this confirmation was cancelled: start the action again",
        "confirm_tampered" => "the staged request does not match its confirmation: start the action again",
        "bearer_required" => "confirmations need a token: use ttm or an MCP client, not the browser session",
        "not_confirmable" => "this change cannot go through a confirmation: run it directly",
        "payload_too_large" => "the request is too large: send a smaller change",
        "token_not_allowed" => "this action is not open to tokens: do it on the web",
        "repo_not_configured" => "link the repository first: ttm project link <KEY>",
        "unsupported_feature" => "this server is older than this ttm: update the server or use another host",
        "skill_hash_mismatch" => "the skill download is corrupt: run the job again",
        "missing_flag" => "pass the flag shown in field, or run in a terminal to be asked",
        "confirmation_required" => "confirm in a terminal, or pass --yes",
        "invalid_flag" => "fix the value of the flag shown in field (ttm <command> --help lists the choices)",
        "split-limit" => "split into fewer subtasks (see params.per_run)",
        "approval-required" => "a person must approve this step: ttm task approve <KEY>",
        "agent-own-runner" => "this agent runs only on its delegator's runners: start one with ttm runner up",
        "no-upgrade" => "the project already uses the latest workflow version",
        "rule-published" => "a published rule cannot change: edit the draft and publish again",
        "too_many_blocks" => "the page is too long for one call: read it by parts (--block)",
        "upgrade_required" => "update ttm: ttm self-update",
        "network" => "check the network and the host (ttm doctor)",
        "rate_limited" | "too_many_requests" => "wait a little and retry",
        "conflict" => "it changed meanwhile: fetch it again and retry",
        "validation" | "bad_request" => "fix the input named in field",
        "authorization_pending" => "finish signing in on the web page, then retry",
        "invalid_grant" | "expired_token" => "sign in again: ttm login",
        _ => return None,
    })
}

/// An error as shown to people.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorView {
    pub summary: String,
    pub rule: Option<String>,
    pub field: Option<String>,
    pub status: Option<u16>,
    pub fix: Option<String>,
    pub next: Option<String>,
}

impl ErrorView {
    pub fn from_anyhow(err: &anyhow::Error) -> Self {
        if let Some(usage) = err.chain().find_map(|e| e.downcast_ref::<UsageError>()) {
            let context = contexts_before(err, |e| e.downcast_ref::<UsageError>().is_some());
            return ErrorView {
                summary: join(context, &usage.message),
                rule: Some(usage.rule.to_string()),
                field: usage.field.clone(),
                status: None,
                fix: usage.fix.clone().or_else(|| hint(usage.rule).map(String::from)),
                next: usage.next.clone(),
            };
        }
        if let Some(api) = err.chain().find_map(|e| e.downcast_ref::<ttm_api::ApiError>()) {
            let context = contexts_before(err, |e| e.downcast_ref::<ttm_api::ApiError>().is_some());
            let (message, rule) = match api.problem() {
                Some(p) => {
                    let mut message = if p.title.is_empty() {
                        p.code.clone()
                    } else {
                        p.title.clone()
                    };
                    if let Some(detail) = p.detail.as_deref().filter(|d| !d.is_empty()) {
                        message = format!("{message}: {detail}");
                    }
                    (
                        message,
                        p.rule
                            .clone()
                            .or_else(|| Some(p.code.clone()).filter(|c| !c.is_empty())),
                    )
                }
                None => (api.to_string(), Some(api.code().to_string())),
            };
            let rule = rule.filter(|r| r != "error");
            let mut fix = rule.as_deref().and_then(hint).map(String::from);
            if let Some(scope) = api.required_scope().or_else(|| {
                api.params()
                    .and_then(|p| p.get("required"))
                    .and_then(|v| v.as_str())
            }) {
                fix = Some(format!(
                    "sign in again with the scope {scope}: ttm login --scope '<scopes> {scope}'"
                ));
            }
            return ErrorView {
                summary: join(context, &message),
                rule,
                field: api.field().map(String::from),
                status: api.status(),
                fix,
                next: None,
            };
        }
        ErrorView {
            summary: format!("{err:#}"),
            rule: None,
            field: None,
            status: None,
            fix: None,
            next: None,
        }
    }

    /// Process exit code: 2 for usage errors, else 1.
    pub fn exit_code(&self) -> u8 {
        match self.rule.as_deref() {
            Some(rule) if USAGE_RULES.contains(&rule) => 2,
            _ => 1,
        }
    }

    pub fn help_url(&self) -> Option<String> {
        self.rule.as_deref().map(vocab::help_url)
    }
}

/// The context messages above the first error matching `is_source`, outermost first.
fn contexts_before(
    err: &anyhow::Error,
    is_source: impl Fn(&(dyn std::error::Error + 'static)) -> bool,
) -> Vec<String> {
    err.chain()
        .take_while(|e| !is_source(*e))
        .map(|e| e.to_string())
        .collect()
}

fn join(mut context: Vec<String>, message: &str) -> String {
    context.push(message.to_string());
    context.join(": ")
}

impl Render for ErrorView {
    fn render(&self, mode: &Mode) -> String {
        let lead = if mode.plain {
            "ERROR:".to_string()
        } else {
            paint(mode, Token::Coral, "✕")
        };
        let mut lines = vec![format!("{lead} {}", self.summary)];
        let mut tags = Vec::new();
        if let Some(rule) = &self.rule {
            tags.push(format!("{} {rule}", paint(mode, Token::Muted, "rule:")));
        }
        if let Some(field) = &self.field {
            tags.push(format!("{} {field}", paint(mode, Token::Muted, "field:")));
        }
        if !tags.is_empty() {
            lines.push(format!("  {}", tags.join("  ")));
        }
        if let Some(fix) = &self.fix {
            lines.push(format!("  {} {fix}", paint(mode, Token::Muted, "Fix:")));
        }
        if let Some(url) = self.help_url() {
            lines.push(format!(
                "  {} {}",
                paint(mode, Token::Muted, "Help:"),
                paint(mode, Token::Blue, &url)
            ));
        }
        if let Some(next) = &self.next {
            lines.push(next_line(mode, next));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::ColorLevel;

    fn problem(rule: Option<&str>) -> anyhow::Error {
        let problem = ttm_api::Problem {
            status: 409,
            code: "conflict".into(),
            title: "AUR-12 changed since rev 4.".into(),
            detail: None,
            field: Some("rev".into()),
            rule: rule.map(String::from),
            params: None,
            required: None,
        };
        anyhow::Error::new(ttm_api::ApiError::Problem(Box::new(problem))).context("updating AUR-12")
    }

    #[test]
    fn api_problem() {
        let v = ErrorView::from_anyhow(&problem(Some("rev-conflict")));
        assert_eq!(v.summary, "updating AUR-12: AUR-12 changed since rev 4.");
        assert_eq!(v.rule.as_deref(), Some("rev-conflict"));
        assert_eq!(v.field.as_deref(), Some("rev"));
        assert!(v.fix.is_some());
        assert_eq!(v.exit_code(), 1);
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        let text = v.render(&mode);
        assert!(text.starts_with("✕ updating AUR-12"), "{text}");
        assert!(text.contains("  rule: rev-conflict  field: rev"), "{text}");
        assert!(
            text.contains("Help: https://toantranmei.com/help/ttm/ma-loi#rev-conflict"),
            "{text}"
        );
        // Without a rule, the problem code stands in.
        assert_eq!(
            ErrorView::from_anyhow(&problem(None)).rule.as_deref(),
            Some("conflict")
        );
    }

    #[test]
    fn usage_errors_exit_2() {
        let err = anyhow::Error::new(UsageError::missing_flag("--workflow", true));
        let v = ErrorView::from_anyhow(&err);
        assert_eq!(
            (v.rule.as_deref(), v.field.as_deref()),
            (Some("missing_flag"), Some("--workflow"))
        );
        assert_eq!(v.fix.as_deref(), Some("pass --workflow or run in a terminal"));
        assert_eq!(v.exit_code(), 2);
        let plain = Mode::fixed(ColorLevel::None, true, 80);
        assert!(v.render(&plain).starts_with("ERROR: missing --workflow"));
    }

    #[test]
    fn plain_errors() {
        let v = ErrorView::from_anyhow(&anyhow::anyhow!("boom"));
        assert_eq!((v.summary.as_str(), v.rule.as_deref()), ("boom", None));
        let mode = Mode::fixed(ColorLevel::None, false, 80);
        assert_eq!(v.render(&mode), "✕ boom");
    }

    #[test]
    fn hint_table_is_large() {
        let rules = [
            "rev-conflict",
            "draft-stale",
            "upgrade-preview-stale",
            "dry-run-required",
            "input_required",
            "stale",
            "secret_in_file",
            "purchase_required",
            "sponsors_only",
            "insufficient_scope",
            "seat_limit",
            "plan_limit",
            "handle_taken",
            "skill_unchanged",
            "skill_limit",
            "self_review",
            "secrets_disabled",
            "storage_disabled",
            "oauth_client_unavailable",
            "job_token_not_allowed",
            "confirm_expired",
            "confirm_stale",
            "repo_not_configured",
            "unsupported_feature",
            "skill_hash_mismatch",
            "missing_flag",
            "confirmation_required",
            "split-limit",
            "approval-required",
            "agent-own-runner",
            "no-upgrade",
            "rule-published",
            "too_many_blocks",
            "upgrade_required",
            "network",
            "conflict",
            "unauthorized",
            "forbidden",
            "not_found",
            "invalid_file",
        ];
        assert!(rules.len() >= 40);
        for rule in rules {
            assert!(hint(rule).is_some(), "{rule}");
        }
    }
}
