//! `ttm workspace list | switch <id|slug|name> | current`. The choice is stored in the profile.

use crate::app::App;
use crate::cli::WorkspaceCommand;
use crate::output::emit;
use anyhow::{Result, bail};
use serde::Serialize;
use std::process::ExitCode;
use ttm_api::WorkspaceSummary;
use ttm_ui::{Callout, Cell, Col, KeyValue, Table, Token, next_line};

#[derive(Serialize)]
struct Row<'a> {
    #[serde(flatten)]
    workspace: &'a WorkspaceSummary,
    current: bool,
}

pub async fn run(mut app: App, command: WorkspaceCommand) -> Result<ExitCode> {
    match command {
        WorkspaceCommand::Members { ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let (client, _) = app.api().await?;
            let members = client.workspace_members(&ws).await?;
            emit(&app, &members, |ui| {
                ui.line(&format!("{} member(s)", members.len()));
                let mut table = Table::new(&[
                    Col::new("NAME").flex(1).min(8),
                    Col::new("EMAIL").flex(1).min(8),
                    Col::new("ROLE"),
                ])
                .empty_text("No members.");
                for m in &members {
                    let mut name = format!("{} {}", ui.render(&ttm_ui::Badge::person()), m.user.name);
                    if let Some(tier) = m.tier() {
                        name.push(' ');
                        name.push_str(&ui.render(&ttm_ui::Badge::member(tier)));
                    }
                    table.push(vec![
                        Cell::new(name),
                        Cell::token(m.user.email.as_str(), Token::Muted),
                        Cell::new(m.role.as_str()),
                    ]);
                }
                ui.print(&table);
            });
        }
        WorkspaceCommand::List => {
            let (client, _) = app.api().await?;
            let me = client.me().await?;
            let current = app.ctx.workspace();
            let rows: Vec<Row> = me
                .workspaces
                .iter()
                .map(|w| Row {
                    workspace: w,
                    current: current.as_deref() == Some(w.id.as_str()),
                })
                .collect();
            emit(&app, &rows, |ui| {
                let current = rows.iter().find(|r| r.current).map(|r| r.workspace.name.as_str());
                ui.line(&match current {
                    Some(name) => format!(
                        "{} workspace(s), current: {}",
                        rows.len(),
                        ui.paint(Token::Ink, name)
                    ),
                    None => format!("{} workspace(s), none chosen", rows.len()),
                });
                let mut table = Table::new(&[
                    Col::new(""),
                    Col::new("NAME").flex(2).min(8),
                    Col::new("SLUG").flex(1).min(4),
                    Col::new("ROLE"),
                    Col::new("KIND"),
                    Col::new("ID").flex(1).min(6),
                ])
                .empty_text("No workspaces.");
                for r in &rows {
                    let mark = match (r.current, ui.mode().plain) {
                        (true, true) => "*".to_string(),
                        (true, false) => ui.paint(Token::Teal, "●"),
                        (false, _) => String::new(),
                    };
                    table.push(vec![
                        Cell::new(mark),
                        Cell::new(r.workspace.name.as_str()),
                        Cell::new(r.workspace.slug.as_deref().unwrap_or("-")),
                        Cell::new(r.workspace.role.as_str()),
                        Cell::new(r.workspace.kind.as_str()),
                        Cell::token(r.workspace.id.as_str(), Token::Muted),
                    ]);
                }
                ui.print(&table);
                if current.is_none() && !rows.is_empty() {
                    ui.line(&next_line(ui.mode(), "ttm workspace switch <name>"));
                }
            });
        }
        WorkspaceCommand::Switch { workspace } => {
            let (client, _) = app.api().await?;
            let me = client.me().await?;
            let chosen = pick(&me.workspaces, &workspace)?.clone();
            app.ctx
                .update_profile(|p| p.workspace = Some(chosen.id.clone()))?;
            emit(&app, &chosen, |ui| {
                ui.print(
                    &Callout::success(format!("Current workspace: {}", chosen.name))
                        .detail(format!("id {} · profile {}", chosen.id, app.ctx.profile_name))
                        .next("ttm project init"),
                )
            });
        }
        WorkspaceCommand::Current => {
            let id = app.ctx.workspace();
            // Best effort: show the name when the server is reachable.
            let found = match (&id, app.api().await) {
                (Some(id), Ok((client, _))) => client
                    .me()
                    .await
                    .ok()
                    .and_then(|me| me.workspaces.into_iter().find(|w| &w.id == id)),
                _ => None,
            };
            let out = serde_json::json!({ "profile": app.ctx.profile_name, "workspace_id": id, "workspace": found });
            emit(&app, &out, |ui| match (&id, &found) {
                (_, Some(w)) => {
                    ui.line(&ui.paint(Token::Ink, &w.name));
                    ui.print(
                        &KeyValue::new()
                            .row("id", w.id.as_str())
                            .opt("slug", w.slug.as_deref())
                            .row("role", w.role.as_str())
                            .row("kind", w.kind.as_str())
                            .row("profile", app.ctx.profile_name.as_str()),
                    );
                }
                (Some(id), None) => {
                    ui.line(id);
                    ui.print(&KeyValue::new().row("profile", app.ctx.profile_name.as_str()));
                }
                (None, _) => {
                    ui.print(&Callout::info("No workspace chosen").next("ttm workspace switch <name>"))
                }
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Exact id/slug/name match; a unique case-insensitive name prefix is also accepted.
pub fn pick<'a>(workspaces: &'a [WorkspaceSummary], needle: &str) -> Result<&'a WorkspaceSummary> {
    let exact: Vec<_> = workspaces.iter().filter(|w| w.matches(needle)).collect();
    match exact.as_slice() {
        [one] => return Ok(one),
        [] => {}
        many => bail!(
            "`{needle}` matches several workspaces ({}); use the id",
            many.iter().map(|w| w.id.as_str()).collect::<Vec<_>>().join(", ")
        ),
    }
    let lower = needle.to_lowercase();
    let prefix: Vec<_> = workspaces
        .iter()
        .filter(|w| w.name.to_lowercase().starts_with(&lower))
        .collect();
    match prefix.as_slice() {
        [one] => Ok(one),
        [] => bail!("no workspace named `{needle}`; see `ttm workspace list`"),
        _ => bail!("`{needle}` is ambiguous; see `ttm workspace list`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ws(id: &str, name: &str, slug: Option<&str>) -> WorkspaceSummary {
        WorkspaceSummary {
            id: id.into(),
            name: name.into(),
            kind: "team".into(),
            role: "member".into(),
            slug: slug.map(String::from),
            owner_membership: None,
        }
    }

    #[test]
    fn pick_by_id_slug_name_and_prefix() {
        let list = vec![
            ws("w1", "Cá nhân", None),
            ws("w2", "Team Aurora", Some("aurora")),
            ws("w3", "Team Borealis", None),
        ];
        assert_eq!(pick(&list, "w1").unwrap().id, "w1");
        assert_eq!(pick(&list, "AURORA").unwrap().id, "w2");
        assert_eq!(pick(&list, "team borealis").unwrap().id, "w3");
        assert_eq!(pick(&list, "cá").unwrap().id, "w1");
        assert!(pick(&list, "team").is_err(), "ambiguous prefix");
        assert!(pick(&list, "nope").is_err());
    }
}
