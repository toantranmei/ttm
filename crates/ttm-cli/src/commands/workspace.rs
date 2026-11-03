//! `ttm workspace list | switch <id|slug|name> | current`. The choice is stored in the profile.

use crate::app::App;
use crate::cli::WorkspaceCommand;
use crate::output::emit;
use anyhow::{Result, bail};
use serde::Serialize;
use std::process::ExitCode;
use ttm_api::WorkspaceSummary;

#[derive(Serialize)]
struct Row<'a> {
    #[serde(flatten)]
    workspace: &'a WorkspaceSummary,
    current: bool,
}

pub async fn run(mut app: App, command: WorkspaceCommand) -> Result<ExitCode> {
    match command {
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
            emit(app.json, &rows, || {
                if rows.is_empty() {
                    println!("No workspaces.");
                }
                for r in &rows {
                    let mark = if r.current { "*" } else { " " };
                    let slug = r
                        .workspace
                        .slug
                        .as_deref()
                        .map(|s| format!(" [{s}]"))
                        .unwrap_or_default();
                    println!(
                        "{mark} {}{slug}  {} · {} · {}",
                        r.workspace.name, r.workspace.kind, r.workspace.role, r.workspace.id
                    );
                }
            });
        }
        WorkspaceCommand::Switch { workspace } => {
            let (client, _) = app.api().await?;
            let me = client.me().await?;
            let chosen = pick(&me.workspaces, &workspace)?.clone();
            app.ctx
                .update_profile(|p| p.workspace = Some(chosen.id.clone()))?;
            emit(app.json, &chosen, || {
                println!(
                    "Current workspace for profile `{}`: {} ({})",
                    app.ctx.profile_name, chosen.name, chosen.id
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
            emit(app.json, &out, || match (&id, &found) {
                (_, Some(w)) => println!("{} ({})", w.name, w.id),
                (Some(id), None) => println!("{id}"),
                (None, _) => println!("No workspace chosen; run `ttm workspace switch <name>`."),
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
