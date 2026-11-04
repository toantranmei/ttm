//! `ttm status`: what this machine would use, without failing when signed out or offline.

use crate::app::App;
use crate::output::emit;
use anyhow::Result;
use serde::Serialize;
use std::process::ExitCode;
use time::OffsetDateTime;

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
    emit(&app, &out, |_ui| {
        println!("ttm {}", out.version);
        println!("host:      {}", out.host);
        println!("profile:   {} ({})", out.profile, out.config_file);
        match &out.token {
            Some(t) => {
                let expiry = t.expires_at.map(|e| format!(", expires {e}")).unwrap_or_default();
                let state = if t.expired { " EXPIRED" } else { "" };
                println!("token:     {} from {}{expiry}{state}", t.masked, t.source);
            }
            None => println!("token:     none (run `ttm login`)"),
        }
        println!(
            "workspace: {}",
            out.workspace
                .as_deref()
                .unwrap_or("not chosen (ttm workspace switch)")
        );
        match &out.server {
            Some(s) => println!(
                "server:    API {} (min ttm {}), features: {}",
                s.api_version.as_deref().unwrap_or("?"),
                s.min_cli.as_deref().unwrap_or("-"),
                if s.features.is_empty() {
                    "-".to_string()
                } else {
                    s.features.join(", ")
                }
            ),
            None => println!("server:    unreachable"),
        }
    });
    Ok(ExitCode::SUCCESS)
}
