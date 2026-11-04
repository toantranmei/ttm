//! `ttm workflow show | validate | simulate | diff | publish | upgrade | copy` (S2 workflow v2).

use super::project::linked;
use crate::app::App;
use crate::cli::WorkflowCommand;
use crate::output::emit;
use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::Value;
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::workflows::{SimActor, SimulateInput, Workflow, WorkflowDiff, item_label};
use ttm_ui::{Ask, Callout, Cell, Change, Col, Diff, Header, Status, Table, Token, Ui, next_line};

const FEATURE: &str = "workflow_v2";
const WHAT: &str = "workflow v2";

/// `(project id, workflow id, project's version)`.
async fn project_workflow(
    app: &App,
    client: &Client,
    project: Option<&str>,
) -> Result<(String, String, Option<i64>)> {
    let (project_id, key) = app.project_ref(client, project).await?;
    let p = client.project(&project_id).await?;
    let workflow_id = p
        .workflow_id
        .ok_or_else(|| anyhow!("project {key} has no workflow v2 yet (the server must upgrade it)"))?;
    // The version the project runs is in the project answer (`workflow_version`), else the latest.
    let version = client
        .get::<Value>(&format!("/factory/projects/{project_id}"))
        .await
        .ok()
        .and_then(|v| v.get("workflow_version").and_then(Value::as_i64));
    Ok((project_id, workflow_id, version))
}

