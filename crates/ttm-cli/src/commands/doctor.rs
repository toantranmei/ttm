//! `ttm doctor`: one line per check; exit code 1 when any check fails.

use crate::app::App;
use crate::output::emit;
use crate::util::{version_of, which};
use anyhow::Result;
use serde::Serialize;
use std::process::ExitCode;
use ttm_api::Client;
use ttm_ui::{Callout, Cell, Col, Status, Table, next_line};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Ok,
    Warn,
    Fail,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub name: &'static str,
    pub level: Level,
    pub detail: String,
}

impl From<Level> for ttm_ui::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::Ok => ttm_ui::Level::Ok,
            Level::Warn => ttm_ui::Level::Warn,
            Level::Fail => ttm_ui::Level::Fail,
        }
    }
}

/// What to run about a check that is not ok.
fn next_for(name: &str) -> Option<&'static str> {
    Some(match name {
        "discovery" | "host" => "ttm status",
        "version" => "ttm self-update",
        "token" => "ttm login",
        "keyring" => "ttm status",
        "git" => "install git, then ttm doctor",
        "claude" => "install Claude Code, then ttm doctor",
        "ttm-mcp" => "ttm self-update",
        _ => return None,
    })
}

fn check(name: &'static str, level: Level, detail: impl Into<String>) -> Check {
    Check {
        name,
        level,
        detail: detail.into(),
    }
}

pub async fn run(app: App) -> Result<ExitCode> {
    let mut checks = vec![check(
        "host",
        Level::Ok,
        format!("{} (profile {})", app.host(), app.ctx.profile_name),
    )];

    let discovery = match app.discovery(true).await {
        Ok(d) => {
            checks.push(check(
                "discovery",
                Level::Ok,
                format!("api_base {}", d.api_base(app.host())),
            ));
            let version = match d.upgrade_required(ttm_core::VERSION) {
                Some(min) => check(
                    "version",
                    Level::Fail,
                    format!(
                        "ttm {} < server minimum {min}; run `ttm self-update`",
                        ttm_core::VERSION
                    ),
                ),
                None => check(
                    "version",
                    Level::Ok,
                    format!(
                        "ttm {}, server API {}",
                        ttm_core::VERSION,
                        d.api_version.as_deref().unwrap_or("?")
                    ),
                ),
            };
            checks.push(version);
            Some(d)
        }
        Err(err) => {
            checks.push(check("discovery", Level::Fail, format!("{err:#}")));
            None
        }
    };

    checks.push(match app.token() {
        Ok(Some((token, source))) => match &discovery {
            Some(d) => {
                let client = Client::new(d.api_base(app.host()), Some(token.access_token.clone()))?;
                match client.me().await {
                    Ok(me) => check(
                        "token",
                        Level::Ok,
                        format!("{} ({source}) as {}", token.masked(), me.user.email),
                    ),
                    Err(err) if err.status() == Some(403) => check(
                        "token",
                        Level::Warn,
                        format!("{} ({source}) valid but lacks workspace:read", token.masked()),
                    ),
                    Err(err) => check(
                        "token",
                        Level::Fail,
                        format!("{} ({source}): {err}", token.masked()),
                    ),
                }
            }
            None => check(
                "token",
                Level::Warn,
                format!("{} ({source}), not verified: server unreachable", token.masked()),
            ),
        },
        Ok(None) => check("token", Level::Warn, "not signed in (ttm login)"),
        Err(err) => check("token", Level::Fail, format!("{err:#}")),
    });

    checks.push(match app.ctx.credentials.keyring_status() {
        Ok(()) => check("keyring", Level::Ok, "OS keyring available"),
        Err(why) => check(
            "keyring",
            Level::Warn,
            format!("{why}; tokens go to credentials.toml (0600)"),
        ),
    });

    checks.push(match which("git") {
        Some(path) => check(
            "git",
            Level::Ok,
            version_of(&path).unwrap_or_else(|| path.display().to_string()),
        ),
        None => check(
            "git",
            Level::Warn,
            "not found on PATH (needed by ttm sync and the runner)",
        ),
    });
    checks.push(match which("claude") {
        Some(path) => check(
            "claude",
            Level::Ok,
            version_of(&path).unwrap_or_else(|| path.display().to_string()),
        ),
        None => check(
            "claude",
            Level::Warn,
            "Claude Code not found on PATH (needed by ttm runner)",
        ),
    });
    checks.push(match which("ttm-mcp") {
        Some(path) => check("ttm-mcp", Level::Ok, path.display().to_string()),
        None => check(
            "ttm-mcp",
            Level::Warn,
            "not found on PATH (MCP clients cannot start it)",
        ),
    });

    let failed = checks.iter().any(|c| c.level == Level::Fail);
    emit(&app, &checks, |ui| {
        let fails = checks.iter().filter(|c| c.level == Level::Fail).count();
        let warns = checks.iter().filter(|c| c.level == Level::Warn).count();
        let headline = match (fails, warns) {
            (0, 0) => Callout::success(format!("All {} checks passed", checks.len())),
            (0, w) => Callout::warning(format!("{w} warning(s), no failure")),
            (f, w) => Callout::error(format!("{f} check(s) failed, {w} warning(s)")),
        };
        ui.print(&headline);
        let mut table = Table::new(&[
            Col::new("CHECK"),
            Col::new("STATUS"),
            Col::new("DETAIL").flex(1).min(10),
        ]);
        for c in &checks {
            table.push(vec![
                Cell::new(c.name),
                Cell::new(ui.render(&Status::level(c.level.into()))),
                Cell::new(c.detail.as_str()),
            ]);
        }
        ui.print(&table);
        let first = checks
            .iter()
            .find(|c| c.level == Level::Fail)
            .or_else(|| checks.iter().find(|c| c.level == Level::Warn));
        if let Some(next) = first.and_then(|c| next_for(c.name)) {
            ui.line(&next_line(ui.mode(), next));
        }
    });
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
