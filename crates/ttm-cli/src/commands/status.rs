//! `ttm status`: what this machine would use, without failing when signed out or offline.

use crate::app::App;
use crate::output::emit;
use anyhow::Result;
use serde::Serialize;
use std::process::ExitCode;
use time::OffsetDateTime;
use ttm_ui::{Callout, KeyValue, Status, next_line};

#[derive(Serialize)]
struct StatusOutput {
    version: &'static str,
    host: String,
    profile: String,
    config_file: String,
    signed_in: bool,
    token: Option<TokenInfo>,
    workspace: Option<String>,
    server: Option<ServerInfo>,
}

#[derive(Serialize)]
struct TokenInfo {
    source: ttm_core::TokenSource,
    masked: String,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
    expired: bool,
    scope: Option<String>,
}

#[derive(Serialize)]
struct ServerInfo {
    api_version: Option<String>,
    min_cli: Option<String>,
    features: Vec<String>,
}

pub async fn run(app: App) -> Result<ExitCode> {
    let token = app.token()?.map(|(t, source)| TokenInfo {
        source,
        masked: t.masked(),
        expires_at: t.expires_at,
        expired: t.is_expired(OffsetDateTime::now_utc()),
        scope: t.scope,
    });
    let server = match app.discovery(false).await {
        Ok(d) => Some(ServerInfo {
            api_version: d.api_version,
            min_cli: d.min_cli,
            features: d.features,
        }),
        Err(err) => {
            tracing::warn!("{err:#}");
            None
        }
    };
    let out = StatusOutput {
        version: ttm_core::VERSION,
        host: app.host().to_string(),
        profile: app.ctx.profile_name.clone(),
        config_file: app.ctx.paths.config_file().display().to_string(),
        signed_in: token.as_ref().is_some_and(|t| !t.expired),
        token,
        workspace: app.ctx.workspace(),
        server,
    };
    let keyring = app.ctx.credentials.keyring_status();
    emit(&app, &out, |ui| {
        let mark = |status: Status| ui.render(&status);
        let mut kv = KeyValue::new()
            .row("version", format!("ttm {}", out.version))
            .row("host", out.host.as_str())
            .row("profile", format!("{} ({})", out.profile, out.config_file));
        kv.push(
            "token",
            match &out.token {
                Some(t) => {
                    let expiry = t.expires_at.map(|e| format!(", expires {e}")).unwrap_or_default();
                    let (status, state) = if t.expired {
                        (Status::fail(), " · expired")
                    } else {
                        (Status::ok(), "")
                    };
                    format!("{} {} from {}{expiry}{state}", mark(status), t.masked, t.source)
                }
                None => format!("{} none", mark(Status::warn())),
            },
        );
        kv.push(
            "keyring",
            match &keyring {
                Ok(()) => format!("{} OS keyring", mark(Status::ok())),
                Err(why) => format!("{} {why}", mark(Status::warn())),
            },
        );
        kv.push(
            "workspace",
            out.workspace.as_deref().unwrap_or("not chosen").to_string(),
        );
        kv.push(
            "server",
            match &out.server {
                Some(s) => format!(
                    "{} API {} (min ttm {}), features: {}",
                    mark(Status::ok()),
                    s.api_version.as_deref().unwrap_or("?"),
                    s.min_cli.as_deref().unwrap_or("-"),
                    if s.features.is_empty() {
                        "-".to_string()
                    } else {
                        s.features.join(", ")
                    }
                ),
                None => format!("{} unreachable", mark(Status::fail())),
            },
        );
        let headline = if out.signed_in {
            Callout::success(format!("Signed in to {}", out.host))
        } else {
            Callout::warning(format!("Not signed in to {}", out.host))
        };
        let next = if !out.signed_in {
            Some("ttm login")
        } else if out.workspace.is_none() {
            Some("ttm workspace switch <name>")
        } else if out.server.is_none() {
            Some("ttm doctor")
        } else {
            None
        };
        ui.print(&headline);
        ui.print(&kv);
        if let Some(next) = next {
            ui.line(&next_line(ui.mode(), next));
        }
    });
    Ok(ExitCode::SUCCESS)
}
