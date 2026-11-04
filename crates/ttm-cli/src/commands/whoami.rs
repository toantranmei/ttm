//! `ttm whoami`.

use crate::app::App;
use crate::output::emit;
use anyhow::Result;
use serde::Serialize;
use std::process::ExitCode;
use ttm_ui::{KeyValue, Token};

#[derive(Serialize)]
struct WhoamiOutput {
    host: String,
    profile: String,
    user: ttm_api::MeUser,
    workspace: Option<ttm_api::WorkspaceSummary>,
    /// `meta.membership` (0.2): the Mei-er membership, `null` for non-members.
    meta: crate::membership::Meta,
}

pub async fn run(app: App) -> Result<ExitCode> {
    let (client, _) = app.api().await?;
    let me = client.me().await?;
    let current = app.ctx.workspace();
    let workspace = current.and_then(|id| me.workspaces.iter().find(|w| w.id == id).cloned());
    let out = WhoamiOutput {
        host: app.host().to_string(),
        profile: app.ctx.profile_name.clone(),
        meta: crate::membership::Meta {
            membership: me.user.membership.clone(),
        },
        user: me.user,
        workspace,
    };
    emit(&app, &out, |ui| {
        ui.line(&format!(
            "{} <{}>",
            ui.paint(Token::Ink, &out.user.name),
            out.user.email
        ));
        let kv = KeyValue::new()
            .row("user", out.user.name.as_str())
            .row("email", out.user.email.as_str())
            .row("host", out.host.as_str())
            .row("profile", out.profile.as_str())
            .row(
                "workspace",
                out.workspace
                    .as_ref()
                    .map(|w| format!("{} ({})", w.name, w.id))
                    .unwrap_or_else(|| "not chosen".into()),
            );
        ui.print(&kv);
        for line in
            crate::membership::lines(ui, out.meta.membership.as_ref(), crate::membership::today(), true)
        {
            ui.line(&line);
        }
        if out.workspace.is_none() {
            ui.line(&ttm_ui::next_line(ui.mode(), "ttm workspace switch <name>"));
        }
    });
    Ok(ExitCode::SUCCESS)
}
