//! `ttm automation list | show | test | enable | disable | runs` (S2 automation rules).

use crate::app::App;
use crate::cli::AutomationCommand;
use crate::output::emit;
use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::automation::{AutomationRule, summarize};
use ttm_ui::{Callout, Cell, Col, Header, Status, Table, Token, Ui, next_line};

async fn rule(client: &Client, project_id: &str, needle: &str) -> Result<AutomationRule> {
    client
        .automation_rules(Some(project_id), None)
        .await?
        .into_iter()
        .find(|r| r.matches(needle))
        .ok_or_else(|| anyhow!("no rule `{needle}` in this project; see `ttm automation list`"))
}

fn enabled_cell(ui: &Ui, on: bool) -> String {
    if ui.mode().plain {
        return if on { "on".into() } else { "off".into() };
    }
    if on {
        ui.paint(Token::Teal, "●")
    } else {
        ui.render(&Status::phase(ttm_api::vocab::Phase::Paused))
    }
}

/// Conditions as lines: `{match: all|any, filters: [...]}`.
fn conditions(v: &Value) -> Vec<String> {
    let filters = v
        .get("filters")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    filters.iter().map(summarize).collect()
}

fn tree(ui: &Ui, r: &AutomationRule) {
    ui.line(&format!(
        "{} {}",
        ui.paint(Token::Ink, "When"),
        summarize(&r.trigger)
    ));
    let conds = conditions(&r.conditions);
    let joiner = r.conditions.get("match").and_then(Value::as_str).unwrap_or("all");
    if conds.is_empty() {
        ui.line(&format!("{} always", ui.paint(Token::Ink, "If")));
    } else {
        ui.line(&format!("{} {joiner} of", ui.paint(Token::Ink, "If")));
        for (i, c) in conds.iter().enumerate() {
            let branch = if i + 1 == conds.len() { "└─" } else { "├─" };
            ui.line(&format!("  {} {c}", if ui.mode().plain { "-" } else { branch }));
        }
    }
    ui.line(&ui.paint(Token::Ink, "Then"));
    for (i, a) in r.actions.iter().enumerate() {
        let branch = if i + 1 == r.actions.len() {
            "└─"
        } else {
            "├─"
        };
        ui.line(&format!(
            "  {} {}",
            if ui.mode().plain { "-" } else { branch },
            summarize(a)
        ));
    }
}