pub async fn run(app: App, command: WorkflowCommand) -> Result<ExitCode> {
    let (client, _) = app.api_for(FEATURE, WHAT).await?;
    match command {
        WorkflowCommand::Show { project, version } => {
            let (_, wf_id, _) = project_workflow(&app, &client, project.project.as_deref()).await?;
            let wf = match version.as_deref() {
                Some("draft") | None => client.workflow(&wf_id).await?,
                Some(v) => {
                    client
                        .workflow_version(&wf_id, v.parse().context("--version is a number or `draft`")?)
                        .await?
                }
            };
            let draft = version.as_deref() == Some("draft");
            emit(&app, &wf, |ui| show(ui, &wf, draft));
        }
        WorkflowCommand::Validate { project, file } => {
            if let Some(file) = file {
                let ws = app.workspace_id(None)?;
                let text =
                    std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
                let path = file
                    .to_string_lossy()
                    .rsplit_once(".ttm/")
                    .map(|(_, p)| p.to_string())
                    .unwrap_or_else(|| "workflow.yaml".into());
                let files = std::collections::BTreeMap::from([(path, text)]);
                let (client, _) = app.api_for("ascode_v2", "as-code files").await?;
                let result = client.ascode_validate(&ws, &files).await?;
                let ok = result.ok && result.errors.is_empty();
                emit(&app, &result, |ui| {
                    if ok {
                        ui.print(&Callout::success(format!("{} is valid", file.display())).next("ttm sync"));
                        return;
                    }
                    ui.print(&Callout::error(format!(
                        "{} issue(s) in {}",
                        result.errors.len(),
                        file.display()
                    )));
                    for e in &result.errors {
                        let at = e.line.map(|l| format!(":{l}")).unwrap_or_default();
                        ui.line(&format!(
                            "{} {} {}{at} {}: {}",
                            ui.paint(Token::Coral, if ui.mode().plain { "ERROR" } else { "✕" }),
                            e.rule.as_deref().unwrap_or("invalid"),
                            e.path,
                            e.field.as_deref().unwrap_or("-"),
                            e.message
                        ));
                    }
                });
                return Ok(if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE });
            }
            let (_, wf_id, _) = project_workflow(&app, &client, project.project.as_deref()).await?;
            let wf = client.workflow(&wf_id).await?;
            let definition = wf
                .draft
                .as_ref()
                .map(|d| d.definition.clone())
                .unwrap_or(wf.definition.clone());
            let result = client.workflow_validate(&wf_id, &definition).await?;
            let ok = result.issues.is_empty();
            emit(&app, &result, |ui| {
                let what = if wf.draft.is_some() {
                    "draft"
                } else {
                    "published version"
                };
                if ok {
                    ui.print(
                        &Callout::success(format!("The {what} of {} is valid", wf.name))
                            .next("ttm workflow publish"),
                    );
                    return;
                }
                ui.print(&Callout::error(format!(
                    "{} issue(s) in the {what} of {}",
                    result.issues.len(),
                    wf.name
                )));
                for i in &result.issues {
                    ui.line(&format!(
                        "{} {} {}: {}",
                        ui.paint(Token::Coral, if ui.mode().plain { "ERROR" } else { "✕" }),
                        i.code.as_deref().unwrap_or("invalid"),
                        i.field.as_deref().unwrap_or("-"),
                        i.message
                    ));
                }
            });
            return Ok(if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE });
        }
        WorkflowCommand::Simulate {
            task,
            to,
            version,
            roles,
        } => {
            let project = linked().ok();
            let (project_id, wf_id, _) =
                project_workflow(&app, &client, project.as_ref().map(|p| p.file.key.as_str())).await?;
            let input = SimulateInput {
                version: version.as_deref().and_then(|v| v.parse().ok()),
                definition: if version.as_deref() == Some("draft") {
                    client.workflow(&wf_id).await?.draft.map(|d| d.definition)
                } else {
                    None
                },
                project_id: Some(project_id),
                task: Some(task.clone()),
                actor: SimActor {
                    kind: "user".into(),
                    roles,
                },
                steps: vec![to.clone()],
                ..SimulateInput::default()
            };
            let sim = client.workflow_simulate(&wf_id, &input).await?;
            let allowed = sim.steps.iter().all(|s| s.allowed);
            emit(&app, &sim, |ui| {
                let head = if allowed {
                    Callout::success(format!("{task} can move to {to}"))
                } else {
                    Callout::warning(format!("{task} cannot move to {to}"))
                };
                ui.print(&head.detail(format!("ends in {}", sim.end_state.as_deref().unwrap_or("-"))));
                let mut t = Table::new(&[
                    Col::new(""),
                    Col::new("FROM"),
                    Col::new("TO"),
                    Col::new("WHY").flex(1).min(10),
                ]);
                for s in &sim.steps {
                    let mark = if s.allowed { Status::ok() } else { Status::fail() };
                    let why = match (&s.blocked, &s.approval_gate) {
                        (Some(b), _) => format!("{}: {}", b.code, b.message),
                        (None, Some(_)) => "waits for an approval".into(),
                        (None, None) if !s.rules.is_empty() => format!("{} rule(s) run", s.rules.len()),
                        _ => "allowed".into(),
                    };
                    t.push(vec![
                        Cell::new(ui.render(&mark)),
                        Cell::new(s.from.as_str()),
                        Cell::new(s.to.as_str()),
                        Cell::new(why),
                    ]);
                }
                ui.print(&t);
            });
            return Ok(if allowed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            });
        }
        WorkflowCommand::Diff { project, from, to } => {
            let (_, wf_id, current) = project_workflow(&app, &client, project.project.as_deref()).await?;
            let wf = client.workflow(&wf_id).await?;
            let from = from.unwrap_or_else(|| current.or(wf.latest_version).unwrap_or(1).to_string());
            let to = to.unwrap_or_else(|| "draft".into());
            let diff = client.workflow_diff(&wf_id, &from, &to).await?;
            emit(&app, &diff, |ui| {
                ui.line(&format!(
                    "{} · v{from} → {}",
                    ui.paint(Token::Ink, &wf.name),
                    version_label(&to)
                ));
                ui.print(&changes(&diff));
            });
        }
        WorkflowCommand::Publish { project, note } => {
            let (_, wf_id, _) = project_workflow(&app, &client, project.project.as_deref()).await?;
            let wf = client.workflow(&wf_id).await?;
            if wf.draft.is_none() && !wf.has_draft {
                bail!(
                    "{} has no draft to publish; edit it on the web or with `ttm sync` (.ttm/workflow.yaml)",
                    wf.name
                );
            }
            let base = wf.latest_version.or(wf.version).unwrap_or(0);
            let diff = client.workflow_diff(&wf_id, &base.to_string(), "draft").await?;
            let mut wizard = app.wizard();
            app.ui.print(
                &Header::new(format!("{} · v{base} → v{}", wf.name, base + 1)).summary("Publish the draft"),
            );
            app.ui.print(&changes(&diff));
            let note = wizard.text(
                Ask::new("Release note", "--note")
                    .value(note)
                    .default(String::new()),
            )?;
            if note.chars().count() > 280 {
                return Err(
                    ttm_ui::UsageError::new("invalid_flag", "the note has more than 280 characters")
                        .field("--note")
                        .into(),
                );
            }
            if !wizard.confirm(&format!("Publish {} v{}?", wf.name, base + 1))? {
                bail!("cancelled");
            }
            wizard.print_equivalent("workflow publish");
            let out = client.workflow_publish(&wf_id, &note, base).await?;
            emit(&app, &out, |ui| {
                let v = out["version"].as_i64().unwrap_or(base + 1);
                let n = out["notified_projects"]
                    .as_array()
                    .map(Vec::len)
                    .or_else(|| out["notified_projects"].as_u64().map(|n| n as usize))
                    .unwrap_or(0);
                ui.print(
                    &Callout::success(format!("Published {} v{v}", wf.name))
                        .detail(format!("{n} project(s) notified to upgrade"))
                        .next("ttm workflow upgrade"),
                )
            });
        }
        WorkflowCommand::Upgrade { project, to } => {
            let (project_id, key) = app.project_ref(&client, project.project.as_deref()).await?;
            let preview = client.workflow_upgrade_preview(&project_id, to).await?;
            app.ui.print(
                &Header::new(format!(
                    "{key} · {} v{} → v{}",
                    preview.name, preview.from_version, preview.to_version
                ))
                .summary(format!("{} task(s) change status", preview.remap.len())),
            );
            app.ui.print(&changes(&preview.diff));
            if !preview.remap.is_empty() {
                let mut t = Table::new(&[
                    Col::new("KEY"),
                    Col::new("TITLE").flex(2).min(8),
                    Col::new("FROM"),
                    Col::new("TO"),
                    Col::new("WHY").flex(1).min(6),
                ]);
                for r in &preview.remap {
                    t.push(vec![
                        Cell::token(r.key.as_str(), Token::Ink),
                        Cell::new(r.title.as_str()),
                        Cell::new(r.from_state.as_str()),
                        Cell::token(r.to_state.as_str(), Token::Teal),
                        Cell::new(r.reason.as_deref().unwrap_or("-")),
                    ]);
                }
                app.ui.print(&t);
            }
            let mut wizard = app.wizard();
            wizard.record("--to", Some(preview.to_version.to_string()));
            if !wizard.confirm(&format!("Upgrade {key} to v{}?", preview.to_version))? {
                bail!("cancelled");
            }
            let out = client
                .workflow_upgrade(&project_id, preview.to_version, &preview.preview_hash, None)
                .await?;
            emit(&app, &out, |ui| {
                ui.print(
                    &Callout::success(format!("{key} now uses {} v{}", preview.name, preview.to_version))
                        .detail(format!(
                            "{} task(s) moved",
                            out["moved"].as_i64().unwrap_or(preview.remap.len() as i64)
                        ))
                        .next("ttm task list"),
                )
            });
        }
        WorkflowCommand::Copy { project, name } => {
            let (_, wf_id, _) = project_workflow(&app, &client, project.project.as_deref()).await?;
            if !app.wizard().confirm("Copy this workflow into an editable one?")? {
                bail!("cancelled");
            }
            let copy = client.workflow_copy(&wf_id, name.as_deref()).await?;
            emit(&app, &copy, |ui| {
                ui.print(
                    &Callout::success(format!("Copied as {} ({})", copy.name, copy.id))
                        .next("ttm workflow show"),
                )
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn version_label(v: &str) -> String {
    if v == "draft" {
        "draft".into()
    } else {
        format!("v{v}")
    }
}

/// A workflow diff as `+ - ~` lines.
pub fn changes(diff: &WorkflowDiff) -> Diff {
    let mut out = Vec::new();
    for (part, label) in [
        (&diff.states, "status"),
        (&diff.transitions, "transition"),
        (&diff.rules, "rule"),
    ] {
        out.extend(
            part.added
                .iter()
                .map(|v| Change::added(format!("{label} {}", item_label(v)))),
        );
        out.extend(
            part.removed
                .iter()
                .map(|v| Change::removed(format!("{label} {}", item_label(v)))),
        );
        out.extend(part.changed.iter().map(|v| {
            let c = Change::changed(format!("{label} {}", item_label(v)));
            match v.get("fields").and_then(Value::as_array) {
                Some(f) => c.detail(f.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", ")),
                None => c,
            }
        }));
    }
    if out.is_empty() && diff.layout_only {
        out.push(Change::changed("layout only").detail("positions on the board"));
    }
    Diff::changes(&out)
}

/// `🤖` when a status assigns an agent automatically, `👤` for a person, `-` otherwise.
fn auto_assign(ui: &Ui, v: &Option<Value>) -> String {
    match v {
        Some(Value::Bool(true)) => ui.render(&ttm_ui::Badge::agent()),
        Some(Value::String(s)) if s == "agent" => ui.render(&ttm_ui::Badge::agent()),
        Some(Value::String(s)) if s == "person" || s == "user" => ui.render(&ttm_ui::Badge::person()),
        Some(Value::Object(o)) if o.get("kind").and_then(Value::as_str) == Some("agent") => {
            ui.render(&ttm_ui::Badge::agent())
        }
        Some(Value::Object(_)) => ui.render(&ttm_ui::Badge::person()),
        _ => "-".into(),
    }
}

/// A transition guard in words.
fn guard_text(v: &Option<Value>) -> String {
    match v {
        None | Some(Value::Null) => "-".into(),
        Some(Value::Object(o)) => o
            .iter()
            .map(|(k, v)| match v {
                Value::Bool(true) => k.replace('_', " "),
                Value::Array(a) => format!(
                    "{} {}",
                    k.replace('_', " "),
                    a.iter().map(item_label).collect::<Vec<_>>().join("/")
                ),
                other => format!("{} {}", k.replace('_', " "), item_label(other)),
            })
            .collect::<Vec<_>>()
            .join(", "),
        Some(other) => item_label(other),
    }
}

fn show(ui: &Ui, wf: &Workflow, draft: bool) {
    let definition = match (&wf.draft, draft) {
        (Some(d), true) => &d.definition,
        _ => &wf.definition,
    };
    let version = if draft {
        "draft".to_string()
    } else {
        format!("v{}", wf.version.or(wf.latest_version).unwrap_or(1))
    };
    let summary = format!(
        "{} status(es) · {} transition(s){}",
        Workflow::states(definition).len(),
        Workflow::transitions(definition).len(),
        if wf.has_draft && !draft {
            " · a draft is waiting"
        } else {
            ""
        }
    );
    ui.print(&Header::new(format!("{} · {version}", wf.name)).summary(summary));
    let mut states = Table::new(&[
        Col::new(""),
        Col::new("STATUS").flex(1).min(8),
        Col::new("ID"),
        Col::new("ROLE"),
        Col::new("AUTO"),
        Col::new("WIP").right(),
    ]);
    for s in Workflow::states(definition) {
        states.push(vec![
            Cell::new(ui.render(&Status::of_category(s.category.as_deref().unwrap_or(&s.id)))),
            Cell::new(s.name.as_str()),
            Cell::token(s.id.as_str(), Token::Muted),
            Cell::new(s.role.as_deref().unwrap_or("-")),
            Cell::new(auto_assign(ui, &s.auto_assign)),
            Cell::new(s.wip.map(|w| w.to_string()).unwrap_or_else(|| "-".into())),
        ]);
    }
    ui.print(&states);
    ui.blank();
    let mut transitions = Table::new(&[
        Col::new("FROM").flex(1).min(4),
        Col::new("TO"),
        Col::new("GUARD").flex(2).min(6),
        Col::new("RULES").right(),
    ]);
    for t in Workflow::transitions(definition) {
        let from = match &t.from {
            Value::Array(a) => a.iter().map(item_label).collect::<Vec<_>>().join(", "),
            Value::Null => "*".into(),
            v => item_label(v),
        };
        let rules = t
            .rules
            .as_ref()
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let rules = if rules == 0 {
            "-".into()
        } else if ui.mode().plain {
            rules.to_string()
        } else {
            format!("⚡{rules}")
        };
        transitions.push(vec![
            Cell::new(from),
            Cell::new(format!("→ {}", t.to)),
            Cell::new(guard_text(&t.guard)),
            Cell::new(rules),
        ]);
    }
    ui.print(&transitions);
    ui.line(&next_line(
        ui.mode(),
        if wf.has_draft {
            "ttm workflow diff"
        } else {
            "ttm workflow validate"
        },
    ));
}
