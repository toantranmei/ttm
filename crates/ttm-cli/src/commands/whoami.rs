//! `ttm whoami`.

use crate::app::App;
use crate::output::emit;
use anyhow::Result;
use serde::Serialize;
use std::process::ExitCode;

#[derive(Serialize)]
struct WhoamiOutput {
    host: String,
    profile: String,
    user: ttm_api::MeUser,
    workspace: Option<ttm_api::WorkspaceSummary>,
}

pub async fn run(app: App) -> Result<ExitCode> {
    let (client, _) = app.api().await?;
    let me = client.me().await?;
    let current = app.ctx.workspace();
    let workspace = current.and_then(|id| me.workspaces.iter().find(|w| w.id == id).cloned());
    let out = WhoamiOutput {
        host: app.host().to_string(),
        profile: app.ctx.profile_name.clone(),
        user: me.user,
        workspace,
    };
    emit(&app, &out, |_ui| {
        println!("{} <{}>", out.user.name, out.user.email);
        println!("host: {}  profile: {}", out.host, out.profile);
        if let Some(w) = &out.workspace {
            println!("workspace: {} ({})", w.name, w.id);
        }
    });
    Ok(ExitCode::SUCCESS)
}
