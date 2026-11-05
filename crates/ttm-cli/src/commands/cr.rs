//! `ttm cr list | show | approve | reject`: plan change requests and, on S5 servers, configuration requests.

use crate::app::App;
use crate::cli::{CrCommand, CrKind, CrStatus};
use crate::output::emit;
use anyhow::{Result, bail};
use serde_json::Value;
use std::process::ExitCode;
use ttm_api::factory::ChangeRequest;
use ttm_ui::{Callout, Cell, Col, Diff, Header, KeyValue, Status, Table, Token, Ui, next_line};

fn kind_str(k: CrKind) -> &'static str {
    match k {
        CrKind::Plan => "plan",
        CrKind::Workflow => "workflow",
        CrKind::Rule => "rule",
        CrKind::Agent => "agent",
        CrKind::Skill => "skill",
    }
}

fn status_glyph(ui: &Ui, status: &str) -> String {
    let s = match status {
        "pending" => Status::of_category("approval"),
        "applied" | "approved" => Status::ok(),
        "rejected" => Status::fail(),
        "stale" => Status::warn(),
        other => Status::of_category(other),
    };
    format!("{} {status}", ui.render(&s))
}

fn author(ui: &Ui, cr: &ChangeRequest) -> String {
    super::task::member(ui, cr.author.as_ref())
}

/// `3d`, `5h`, `12m` since an RFC 3339 time.
pub fn age(at: Option<&str>) -> String {
    let Some(at) = at else { return "-".into() };
    let Ok(t) = time::OffsetDateTime::parse(at, &time::format_description::well_known::Rfc3339) else {
        return "-".into();
    };
    let secs = (time::OffsetDateTime::now_utc() - t).whole_seconds().max(0);
    match secs {
        s if s >= 86_400 => format!("{}d", s / 86_400),
        s if s >= 3_600 => format!("{}h", s / 3_600),
        s => format!("{}m", s / 60),
    }
}

fn file_count(cr: &ChangeRequest) -> usize {
    match &cr.files {
        Some(Value::Array(a)) => a.len(),
        Some(Value::Object(o)) => o.len(),
        _ => cr.changes.len(),
    }
}

pub async fn run(app: App, command: CrCommand) -> Result<ExitCode> {
    let (client, discovery) = app.api().await?;
    let config = discovery.has_feature("ascode_v2");
    match command {
        CrCommand::List {
            project,
            kind,
            status,
        } => {
            let (pid, key) = app.project_ref(&client, project.project.as_deref()).await?;
            let status_q = match status {
                CrStatus::Pending => Some("pending"),
                CrStatus::Applied => Some("applied"),
                CrStatus::Rejected => Some("rejected"),
                CrStatus::All => None,
            };
            let mut list: Vec<ChangeRequest> = Vec::new();
            if kind.is_none_or(|k| k == CrKind::Plan) {
                list.extend(
                    client
                        .project_change_requests(&pid)
                        .await?
                        .into_iter()
                        .filter(|c| status_q.is_none_or(|s| c.status == s))
                        .map(|mut c| {
                            c.kind.get_or_insert_with(|| "plan".into());
                            c
                        }),
                );
            }
            if kind.is_none_or(|k| k != CrKind::Plan) {
                if !config && kind.is_some() {
                    return Err(ttm_ui::UsageError::unsupported_feature(
                        "ascode_v2",
                        "configuration requests",
                    )
                    .into());
                }
                if config {
                    let ws = app.workspace_id(None)?;
                    list.extend(client.config_requests(&ws, status_q, kind.map(kind_str)).await?);
                }
            }
            list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            emit(&app, &list, |ui| {
                ui.line(&format!("{key} · {} change request(s)", list.len()));
                let mut t = Table::new(&[
                    Col::new("ID"),
                    Col::new("KIND"),
                    Col::new("TARGET").flex(1).min(6),
                    Col::new("STATUS"),
                    Col::new("AUTHOR").flex(1).min(6),
                    Col::new("FILES").right(),
                    Col::new("AGE").right(),
                ])
                .empty_text("No change requests.");
                for c in &list {
                    t.push(vec![
                        Cell::token(c.id.chars().take(8).collect::<String>(), Token::Ink),
                        Cell::new(c.kind.as_deref().unwrap_or("plan")),
                        Cell::new(c.target.as_deref().unwrap_or("-")),
                        Cell::new(status_glyph(ui, &c.status)),
                        Cell::new(author(ui, c)),
                        Cell::new(file_count(c).to_string()),
                        Cell::token(age(c.created_at.as_deref()), Token::Muted),
                    ]);
                }
                ui.print(&t);
                if let Some(c) = list.first() {
                    ui.line(&next_line(ui.mode(), &format!("ttm cr show {}", c.id)));
                }
            });
        }
        CrCommand::Show { id } => {
            let cr = client.change_request(&id).await?;
            emit(&app, &cr, |ui| show(ui, &cr));
        }
        CrCommand::Approve { id } => decide(&app, &client, &id, true, None).await?,
        CrCommand::Reject { id, note } => decide(&app, &client, &id, false, note).await?,
    }
    Ok(ExitCode::SUCCESS)
}

