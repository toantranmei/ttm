//! What the runner says to people, through `ttm-ui`: one line per run start, end and refusal, and one line per event
//! (`ttm agent run`, `ttm agent logs`).
//!
//! ```text
//! ● AUR-12 running · sonnet · worktree ~/orca/workspaces/aurora/aur-12
//! ✓ AUR-12 succeeded → https://github.com/a/b/pull/7
//! ✕ AUR-12 refused: repo_not_configured
//! ```

use crate::executor::JobOutcome;
use ttm_api::factory::{JobSpec, RunEvent};
use ttm_api::vocab::Phase;
use ttm_ui::{Status, Token, Ui};

/// `● AUR-12 running · sonnet · worktree <path>`.
pub fn started(ui: &Ui, job: &JobSpec, worktree: &std::path::Path) -> String {
    let mut parts = vec![format!("{} running", ui.paint(Token::Ink, &job.task_key))];
    if let Some(model) = job.model.as_deref().filter(|m| !m.is_empty()) {
        parts.push(model.to_string());
    }
    parts.push(format!("worktree {}", worktree.display()));
    format!(
        "{} {}",
        ui.render(&Status::phase(Phase::Doing)),
        parts.join(" · ")
    )
}

/// `✓ AUR-12 succeeded → <PR>` / `✕ AUR-12 failed` / `⏸ AUR-12 paused until …`.
pub fn finished(ui: &Ui, task_key: &str, outcome: &JobOutcome) -> String {
    let phase = match outcome.status.as_str() {
        "succeeded" => Phase::Done,
        _ if outcome.paused_until.is_some() => Phase::Paused,
        "cancelled" | "canceled" => Phase::Paused,
        _ => Phase::Blocked,
    };
    let mut line = format!(
        "{} {} {}",
        ui.render(&Status::phase(phase)),
        ui.paint(Token::Ink, task_key),
        outcome.status
    );
    if let Some(pr) = &outcome.pr_url {
        line.push_str(&format!(" → {}", ui.paint(Token::Blue, pr)));
    } else if let Some(until) = &outcome.paused_until {
        line.push_str(&format!(" until {until}"));
    }
    line
}

/// `✕ AUR-12 refused: <code>: <message>`.
pub fn refused(ui: &Ui, task_key: &str, code: &str, message: &str) -> String {
    format!(
        "{} {} refused: {} {}",
        ui.render(&Status::phase(Phase::Blocked)),
        ui.paint(Token::Ink, task_key),
        ui.paint(Token::Coral, code),
        ui.paint(Token::Muted, message)
    )
}

/// Glyph and colour of an event kind (`--plain`: `[kind]`).
fn mark(ui: &Ui, event: &RunEvent) -> (String, Option<Token>) {
    let plain = ui.mode().plain;
    let (glyph, token) = match event.kind.as_str() {
        "thought" => ("·", Some(Token::Muted)),
        "action" => ("›", Some(Token::Blue)),
        "mcp_call" => ("◆", Some(Token::Purple)),
        "error" => ("✕", Some(Token::Coral)),
        "plan" => ("○", None),
        "approval_request" => ("◐", Some(Token::Yellow)),
        "response" => ("✓", Some(Token::Teal)),
        "usage" => ("$", Some(Token::Muted)),
        "rule_check" => {
            let passed = event
                .data
                .as_ref()
                .and_then(|d| d.get("passed"))
                .and_then(|p| p.as_bool())
                .unwrap_or(true);
            if passed {
                ("✓", Some(Token::Teal))
            } else {
                ("✕", Some(Token::Coral))
            }
        }
        "progress" => {
            let pct = event
                .data
                .as_ref()
                .and_then(|d| d.get("percent").or_else(|| d.get("pct")))
                .and_then(|p| p.as_f64());
            return (
                match (pct, plain) {
                    (Some(p), _) => format!("{:>3.0}%", p),
                    (None, true) => "[progress]".into(),
                    (None, false) => "%".into(),
                },
                Some(Token::Teal),
            );
        }
        _ => ("·", None),
    };
    if plain {
        (format!("[{}]", event.kind), None)
    } else {
        (ui.paint(token.unwrap_or(Token::Ink), glyph), token)
    }
}

/// `10:02:03 › edited src/login.rs` (time only when the event has one).
pub fn event_line(ui: &Ui, event: &RunEvent) -> String {
    let (glyph, token) = mark(ui, event);
    let time = event
        .at
        .as_deref()
        .and_then(|at| at.get(11..19))
        .map(|t| format!("{} ", ui.paint(Token::Muted, t)))
        .unwrap_or_default();
    let message = event.message.replace('\n', " ");
    let message = match token {
        Some(Token::Muted) | Some(Token::Coral) => ui.paint(token.unwrap_or(Token::Ink), &message),
        _ => message,
    };
    format!("{time}{glyph} {message}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttm_ui::{ColorLevel, Mode};

    fn ui(plain: bool) -> Ui {
        Ui::buffered(Mode::fixed(ColorLevel::None, plain, 80)).0
    }

    #[test]
    fn event_lines() {
        let e = RunEvent {
            id: Some(1),
            kind: "action".into(),
            message: "edited src/login.rs".into(),
            data: None,
            at: Some("2026-11-05T10:02:03Z".into()),
        };
        assert_eq!(event_line(&ui(false), &e), "10:02:03 › edited src/login.rs");
        assert_eq!(event_line(&ui(true), &e), "10:02:03 [action] edited src/login.rs");
        let p = RunEvent {
            kind: "progress".into(),
            data: Some(serde_json::json!({"percent": 40})),
            at: None,
            ..e
        };
        assert_eq!(event_line(&ui(false), &p), " 40% edited src/login.rs");
    }

    #[test]
    fn finish_lines() {
        let outcome = JobOutcome {
            run_id: "r1".into(),
            status: "succeeded".into(),
            branch: "b".into(),
            worktree: None,
            pr_url: Some("https://x/pull/7".into()),
            paused_until: None,
        };
        assert_eq!(
            finished(&ui(false), "AUR-12", &outcome),
            "✓ AUR-12 succeeded → https://x/pull/7"
        );
        assert_eq!(
            finished(&ui(true), "AUR-12", &outcome),
            "done AUR-12 succeeded → https://x/pull/7"
        );
        assert_eq!(
            refused(&ui(false), "AUR-12", "repo_not_configured", "no clone"),
            "✕ AUR-12 refused: repo_not_configured no clone"
        );
    }
}