pub async fn run(app: App, command: AutomationCommand) -> Result<ExitCode> {
    let (client, _) = app.api_for("workflow_v2", "automation rules").await?;
    match command {
        AutomationCommand::List { project } => {
            let (pid, key) = app.project_ref(&client, project.project.as_deref()).await?;
            let rules = client.automation_rules(Some(&pid), None).await?;
            emit(&app, &rules, |ui| {
                let on = rules.iter().filter(|r| r.enabled).count();
                ui.line(&format!("{key} · {} rule(s), {on} on", rules.len()));
                let mut t = Table::new(&[
                    Col::new(""),
                    Col::new("NAME").flex(2).min(8),
                    Col::new("WHEN").flex(2).min(8),
                    Col::new("RUNS 7D").right(),
                    Col::new("ID"),
                ])
                .empty_text("No rules.");
                for r in &rules {
                    t.push(vec![
                        Cell::new(enabled_cell(ui, r.enabled)),
                        Cell::new(r.name.as_str()),
                        Cell::new(summarize(&r.trigger)),
                        Cell::new(r.runs_7d.map(|n| n.to_string()).unwrap_or_else(|| "-".into())),
                        Cell::token(r.slug.as_deref().unwrap_or(&r.id), Token::Muted),
                    ]);
                }
                ui.print(&t);
                if let Some(r) = rules.first() {
                    ui.line(&next_line(
                        ui.mode(),
                        &format!("ttm automation show {}", r.slug.as_deref().unwrap_or(&r.id)),
                    ));
                }
            });
        }
        AutomationCommand::Show {
            rule: needle,
            project,
        } => {
            let (pid, _) = app.project_ref(&client, project.project.as_deref()).await?;
            let r = rule(&client, &pid, &needle).await?;
            emit(&app, &r, |ui| {
                ui.print(&Header::new(r.name.as_str()).summary(format!(
                    "{} · {} run(s) in 7 days{}",
                    if r.enabled { "on" } else { "off" },
                    r.runs_7d.unwrap_or(0),
                    if r.builtin == Some(true) {
                        " · built-in"
                    } else {
                        ""
                    }
                )));
                tree(ui, &r);
                ui.line(&next_line(
                    ui.mode(),
                    &format!("ttm automation test {needle} --task <KEY>"),
                ));
            });
        }
        AutomationCommand::Test {
            rule: needle,
            task,
            project,
        } => {
            let (pid, _) = app.project_ref(&client, project.project.as_deref()).await?;
            let r = rule(&client, &pid, &needle).await?;
            let dry = client.automation_test(&r.id, &pid, &task).await?;
            emit(&app, &dry, |ui| {
                let head = if dry.matched {
                    Callout::success(format!("{} would run on {task}", r.name))
                } else {
                    Callout::info(format!("{} would not run on {task}", r.name))
                };
                ui.print(&head.detail("Dry run: nothing changed."));
                let mut t = Table::new(&[
                    Col::new(""),
                    Col::new("ACTION").flex(2).min(8),
                    Col::new("WHY").flex(1).min(6),
                ]);
                for a in &dry.actions {
                    let will = a
                        .get("will_run")
                        .or_else(|| a.get("ok"))
                        .and_then(Value::as_bool)
                        .unwrap_or(dry.matched);
                    let why = a
                        .get("reason")
                        .or_else(|| a.get("skipped"))
                        .and_then(Value::as_str)
                        .unwrap_or(if will { "will do" } else { "skipped" });
                    t.push(vec![
                        Cell::new(ui.render(&if will { Status::ok() } else { Status::fail() })),
                        Cell::new(summarize(a)),
                        Cell::new(why),
                    ]);
                }
                if !t.is_empty() {
                    ui.print(&t);
                }
            });
        }
        AutomationCommand::Enable {
            rule: needle,
            project,
        } => {
            toggle(&app, &client, &needle, project.project.as_deref(), true).await?;
        }
        AutomationCommand::Disable {
            rule: needle,
            project,
        } => {
            toggle(&app, &client, &needle, project.project.as_deref(), false).await?;
        }
        AutomationCommand::Runs {
            rule: needle,
            limit,
            project,
        } => {
            let (pid, _) = app.project_ref(&client, project.project.as_deref()).await?;
            let rule_id = match &needle {
                Some(n) => Some(rule(&client, &pid, n).await?.id),
                None => None,
            };
            let runs = client
                .automation_runs(
                    rule_id.as_deref(),
                    if rule_id.is_none() { Some(&pid) } else { None },
                    limit,
                )
                .await?;
            emit(&app, &runs, |ui| {
                ui.line(&format!("{} run(s)", runs.len()));
                let mut t = Table::new(&[
                    Col::new(""),
                    Col::new("WHEN"),
                    Col::new("TASK"),
                    Col::new("STATUS"),
                    Col::new("REASON").flex(1).min(6),
                ])
                .empty_text("No runs.");
                for r in &runs {
                    let ok = matches!(r.status.as_str(), "ok" | "succeeded" | "done" | "applied");
                    let task = r
                        .task
                        .as_ref()
                        .and_then(|t| t.get("key").or_else(|| t.get("id")))
                        .and_then(Value::as_str)
                        .unwrap_or("-");
                    t.push(vec![
                        Cell::new(ui.render(&if ok { Status::ok() } else { Status::fail() })),
                        Cell::token(
                            r.created_at.as_deref().map(short_time).unwrap_or("-"),
                            Token::Muted,
                        ),
                        Cell::new(task),
                        Cell::new(r.status.as_str()),
                        Cell::new(r.reason.as_deref().unwrap_or("-")),
                    ]);
                }
                ui.print(&t);
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `2026-11-05T10:02:03Z` → `11-05 10:02`.
pub fn short_time(at: &str) -> &str {
    at.get(5..16).unwrap_or(at)
}

async fn toggle(app: &App, client: &Client, needle: &str, project: Option<&str>, on: bool) -> Result<()> {
    let (pid, _) = app.project_ref(client, project).await?;
    let r = rule(client, &pid, needle).await?;
    let verb = if on { "Turn on" } else { "Turn off" };
    if !app.wizard().confirm(&format!("{verb} {}?", r.name))? {
        bail!("cancelled");
    }
    let updated = client.automation_set_enabled(&r.id, on).await?;
    emit(app, &updated, |ui| {
        let state = if on { "on" } else { "off" };
        ui.print(&Callout::success(format!("{} is {state}", updated.name)).next("ttm automation list"))
    });
    Ok(())
}