fn show(ui: &Ui, cr: &ChangeRequest) {
    let kind = cr.kind.as_deref().unwrap_or("plan");
    ui.print(&Header::new(format!("{kind} change request {}", cr.id)).summary(status_glyph(ui, &cr.status)));
    ui.print(
        &KeyValue::new()
            .opt("target", cr.target.as_deref())
            .row("author", author(ui, cr))
            .opt("base rev", cr.base_rev.map(|r| r.to_string()))
            .opt("created", cr.created_at.as_deref())
            .opt("note", cr.note.as_deref()),
    );
    // Files: `[{path, old, new}]` / `{path: {old, new}}`, or a unified `diff` string, or plan `changes`.
    let mut shown = false;
    let files: Vec<(String, String, String)> = match &cr.files {
        Some(Value::Array(a)) => a
            .iter()
            .map(|f| {
                (
                    f.get("path").and_then(Value::as_str).unwrap_or("?").to_string(),
                    f.get("old")
                        .or_else(|| f.get("base"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    f.get("new")
                        .or_else(|| f.get("text"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                )
            })
            .collect(),
        Some(Value::Object(o)) => o
            .iter()
            .map(|(p, f)| {
                (
                    p.clone(),
                    f.get("old").and_then(Value::as_str).unwrap_or("").to_string(),
                    f.get("new")
                        .and_then(Value::as_str)
                        .or_else(|| f.as_str())
                        .unwrap_or("")
                        .to_string(),
                )
            })
            .collect(),
        _ => vec![],
    };
    for (path, old, new) in &files {
        ui.blank();
        ui.print(&Diff::text(old, new, path.clone()));
        shown = true;
    }
    if !shown && let Some(Value::String(d)) = &cr.diff {
        ui.blank();
        for line in d.lines() {
            let token = if line.starts_with('+') {
                Some(Token::Teal)
            } else if line.starts_with('-') {
                Some(Token::Coral)
            } else if line.starts_with("@@") {
                Some(Token::Purple)
            } else {
                None
            };
            ui.line(&match token {
                Some(t) => ui.paint(t, line),
                None => line.to_string(),
            });
        }
        shown = true;
    }
    if !shown && !cr.changes.is_empty() {
        ui.blank();
        let changes: Vec<ttm_ui::Change> = cr
            .changes
            .iter()
            .map(|c| {
                let key = c.get("key").and_then(Value::as_str).unwrap_or("?");
                let op = c.get("op").and_then(Value::as_str).unwrap_or("update");
                let fields = c.get("fields").map(|f| match f {
                    Value::Object(o) => o.keys().cloned().collect::<Vec<_>>().join(", "),
                    Value::Array(a) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
                    _ => String::new(),
                });
                let change = match op {
                    "create" => ttm_ui::Change::added(key),
                    "delete" => ttm_ui::Change::removed(key),
                    _ => ttm_ui::Change::changed(key),
                };
                match fields {
                    Some(f) if !f.is_empty() => change.detail(f),
                    _ => change,
                }
            })
            .collect();
        ui.print(&Diff::changes(&changes));
    }
    if cr.status == "pending" {
        ui.line(&next_line(ui.mode(), &format!("ttm cr approve {}", cr.id)));
    }
}

async fn decide(
    app: &App,
    client: &ttm_api::Client,
    id: &str,
    approve: bool,
    note: Option<String>,
) -> Result<()> {
    let cr = client.change_request(id).await?;
    if !app.json {
        show(&app.ui, &cr);
    }
    let verb = if approve { "Approve" } else { "Reject" };
    if !app.wizard().confirm(&format!("{verb} change request {id}?"))? {
        bail!("cancelled");
    }
    let out = client.decide_change_request(id, approve, note.as_deref()).await?;
    emit(app, &out, |ui| {
        let done = if approve { "approved" } else { "rejected" };
        ui.print(
            &Callout::success(format!("Change request {id} {done}")).next(if approve {
                "ttm sync"
            } else {
                "ttm cr list"
            }),
        )
    });
    Ok(())
}
