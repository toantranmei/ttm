//! `ttm logout`: revoke on the server (RFC 7009), then forget the token locally.

use crate::app::App;
use crate::output::emit;
use anyhow::Result;
use serde::Serialize;
use std::process::ExitCode;
use ttm_core::TokenSource;
use ttm_ui::Callout;

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[derive(Serialize)]
struct LogoutOutput {
    host: String,
    profile: String,
    revoked: bool,
    removed: bool,
}

pub async fn run(app: App) -> Result<ExitCode> {
    let mut revoked = false;
    if let Some((token, source)) = app.token()? {
        if source == TokenSource::Env {
            tracing::warn!("the token comes from TTM_TOKEN; unset it to sign out completely");
        }
        match app.discovery(false).await {
            Ok(discovery) => {
                let client = app.anonymous(&discovery)?;
                let endpoint = discovery.revocation_endpoint(app.host());
                let mut ok = client.revoke_token(&endpoint, &token.access_token).await.is_ok();
                if let Some(refresh) = &token.refresh_token {
                    ok &= client.revoke_token(&endpoint, refresh).await.is_ok();
                }
                if !ok {
                    tracing::warn!(
                        "the server did not confirm the revocation; the token is removed locally anyway"
                    );
                }
                revoked = ok;
            }
            Err(err) => tracing::warn!("{err:#}; removing the token locally only"),
        }
    }
    let removed = app.ctx.credentials.delete(&app.ctx.profile_name, app.host())?;
    let out = LogoutOutput {
        host: app.host().to_string(),
        profile: app.ctx.profile_name.clone(),
        revoked,
        removed,
    };
    emit(&app, &out, |ui| {
        let callout = if removed || revoked {
            Callout::success(format!("Signed out of {}", out.host)).detail(format!(
                "profile {} · revoked on the server: {}",
                out.profile,
                yes_no(out.revoked)
            ))
        } else {
            Callout::info(format!("Not signed in to {}", out.host)).detail(format!("profile {}", out.profile))
        };
        ui.print(&callout.next("ttm login"));
    });
    Ok(ExitCode::SUCCESS)
}
