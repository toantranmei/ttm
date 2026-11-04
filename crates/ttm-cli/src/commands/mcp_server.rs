//! `ttm mcp-server list | add | connect | remove` (S3 remote MCP servers of a workspace).

use crate::app::App;
use crate::cli::{ConnectFor, McpAuth, McpServerCommand, McpTransport};
use crate::output::{emit, waiting};
use crate::util::open_browser;
use anyhow::{Result, anyhow, bail};
use std::process::ExitCode;
use std::time::Duration;
use ttm_api::Client;
use ttm_api::mcp_servers::{McpServer, NewMcpServer, StaticSecret};
use ttm_ui::{Ask, Callout, Cell, Choice, Col, Status, Table, Token, Ui, next_line, truncate};

/// How long `connect` waits for the browser.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5 * 60);

async fn find(client: &Client, ws: &str, name: &str) -> Result<McpServer> {
    client
        .mcp_servers(ws)
        .await?
        .into_iter()
        .find(|s| s.name == name || s.id == name)
        .ok_or_else(|| anyhow!("no MCP server `{name}`; see `ttm mcp-server list`"))
}

fn status_cell(ui: &Ui, s: &McpServer) -> String {
    let st = s.status.as_deref().unwrap_or("active");
    let glyph = match st {
        "active" => Status::ok(),
        "disabled" => Status::phase(ttm_api::vocab::Phase::Paused),
        "needs_auth" => Status::warn(),
        _ => Status::fail(),
    };
    format!("{} {st}", ui.render(&glyph))
}

fn connected_text(s: &McpServer) -> String {
    match (s.connected.workspace, s.connected.me) {
        (true, true) => "workspace, me".into(),
        (true, false) => "workspace".into(),
        (false, true) => "me".into(),
        _ => "-".into(),
    }
}

#[allow(clippy::ptr_arg)] // validators take `&T`
fn valid_name(v: &String) -> std::result::Result<(), String> {
    let ok = !v.is_empty()
        && v.len() <= 32
        && v.chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && v.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && v != "ttm";
    if ok {
        Ok(())
    } else {
        Err("lower-case letters, digits and -, not `ttm`, e.g. linear".into())
    }
}

