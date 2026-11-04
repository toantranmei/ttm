//! `ttm agent list | show | create | edit | pause | resume | archive | skills add|rm | mcp add|rm` (S3 agents v2).

use super::notes::read_input;
use crate::app::App;
use crate::cli::{AgentCommand, AgentFields, AgentMcpCommand, AgentSkillsCommand};
use crate::output::emit;
use anyhow::{Result, anyhow, bail};
use serde_json::json;
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::agents::{Agent, AgentInput, AgentMcp, SkillPin};
use ttm_ui::{
    Ask, Badge, Callout, Cell, Choice, Col, Header, KeyValue, Status, Table, Token, Ui, UsageError, next_line,
};

const FEATURE: &str = "agent_v2";
const WHAT: &str = "agents v2";

pub async fn run(app: App, command: AgentCommand) -> Result<ExitCode> {
    let (client, _) = app.api_for(FEATURE, WHAT).await?;
    match command {
        AgentCommand::List { ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let agents = client.agents(&ws).await?;
            emit(&app, &agents, |ui| {
                ui.line(&format!("{} agent(s)", agents.len()));
                ui.print(&agent_table(ui, &agents));
                ui.line(&next_line(
                    ui.mode(),
                    match agents.first() {
                        Some(_) => "ttm agent show <handle>",
                        None => "ttm agent create",
                    },
                ));
            });
        }
        AgentCommand::Show { agent, ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let found = find(&client, &ws, &agent).await?;
            emit(&app, &found, |ui| show(ui, &found));
        }
        AgentCommand::Create { preset, fields, ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            create(&app, &client, &ws, preset, fields).await?;
        }
        AgentCommand::Edit {
            agent,
            fields,
            git_email,
            git_name,
            bio,
            ws,
        } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let found = find(&client, &ws, &agent).await?;
            let mut input = input_of(&fields)?;
            input.git_email = git_email;
            input.git_name = git_name;
            input.bio = bio;
            if input == AgentInput::default() && fields.skills.is_empty() {
                return Err(UsageError::missing_flag("--name, --role, --model, … ", false)
                    .fix("pass at least one field to change (ttm agent edit --help)")
                    .into());
            }
            let updated = if input == AgentInput::default() {
                found.clone()
            } else {
                client.update_agent(&ws, &found.id, &input).await?
            };
            if !fields.skills.is_empty() {
                let pins = resolve_skills(&client, &ws, &fields.skills).await?;
                client.put_agent_skills(&ws, &found.id, &pins).await?;
            }
            emit(&app, &updated, |ui| {
                ui.print(
                    &Callout::success(format!("Updated {}", updated.label()))
                        .next(format!("ttm agent show {}", updated.label())),
                )
            });
        }
        AgentCommand::Pause { agent, ws } => {
            status(&app, &client, ws.workspace, &agent, "paused", "Pause").await?
        }
        AgentCommand::Resume { agent, ws } => {
            status(&app, &client, ws.workspace, &agent, "active", "Resume").await?
        }
        AgentCommand::Archive { agent, ws } => {
            status(&app, &client, ws.workspace, &agent, "archived", "Archive").await?
        }
        AgentCommand::Skills(cmd) => skills(&app, &client, cmd).await?,
        AgentCommand::Mcp(cmd) => mcp(&app, &client, cmd).await?,
        AgentCommand::Run { .. } | AgentCommand::Logs { .. } | AgentCommand::Stop { .. } => {
            unreachable!("runs are handled by agent::run")
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The agent named by id, handle (`@dev` or `dev`) or name.
pub async fn find(client: &Client, ws: &str, needle: &str) -> Result<Agent> {
    client
        .agents(ws)
        .await?
        .into_iter()
        .find(|a| a.matches(needle))
        .ok_or_else(|| anyhow!("no agent `{needle}` in this workspace; see `ttm agent list`"))
}

/// `🤖 DEV · Code ✦ Mei-er` (owner's badge).
pub fn agent_name(ui: &Ui, a: &Agent) -> String {
    let mut s = format!("{} {}", ui.render(&Badge::agent()), a.name);
    if let Some(b) = Badge::of_tier(a.owner_membership.as_ref().map(|m| m.tier.as_str())) {
        s.push(' ');
        s.push_str(&ui.render(&b));
    }
    s
}

fn status_cell(ui: &Ui, status: Option<&str>) -> String {
    let s = status.unwrap_or("active");
    let glyph = match s {
        "active" => Status::phase(ttm_api::vocab::Phase::Doing),
        "paused" => Status::phase(ttm_api::vocab::Phase::Paused),
        "archived" => Status::phase(ttm_api::vocab::Phase::Done),
        _ => Status::of_category(s),
    };
    format!("{} {s}", ui.render(&glyph))
}

/// `ttm-tdd, ◐ ttm-open-pr` (◐ = a version waits for review).
fn skills_text(ui: &Ui, a: &Agent) -> String {
    if a.skills.is_empty() {
        return "-".into();
    }
    a.skills
        .iter()
        .map(|s| {
            if s.state.as_deref() == Some("pending") {
                format!("{} {}", ui.render(&Status::of_category("review")), s.slug)
            } else {
                s.slug.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn agent_table(ui: &Ui, agents: &[Agent]) -> Table {
    let mut t = Table::new(&[
        Col::new("NAME").flex(2).min(8),
        Col::new("HANDLE"),
        Col::new("ROLE").max(10),
        Col::new("MODEL").max(12),
        Col::new("SKILLS").flex(2).min(6),
        Col::new("MCP").right(),
        Col::new("STATUS"),
    ])
    .empty_text("No agents.");
    for a in agents {
        t.push(vec![
            Cell::new(agent_name(ui, a)),
            Cell::token(
                a.handle
                    .as_deref()
                    .map(|h| format!("@{h}"))
                    .unwrap_or_else(|| "-".into()),
                Token::Blue,
            ),
            Cell::new(a.role.as_deref().unwrap_or("-")),
            Cell::new(a.model.as_deref().unwrap_or("-")),
            Cell::new(skills_text(ui, a)),
            Cell::new(a.mcp().len().to_string()),
            Cell::new(status_cell(ui, a.status.as_deref())),
        ]);
    }
    t
}

fn show(ui: &Ui, a: &Agent) {
    ui.print(
        &Header::new(format!("{} · {}", a.name, a.label())).summary(format!(
            "{} · {} · {}",
            a.role.as_deref().unwrap_or("-"),
            a.model.as_deref().unwrap_or("-"),
            status_cell(ui, a.status.as_deref())
        )),
    );
    let kv = KeyValue::new()
        .row("id", a.id.as_str())
        .opt("bio", a.bio.as_deref())
        .row("git name", a.git_name.as_deref().unwrap_or(&a.name))
        .row("git email", a.git_email.as_deref().unwrap_or("default"))
        .opt("tags", (!a.tags.is_empty()).then(|| a.tags.join(", ")))
        .opt(
            "projects",
            a.extra.get("projects").and_then(|p| p.as_array()).map(|p| {
                p.iter()
                    .filter_map(|x| x.get("key").or(Some(x)).and_then(|k| k.as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            }),
        );
    ui.print(&kv);
    ui.blank();
    let mut skills = Table::new(&[
        Col::new("SKILL").flex(1).min(6),
        Col::new("VERSION"),
        Col::new("STATE"),
    ])
    .empty_text("No skills.");
    for s in &a.skills {
        let version = match (s.pinned_version, s.effective_version) {
            (Some(p), _) => format!("v{p} (pinned)"),
            (None, Some(e)) => format!("v{e}"),
            _ => "-".into(),
        };
        let state = match s.state.as_deref() {
            Some("pending") => Cell::token("pending review", Token::Yellow),
            Some("none") => Cell::token("no approved version", Token::Coral),
            _ => Cell::token("ok", Token::Teal),
        };
        skills.push(vec![Cell::new(s.slug.as_str()), Cell::new(version), state]);
    }
    ui.print(&skills);
    ui.blank();
    let mut mcp = Table::new(&[
        Col::new("MCP").flex(1).min(6),
        Col::new("TOOLS").flex(1).min(4),
        Col::new("REQUIRED"),
    ])
    .empty_text("No MCP servers.");
    for m in a.mcp() {
        mcp.push(vec![
            Cell::new(m.server.as_str()),
            Cell::new(if m.tools.is_empty() {
                "*".to_string()
            } else {
                m.tools.join(", ")
            }),
            Cell::new(if m.required { "yes" } else { "no" }),
        ]);
    }
    ui.print(&mcp);
    ui.line(&next_line(
        ui.mode(),
        &format!("ttm agent edit {} --model <model>", a.label()),
    ));
}

/// Fields given as flags.
fn input_of(f: &AgentFields) -> Result<AgentInput> {
    Ok(AgentInput {
        name: f.name.clone(),
        handle: f.handle.clone().map(|h| h.trim_start_matches('@').to_string()),
        role: f.role.clone(),
        model: f.model.clone(),
        max_budget_tokens: f.budget,
        daily_budget_tokens: f.daily_budget,
        max_turns: f.turns,
        permission: f.permission.clone(),
        instructions: f.instructions_file.as_deref().map(read_input).transpose()?,
        ..AgentInput::default()
    })
}

/// `slug` or `slug@3` → pins (skill ids from the workspace, built-in included).
async fn resolve_skills(client: &Client, ws: &str, specs: &[String]) -> Result<Vec<SkillPin>> {
    let all = client.skills(ws, None, None, true).await?;
    specs
        .iter()
        .map(|spec| {
            let (slug, version) = match spec.split_once('@') {
                Some((s, v)) => (
                    s,
                    Some(v.parse::<i64>().map_err(|_| anyhow!("bad version in `{spec}`"))?),
                ),
                None => (spec.as_str(), None),
            };
            let skill = all
                .iter()
                .find(|s| s.slug == slug || s.id == slug)
                .ok_or_else(|| anyhow!("no skill `{slug}`; see `ttm skill list --builtin`"))?;
            Ok(SkillPin {
                skill_id: skill.id.clone(),
                pinned_version: version,
            })
        })
        .collect()
}

#[allow(clippy::ptr_arg)] // validators take `&T`
fn valid_handle(h: &String) -> std::result::Result<(), String> {
    let ok = !h.is_empty()
        && h.len() <= 32
        && h.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && h.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err("use lower-case letters, digits and -, e.g. dev or qa-bot".into())
    }
}

#[allow(clippy::ptr_arg)] // validators take `&T`
fn not_empty(v: &String) -> std::result::Result<(), String> {
    if v.trim().is_empty() {
        Err("cannot be empty".into())
    } else {
        Ok(())
    }
}

async fn create(
    app: &App,
    client: &Client,
    ws: &str,
    preset: Option<String>,
    fields: AgentFields,
) -> Result<()> {
    let mut wizard = app.wizard();
    let presets = client.agent_presets(None).await.unwrap_or_default();
    let mut input = input_of(&fields)?;
    // A preset fills everything; without one (and without --name) a terminal is asked for one.
    let preset = if preset.is_some() || fields.name.is_some() || presets.is_empty() {
        preset
    } else {
        let mut choices: Vec<Choice<String>> = presets
            .iter()
            .map(|p| {
                Choice::new(p.code.clone(), p.name.clone()).hint(p.description.clone().unwrap_or_default())
            })
            .collect();
        choices.push(Choice::new("-".to_string(), "No preset: answer each question"));
        let picked = wizard.select(
            Ask::new("Start from a preset", "--preset").default("-".to_string()),
            &choices,
        )?;
        (picked != "-").then_some(picked)
    };
    if let Some(code) = &preset {
        let p = presets.iter().find(|p| &p.code == code);
        if p.is_none() && !presets.is_empty() {
            bail!(
                "no preset `{code}`; known: {}",
                presets
                    .iter()
                    .map(|p| p.code.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        input.preset = Some(code.clone());
        input.name = input.name.or_else(|| p.map(|p| p.name.clone()));
        input.handle = input.handle.or_else(|| p.and_then(|p| p.handle.clone()));
    } else {
        let name = wizard.text(
            Ask::new("Agent name", "--name")
                .value(input.name.clone())
                .validate(not_empty),
        )?;
        let suggested = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let handle = wizard.text(
            Ask::new("Handle (for @mentions)", "--handle")
                .value(input.handle.clone())
                .default(suggested)
                .validate(valid_handle),
        )?;
        let role = wizard.text(
            Ask::new("Work role", "--role")
                .value(input.role.clone())
                .default("DEV".into()),
        )?;
        let models = ["sonnet", "opus", "haiku"]
            .map(|m| Choice::new(m.to_string(), m))
            .to_vec();
        let model = wizard.select(
            Ask::new("Model", "--model")
                .value(input.model.clone())
                .default("sonnet".to_string()),
            &models,
        )?;
        input.name = Some(name);
        input.handle = Some(handle);
        input.role = Some(role);
        input.model = Some(model);
    }
    if let Some(p) = &input.preset {
        wizard.record("--preset", Some(p.clone()));
    }
    let label = input.name.clone().unwrap_or_default();
    if !wizard.confirm(&format!("Create agent {label}?"))? {
        bail!("cancelled");
    }
    wizard.print_equivalent("agent create");
    let created = client.create_agent(ws, &input).await?;
    if !fields.skills.is_empty() {
        let pins = resolve_skills(client, ws, &fields.skills).await?;
        client.put_agent_skills(ws, &created.id, &pins).await?;
    }
    emit(app, &created, |ui| {
        ui.print(
            &Callout::success(format!("Created agent {} ({})", created.name, created.label()))
                .details(fields.skills.iter().map(|s| format!("skill {s}")))
                .next(format!("ttm agent show {}", created.label())),
        )
    });
    Ok(())
}

async fn status(
    app: &App,
    client: &Client,
    ws: Option<String>,
    agent: &str,
    to: &str,
    verb: &str,
) -> Result<()> {
    let ws = app.workspace_id(ws.as_deref())?;
    let found = find(client, &ws, agent).await?;
    if !app.wizard().confirm(&format!("{verb} {}?", found.label()))? {
        bail!("cancelled");
    }
    let updated = client
        .update_agent(
            &ws,
            &found.id,
            &AgentInput {
                status: Some(to.into()),
                ..AgentInput::default()
            },
        )
        .await?;
    emit(app, &updated, |ui| {
        ui.print(&Callout::success(format!("{} is now {to}", updated.label())).next("ttm agent list"))
    });
    Ok(())
}

async fn skills(app: &App, client: &Client, cmd: AgentSkillsCommand) -> Result<()> {
    let (agent, skill, ws, add) = match cmd {
        AgentSkillsCommand::Add { agent, skill, ws } => (agent, skill, ws, true),
        AgentSkillsCommand::Rm { agent, skill, ws } => (agent, skill, ws, false),
    };
    let ws = app.workspace_id(ws.workspace.as_deref())?;
    let found = find(client, &ws, &agent).await?;
    let mut pins: Vec<SkillPin> = found
        .skills
        .iter()
        .map(|s| SkillPin {
            skill_id: s.skill_id.clone(),
            pinned_version: s.pinned_version,
        })
        .collect();
    let target = resolve_skills(client, &ws, std::slice::from_ref(&skill))
        .await?
        .remove(0);
    pins.retain(|p| p.skill_id != target.skill_id);
    if add {
        pins.push(target);
    }
    let out = client.put_agent_skills(&ws, &found.id, &pins).await?;
    emit(app, &out, |ui| {
        let verb = if add { "Added" } else { "Removed" };
        ui.print(
            &Callout::success(format!(
                "{verb} skill {skill} {} {}",
                if add { "to" } else { "from" },
                found.label()
            ))
            .next(format!("ttm agent show {}", found.label())),
        )
    });
    Ok(())
}

async fn mcp(app: &App, client: &Client, cmd: AgentMcpCommand) -> Result<()> {
    let (agent, server, ws) = match &cmd {
        AgentMcpCommand::Add {
            agent, server, ws, ..
        }
        | AgentMcpCommand::Rm { agent, server, ws } => (agent.clone(), server.clone(), ws.clone()),
    };
    let ws = app.workspace_id(ws.workspace.as_deref())?;
    let found = find(client, &ws, &agent).await?;
    let mut grants = found.mcp();
    // `local:<name>` stays as is; a workspace server is stored by id and must exist.
    let key = if let Some(local) = server.strip_prefix("local:") {
        format!("local:{local}")
    } else {
        let servers = client.mcp_servers(&ws).await?;
        let s = servers
            .iter()
            .find(|s| s.name == server || s.id == server)
            .ok_or_else(|| {
                anyhow!("no MCP server `{server}` in this workspace; see `ttm mcp-server list`")
            })?;
        if let AgentMcpCommand::Add { tools, .. } = &cmd {
            let known = s.tool_names();
            if let Some(bad) = tools
                .iter()
                .find(|t| *t != "*" && !known.is_empty() && !known.contains(t))
            {
                return Err(
                    UsageError::new("invalid_flag", format!("`{bad}` is not a tool of {}", s.name))
                        .field("--tools")
                        .fix(format!("known tools: {}", known.join(", ")))
                        .into(),
                );
            }
        }
        s.id.clone()
    };
    grants.retain(|g| g.server != key);
    let add = matches!(cmd, AgentMcpCommand::Add { .. });
    if let AgentMcpCommand::Add { tools, required, .. } = &cmd {
        grants.push(AgentMcp {
            server: key.clone(),
            tools: if tools.is_empty() {
                vec!["*".into()]
            } else {
                tools.clone()
            },
            required: *required,
        });
    }
    let updated = client
        .update_agent(
            &ws,
            &found.id,
            &AgentInput {
                profile: Some(json!({ "mcp": grants })),
                ..AgentInput::default()
            },
        )
        .await?;
    emit(app, &updated, |ui| {
        let verb = if add { "Granted" } else { "Removed" };
        ui.print(
            &Callout::success(format!("{verb} MCP {server} for {}", updated.label()))
                .next(format!("ttm agent show {}", updated.label())),
        )
    });
    Ok(())
}