#[allow(clippy::ptr_arg)] // validators take `&T`
fn valid_url(v: &String) -> std::result::Result<(), String> {
    if v.starts_with("https://") || v.starts_with("http://") {
        Ok(())
    } else {
        Err("an http(s) URL".into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Word(&'static str);

impl std::fmt::Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

pub async fn run(app: App, command: McpServerCommand) -> Result<ExitCode> {
    let (client, _) = app.api_for("agent_v2", "remote MCP servers").await?;
    match command {
        McpServerCommand::List { ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let list = client.mcp_servers(&ws).await?;
            emit(&app, &list, |ui| {
                ui.line(&format!("{} MCP server(s)", list.len()));
                let mut t = Table::new(&[
                    Col::new("NAME"),
                    Col::new("URL").flex(2).min(10),
                    Col::new("AUTH"),
                    Col::new("STATUS"),
                    Col::new("TOOLS").right(),
                    Col::new("CONNECTED"),
                ])
                .empty_text("No MCP servers.");
                for s in &list {
                    t.push(vec![
                        Cell::token(s.name.as_str(), Token::Ink),
                        Cell::token(s.url.as_str(), Token::Blue),
                        Cell::new(s.auth.as_deref().unwrap_or("none")),
                        Cell::new(status_cell(ui, s)),
                        Cell::new(s.tool_names().len().to_string()),
                        Cell::new(connected_text(s)),
                    ]);
                }
                ui.print(&t);
                if let Some(e) = list.iter().find_map(|s| s.last_error.as_ref().map(|e| (s, e))) {
                    ui.print(
                        &Callout::warning(format!("{}: {}", e.0.name, truncate(e.1, 200)))
                            .next(format!("ttm mcp-server connect {}", e.0.name)),
                    );
                } else {
                    ui.line(&next_line(ui.mode(), "ttm agent mcp add <agent> <server>"));
                }
            });
        }
        McpServerCommand::Add {
            name,
            url,
            transport,
            auth,
            secret_header,
            secret_env,
            ws,
        } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let mut w = app.wizard();
            let name = w.text(
                Ask::new("Name (used as mcp__<name>__<tool>)", "--name")
                    .value(name)
                    .validate(valid_name),
            )?;
            let url = w.text(Ask::new("Server URL", "--url").value(url).validate(valid_url))?;
            let transport = w.select(
                Ask::new("Transport", "--transport")
                    .value(transport.map(|t| Word(if t == McpTransport::Sse { "sse" } else { "http" })))
                    .default(Word("http")),
                &[
                    Choice::new(Word("http"), "Streamable HTTP"),
                    Choice::new(Word("sse"), "Server-sent events (older servers)"),
                ],
            )?;
            let auth = w.select(
                Ask::new("Credentials", "--auth")
                    .value(auth.map(|a| {
                        Word(match a {
                            McpAuth::None => "none",
                            McpAuth::Workspace => "workspace",
                            McpAuth::Delegator => "delegator",
                        })
                    }))
                    .default(Word("none")),
                &[
                    Choice::new(Word("none"), "None"),
                    Choice::new(
                        Word("workspace"),
                        "One for the workspace (OAuth or a secret header)",
                    ),
                    Choice::new(
                        Word("delegator"),
                        "Each delegator connects their own account (OAuth)",
                    ),
                ],
            )?;
            // A secret header only with workspace credentials; its value never comes from a flag.
            let static_secret = match (auth.0, &secret_header, &secret_env) {
                ("workspace", Some(header), _) => Some(StaticSecret {
                    header: header.clone(),
                    value: w.secret_from_env("Secret value", "--secret-env", secret_env.as_deref())?,
                }),
                (_, None, Some(_)) => {
                    return Err(ttm_ui::UsageError::missing_flag("--secret-header", false)
                        .fix("pass --secret-header <Header> with --secret-env")
                        .into());
                }
                _ => None,
            };
            if !w.confirm(&format!("Add MCP server {name} ({url})?"))? {
                bail!("cancelled");
            }
            w.print_equivalent("mcp-server add");
            let created = client
                .create_mcp_server(&NewMcpServer {
                    workspace_id: ws,
                    name: name.clone(),
                    url,
                    transport: transport.0.into(),
                    auth: auth.0.into(),
                    static_secret,
                })
                .await?;
            emit(&app, &created, |ui| {
                let next = if auth.0 == "none" || secret_header.is_some() {
                    format!("ttm agent mcp add <agent> {name}")
                } else {
                    format!(
                        "ttm mcp-server connect {name} --for {}",
                        if auth.0 == "workspace" { "workspace" } else { "me" }
                    )
                };
                ui.print(
                    &Callout::success(format!("Added MCP server {name}"))
                        .detail(format!("{} tool(s) found", created.tool_names().len()))
                        .next(next),
                )
            });
        }
        McpServerCommand::Connect {
            name,
            for_,
            no_browser,
            ws,
        } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let server = find(&client, &ws, &name).await?;
            let for_word = if for_ == ConnectFor::Workspace {
                "workspace"
            } else {
                "me"
            };
            let url = client.mcp_connect(&server.id, for_word).await?;
            open_browser(&app.ui, &url, no_browser);
            let spinner = waiting(
                &app.ui,
                &format!("Waiting for {name} to be connected (5 minutes)…"),
            );
            let deadline = tokio::time::Instant::now() + CONNECT_TIMEOUT;
            let connected = loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let now = find(&client, &ws, &name).await?;
                let done = if for_ == ConnectFor::Workspace {
                    now.connected.workspace
                } else {
                    now.connected.me
                };
                if done {
                    break now;
                }
                if tokio::time::Instant::now() >= deadline {
                    spinner.clear();
                    bail!(
                        "{name} was not connected within 5 minutes; run `ttm mcp-server connect {name}` again"
                    );
                }
            };
            spinner.clear();
            emit(&app, &connected, |ui| {
                ui.print(
                    &Callout::success(format!("{name} is connected for {for_word}"))
                        .detail(format!("{} tool(s)", connected.tool_names().len()))
                        .next(format!("ttm agent mcp add <agent> {name}")),
                )
            });
        }
        McpServerCommand::Remove { name, ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let server = find(&client, &ws, &name).await?;
            let users: Vec<String> = client
                .agents(&ws)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|a| a.mcp().iter().any(|m| m.server == server.id))
                .map(|a| a.label())
                .collect();
            if !users.is_empty() {
                app.ui.eprint(&Callout::warning(format!(
                    "{} agent(s) use {name}: {}",
                    users.len(),
                    users.join(", ")
                )));
            }
            if !app.wizard().confirm(&format!("Remove MCP server {name}?"))? {
                bail!("cancelled");
            }
            client.delete_mcp_server(&server.id).await?;
            emit(
                &app,
                &serde_json::json!({ "removed": server.id, "agents": users }),
                |ui| {
                    ui.print(
                        &Callout::success(format!("Removed MCP server {name}")).next("ttm mcp-server list"),
                    )
                },
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}
