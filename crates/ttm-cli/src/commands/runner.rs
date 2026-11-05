//! `ttm runner up | down | status | config | install-service | uninstall-service`.

use crate::app::App;
use crate::cli::{RunnerCommand, RunnerConfigArgs, RunnerConfigSub, RunnerMcpCommand};
use crate::output::emit;
use crate::util::which;
use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use time::OffsetDateTime;
use ttm_api::Client;
use ttm_api::factory::RegisterRunner;
use ttm_core::{CredentialStore, StoredToken};
use ttm_runner::connection::MergeSetup;
use ttm_runner::{Executor, Runner, RunnerConfig, SharedClient, capacity, harness, service};
use ttm_ui::vocab::Phase;
use ttm_ui::{Callout, Cell, Col, KeyValue, Status, Table, Token, next_line};

/// The runner's own token is stored apart from the user's (`<profile>#runner@<host>`), and never read from TTM_TOKEN.
fn runner_store(app: &App) -> (CredentialStore, String) {
    let use_keyring = std::env::var("TTM_NO_KEYRING").map_or(true, |v| v.is_empty() || v == "0");
    (
        CredentialStore::new(&app.ctx.paths, use_keyring, None),
        format!("{}#runner", app.ctx.profile_name),
    )
}

/// Where the Claude Code OAuth token for teammates' jobs is kept (`<profile>#claude-team`, keyring or
/// credentials.toml), never read from TTM_TOKEN.
const CLAUDE_TEAM_HOST: &str = "https://claude.ai";

fn claude_team_store(app: &App) -> (CredentialStore, String) {
    let use_keyring = std::env::var("TTM_NO_KEYRING").map_or(true, |v| v.is_empty() || v == "0");
    (
        CredentialStore::new(&app.ctx.paths, use_keyring, None),
        format!("{}#claude-team", app.ctx.profile_name),
    )
}

/// The saved Claude Code token for teammates' jobs.
pub fn team_claude_token(app: &App) -> Option<String> {
    let (store, account) = claude_team_store(app);
    store
        .load(&account, CLAUDE_TEAM_HOST)
        .ok()
        .flatten()
        .map(|(t, _)| t.access_token)
        .filter(|t| !t.is_empty())
}

/// Everything needed to (re)register this machine, detached from `App` so the runner loop can renew its token.
#[derive(Clone)]
pub struct Registrar {
    user: Client,
    api_base: String,
    cache_dir: PathBuf,
    config_dir: PathBuf,
    store: CredentialStore,
    account: String,
    host: String,
    body: RegisterRunner,
}

impl Registrar {
    /// Registers (the server keeps the id for the same name and revokes the previous token) and saves the token
    /// with its expiry.
    pub async fn register(&self) -> Result<(String, Client, Option<OffsetDateTime>)> {
        // A renewal hours later announces runner.toml as it is now, not as it was at start.
        let body = match RunnerConfig::load(&self.config_dir) {
            Ok(current) => register_body(&self.body, &current),
            Err(_) => self.body.clone(),
        };
        let reg = self
            .user
            .register_runner(&body)
            .await
            .context("registering this runner")?;
        let expires = reg.expires_at.as_deref().and_then(parse_rfc3339);
        let token = StoredToken {
            expires_at: expires,
            ..StoredToken::bare(&reg.token)
        };
        self.store.save(&self.account, &self.host, &token)?;
        let mut config = RunnerConfig::load(&self.config_dir)?;
        if config.runner_id.as_deref() != Some(reg.runner_id.as_str()) {
            config.runner_id = Some(reg.runner_id.clone());
            config.save(&self.config_dir)?;
        }
        Ok((reg.runner_id, self.client(reg.token)?, expires))
    }

    fn client(&self, token: String) -> Result<Client> {
        Ok(Client::new(&self.api_base, Some(token))?.with_cache_dir(&self.cache_dir))
    }

    /// The renew hook of the runner loop.
    pub fn renew(&self) -> ttm_runner::connection::Renew {
        let me = self.clone();
        std::sync::Arc::new(move || {
            let me = me.clone();
            Box::pin(async move { me.register().await.map(|(_, client, expires)| (client, expires)) })
        })
    }
}

fn parse_rfc3339(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
}

/// The registration body with what `runner.toml` says now: local MCP servers and projects with a local clone.
pub fn register_body(base: &RegisterRunner, config: &RunnerConfig) -> RegisterRunner {
    RegisterRunner {
        local_mcp: ttm_runner::local_mcp::announce(&config.mcp),
        projects: Some(config.announced_projects()),
        ..base.clone()
    }
}

/// Registers this machine when there is no runner token or it is about to expire; returns the runner id, a client
/// using the runner token, its expiry, and the registrar for later renewals.
pub async fn ensure_registered(
    app: &App,
    config: &mut RunnerConfig,
) -> Result<(String, Client, Option<OffsetDateTime>, Registrar)> {
    let (store, account) = runner_store(app);
    let saved = store.load(&account, app.host())?.map(|(t, _)| t);
    let (user, discovery) = app.api().await?;
    let registrar = Registrar {
        user,
        api_base: discovery.api_base(app.host()),
        cache_dir: app.ctx.paths.cache_dir.clone(),
        config_dir: app.ctx.paths.config_dir.clone(),
        store,
        account,
        host: app.host().to_string(),
        body: RegisterRunner {
            name: config.name.clone(),
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            version: ttm_core::VERSION.into(),
            harnesses: harness::installed(),
            max_parallel: capacity::effective(config.max_parallel),
            accept_team_jobs: config.accept_team_jobs,
            features: ttm_runner::features(team_claude_token(app).is_some()),
            local_mcp: ttm_runner::local_mcp::announce(&config.mcp),
            projects: Some(config.announced_projects()),
        },
    };
    let fresh =
        |t: &StoredToken| !ttm_runner::connection::needs_renewal(t.expires_at, OffsetDateTime::now_utc());
    let (id, client, expires) = match (config.runner_id.clone(), saved) {
        (Some(id), Some(token)) if fresh(&token) && token_alive(&registrar, &id, &token).await? => {
            (id, registrar.client(token.access_token)?, token.expires_at)
        }
        _ => {
            let (id, client, expires) = registrar.register().await?;
            config.runner_id = Some(id.clone());
            (id, client, expires)
        }
    };
    Ok((id, client, expires, registrar))
}

/// Whether the saved runner token still speaks for this runner: a runner turned off from the web loses its channel
/// token (404 / 401 from then on), and a later `ttm runner up` must register again instead of polling 404 forever.
async fn token_alive(registrar: &Registrar, id: &str, token: &StoredToken) -> Result<bool> {
    let client = registrar.client(token.access_token.clone())?;
    let beat = ttm_api::factory::Heartbeat {
        running: Vec::new(),
        capacity: 0,
        paused_until: None,
        local_mcp: None,
        projects: None,
        features: None,
    };
    match client.heartbeat(id, &beat).await {
        Ok(_) => Ok(true),
        Err(err) if matches!(err.status(), Some(401 | 403 | 404)) => {
            tracing::info!("the saved runner token no longer works ({err}); registering again");
            Ok(false)
        }
        // offline or a server error: keep the token, the run loop retries
        Err(_) => Ok(true),
    }
}

/// `ttm-mcp` next to this `ttm`, else on PATH.
pub fn ttm_mcp_path() -> PathBuf {
    let exe = format!("ttm-mcp{}", std::env::consts::EXE_SUFFIX);
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join(&exe)))
        .filter(|p| p.exists())
        .or_else(|| which("ttm-mcp"))
        .unwrap_or_else(|| PathBuf::from(exe))
}

pub fn executor(app: &App, client: SharedClient, config: RunnerConfig, echo: bool) -> Result<Executor> {
    let worktree_root = config
        .worktree_root(app.ctx.config.worktree_root())
        .ok_or_else(|| anyhow!("no worktree root; set one with `ttm runner config --worktree-root <dir>`"))?;
    Ok(Executor {
        client,
        host: app.host().to_string(),
        config,
        worktree_root,
        cache_dir: app.ctx.paths.cache_dir.clone(),
        ttm_exe: std::env::current_exe()?,
        ttm_mcp_exe: ttm_mcp_path(),
        echo,
        ui: app.ui.clone(),
        config_dir: Some(app.ctx.paths.config_dir.clone()),
        team_claude_token: team_claude_token(app),
    })
}

fn pid_file(app: &App) -> PathBuf {
    app.ctx.paths.cache_dir.join("runner.pid")
}

pub async fn run(app: App, command: RunnerCommand) -> Result<ExitCode> {
    let config_dir = app.ctx.paths.config_dir.clone();
    match command {
        RunnerCommand::Up => {
            let mut config = RunnerConfig::load(&config_dir)?;
            if harness::installed().is_empty() {
                bail!("no agent harness found; install Claude Code (`claude`) first");
            }
            let (runner_id, client, token_expires, registrar) = ensure_registered(&app, &mut config).await?;
            let removed = ttm_runner::worktree::prune(&app.ctx.paths.cache_dir, config.keep_days);
            if removed > 0 {
                tracing::info!("removed {removed} old worktree(s)");
            }
            let capacity = capacity::effective(config.max_parallel);
            let merge = if config.auto_merge {
                let (user, _) = app.api().await?;
                Some(MergeSetup {
                    user,
                    repos: config.repos.clone(),
                    worktree_root: config
                        .worktree_root(app.ctx.config.worktree_root())
                        .ok_or_else(|| anyhow!("no worktree root"))?,
                })
            } else {
                None
            };
            let shared = SharedClient::new(client);
            let executor = Arc::new(executor(&app, shared.clone(), config, false)?);
            std::fs::create_dir_all(&app.ctx.paths.cache_dir)?;
            std::fs::write(pid_file(&app), std::process::id().to_string())?;
            app.ui.eprint(
                &Callout::success(format!("Runner {runner_id} up: {capacity} agent(s) at once"))
                    .detail("Ctrl-C to stop. Each run prints one line when it starts and when it ends."),
            );
            let (tx, rx) = tokio::sync::watch::channel(false);
            tokio::spawn(async move {
                wait_for_signal().await;
                let _ = tx.send(true);
            });
            let result = Runner {
                client: shared,
                runner_id,
                capacity,
                executor,
                token_expires,
                renew: Some(registrar.renew()),
                merge,
                config_file: Some(RunnerConfig::path(&config_dir)),
            }
            .run(rx)
            .await;
            let _ = std::fs::remove_file(pid_file(&app));
            result?;
        }
        RunnerCommand::Down => {
            let pid = std::fs::read_to_string(pid_file(&app))
                .ok()
                .and_then(|p| p.trim().parse::<u32>().ok());
            let stopped = match pid {
                Some(pid) => kill(pid),
                None => false,
            };
            emit(&app, &json!({ "stopped": stopped, "pid": pid }), |ui| {
                if stopped {
                    ui.print(&Callout::success("Runner stopped"));
                } else {
                    ui.print(
                        &Callout::info("No running runner found")
                            .detail("A service restarts it; uninstalling the service keeps it off.")
                            .next("ttm runner uninstall-service"),
                    );
                }
            });
        }
        RunnerCommand::Status => {
            let config = RunnerConfig::load(&config_dir)?;
            let pid = std::fs::read_to_string(pid_file(&app))
                .ok()
                .and_then(|p| p.trim().parse::<u32>().ok());
            let out = json!({
                "runner_id": config.runner_id,
                "name": config.name,
                "running": pid.is_some_and(alive),
                "pid": pid,
                "capacity": capacity::effective(config.max_parallel),
                "accept_team_jobs": config.accept_team_jobs,
                "harnesses": harness::installed(),
                "repos": config.repos,
                "projects": config.announced_projects(),
                "worktree_root": config.worktree_root(app.ctx.config.worktree_root()),
                "features": ttm_runner::features(team_claude_token(&app).is_some()),
                "local_mcp": ttm_runner::local_mcp::announce(&config.mcp),
            });
            let features = ttm_runner::features(team_claude_token(&app).is_some());
            emit(&app, &out, |ui| {
                let running = pid.is_some_and(alive);
                ui.line(&format!(
                    "{} {} {}",
                    ui.render(&if running {
                        Status::phase(Phase::Doing)
                    } else {
                        Status::phase(Phase::Paused)
                    }),
                    ui.paint(Token::Ink, &config.name),
                    if running { "running" } else { "not running" }
                ));
                let harnesses = harness::installed();
                let kv = KeyValue::new()
                    .row("runner", config.runner_id.as_deref().unwrap_or("not registered"))
                    .opt("pid", pid.filter(|_| running).map(|p| p.to_string()))
                    .row("capacity", capacity::effective(config.max_parallel).to_string())
                    .row("team jobs", if config.accept_team_jobs { "yes" } else { "no" })
                    .row(
                        "harnesses",
                        if harnesses.is_empty() {
                            "none".to_string()
                        } else {
                            harnesses.join(", ")
                        },
                    )
                    .opt(
                        "worktrees",
                        config
                            .worktree_root(app.ctx.config.worktree_root())
                            .map(|p| p.display().to_string()),
                    )
                    .row("features", features.join(", "));
                ui.print(&kv);
                if !features.iter().any(|f| f == ttm_runner::TEAM_ISOLATION) && config.accept_team_jobs {
                    ui.print(
                        &Callout::warning("Teammates' jobs run without isolation")
                            .detail("They get no skills or MCP servers until a Claude token is saved.")
                            .next("ttm runner config claude-token"),
                    );
                }
                ui.blank();
                ui.print(&mcp_table(&config.mcp));
                let mut repos = Table::new(&[Col::new("PROJECT"), Col::new("CLONE").flex(1).min(10)])
                    .empty_text("No local clones configured.");
                for (k, v) in &config.repos {
                    repos.push(vec![
                        Cell::token(k.as_str(), Token::Ink),
                        Cell::new(v.display().to_string()),
                    ]);
                }
                ui.blank();
                ui.print(&repos);
                let next = if config.repos.is_empty() {
                    Some("ttm runner config --repo <KEY>=<path>")
                } else if !running {
                    Some("ttm runner up")
                } else {
                    None
                };
                if let Some(next) = next {
                    ui.line(&next_line(ui.mode(), next));
                }
            });
        }
        RunnerCommand::Config(cmd) if cmd.sub.is_some() => match cmd.sub.expect("checked") {
            RunnerConfigSub::Mcp(sub) => return mcp_command(&app, &config_dir, sub),
            RunnerConfigSub::ClaudeToken { remove } => return claude_token(&app, remove),
        },
        RunnerCommand::Config(cmd) => {
            let args = cmd.args;
            let mut config = RunnerConfig::load(&config_dir)?;
            apply_config(&mut config, args)?;
            config.save(&config_dir)?;
            emit(&app, &config, |ui| {
                ui.print(&Callout::success(format!(
                    "Saved {}",
                    RunnerConfig::path(&config_dir).display()
                )));
                let kv = KeyValue::new()
                    .row("name", config.name.as_str())
                    .row(
                        "max parallel",
                        config
                            .max_parallel
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "auto".into()),
                    )
                    .row("team jobs", if config.accept_team_jobs { "yes" } else { "no" })
                    .row("auto merge", if config.auto_merge { "yes" } else { "no" })
                    .row("keep days", config.keep_days.to_string())
                    .row("repos", config.repos.len().to_string());
                ui.print(&kv);
            });
        }
        RunnerCommand::InstallService => {
            let (file, path) = install_service(&app)?;
            emit(&app, &json!({ "installed": file, "path": path }), |ui| {
                ui.print(
                    &Callout::success("Runner service installed: it starts at login")
                        .detail(format!("file {file}"))
                        .details(path.iter().map(|p| format!("PATH of the service: {p}")))
                        .next("ttm runner status"),
                );
            });
        }
        RunnerCommand::UninstallService => {
            let removed = uninstall_service()?;
            emit(&app, &json!({ "removed": removed }), |ui| match &removed {
                Some(what) => {
                    ui.print(&Callout::success("Runner service stopped and removed").detail(what.as_str()))
                }
                None => ui.print(&Callout::info("No runner service installed")),
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `NAME  COMMAND  ALLOW  TEAM JOBS` of the local MCP servers.
fn mcp_table(section: &ttm_runner::local_mcp::McpSection) -> Table {
    let mut table = Table::new(&[
        Col::new("LOCAL MCP"),
        Col::new("COMMAND").flex(2).min(8),
        Col::new("ALLOW").flex(1).min(6),
        Col::new("TEAM JOBS"),
    ])
    .empty_text("No local MCP servers (ttm runner config mcp add <name> --command <cmd>).");
    for (name, m) in &section.local {
        let command = std::iter::once(m.command.as_str())
            .chain(m.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        table.push(vec![
            Cell::token(name.as_str(), Token::Ink),
            Cell::new(command),
            Cell::new(if m.allow.is_empty() {
                "-".to_string()
            } else {
                m.allow.join(", ")
            }),
            Cell::new(if m.team_jobs { "yes" } else { "no" }),
        ]);
    }
    table
}

/// `K=env:VAR` / `K=value` pairs of `--env`.
pub fn parse_env(pairs: &[String]) -> Result<Vec<(String, String)>> {
    pairs
        .iter()
        .map(|p| {
            let (k, v) = p
                .split_once('=')
                .ok_or_else(|| anyhow!("--env takes K=env:VAR or K=value, got `{p}`"))?;
            let k = k.trim();
            if k.is_empty() || !k.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
                bail!("`{k}` is not an environment variable name");
            }
            Ok((k.to_string(), v.to_string()))
        })
        .collect()
}

/// `ttm runner config mcp …`: edits runner.toml with `toml_edit` (comments stay).
fn mcp_command(app: &App, config_dir: &Path, sub: RunnerMcpCommand) -> Result<ExitCode> {
    use ttm_runner::local_mcp::edit;
    let file = RunnerConfig::path(config_dir);
    let text = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .with_context(|| format!("reading {}", file.display()))?;
    let pick = |doc: &toml_edit::DocumentMut, names: &[String]| -> Vec<String> {
        if names.is_empty() {
            edit::names(doc)
        } else {
            names.to_vec()
        }
    };
    let (message, next_hint) = match sub {
        RunnerMcpCommand::List => {
            let config = RunnerConfig::load(config_dir)?;
            let out = json!({ "local_mcp": config.mcp.local });
            emit(app, &out, |ui| {
                ui.line(&format!("{} local MCP server(s)", config.mcp.local.len()));
                ui.print(&mcp_table(&config.mcp));
                ui.line(&ui.paint(
                    Token::Muted,
                    "Announced to the server: names and allowed agents only (never commands or environment).",
                ));
                if config.mcp.local.is_empty() {
                    ui.line(&next_line(
                        ui.mode(),
                        "ttm runner config mcp add <name> --command <cmd>",
                    ));
                }
            });
            return Ok(ExitCode::SUCCESS);
        }
        RunnerMcpCommand::Add {
            name,
            command,
            args,
            env,
        } => {
            let env = parse_env(&env)?;
            if env.iter().any(|(_, v)| !v.starts_with("env:") && looks_secret(v)) {
                bail!(
                    "put secrets in the environment and pass --env K=env:VAR; runner.toml never holds them"
                );
            }
            edit::add(&mut doc, &name, &command, &args, &env)?;
            (
                format!("Local MCP server `{name}` added"),
                Some(format!("ttm runner config mcp allow <agent> --mcp {name}")),
            )
        }
        RunnerMcpCommand::Allow { agent, mcp } => {
            let names = pick(&doc, &mcp);
            for n in &names {
                edit::allow(&mut doc, n, std::slice::from_ref(&agent))?;
            }
            (
                format!("@{} may use {}", agent.trim_start_matches('@'), names.join(", ")),
                None,
            )
        }
        RunnerMcpCommand::Deny { agent, mcp } => {
            let names = pick(&doc, &mcp);
            for n in &names {
                edit::deny(&mut doc, n, std::slice::from_ref(&agent))?;
            }
            (
                format!(
                    "@{} may no longer use {}",
                    agent.trim_start_matches('@'),
                    names.join(", ")
                ),
                None,
            )
        }
        RunnerMcpCommand::TeamJobs { name, state } => {
            edit::team_jobs(&mut doc, &name, state == "on")?;
            (
                format!(
                    "Teammates' jobs {} `{name}`",
                    if state == "on" { "may use" } else { "may not use" }
                ),
                None,
            )
        }
        RunnerMcpCommand::Remove { name } => {
            edit::remove(&mut doc, &name)?;
            (format!("Local MCP server `{name}` removed"), None)
        }
    };
    ttm_core::paths::write_atomic(&file, doc.to_string().as_bytes(), Some(0o644))?;
    let config = RunnerConfig::load(config_dir)?;
    emit(app, &json!({ "local_mcp": config.mcp.local }), |ui| {
        let mut callout = Callout::success(message).detail(format!(
            "{} · a running runner announces it at its next heartbeat",
            file.display()
        ));
        if let Some(next) = next_hint.clone().or_else(|| next_of(&config)) {
            callout = callout.next(next);
        }
        ui.print(&callout);
    });
    Ok(ExitCode::SUCCESS)
}

fn next_of(config: &RunnerConfig) -> Option<String> {
    config
        .mcp
        .local
        .iter()
        .find(|(_, m)| m.allow.is_empty())
        .map(|(n, _)| format!("ttm runner config mcp allow <agent> --mcp {n}"))
}

/// A literal `--env` value that looks like a token (long, no spaces, mixed letters and digits).
fn looks_secret(value: &str) -> bool {
    let v = value.trim();
    v.len() >= 20
        && !v.contains(' ')
        && v.chars().any(|c| c.is_ascii_digit())
        && v.chars().any(|c| c.is_ascii_alphabetic())
}

/// `ttm runner config claude-token`: reads the token from stdin and keeps it in the keyring.
fn claude_token(app: &App, remove: bool) -> Result<ExitCode> {
    let (store, account) = claude_team_store(app);
    if remove {
        let removed = store.delete(&account, CLAUDE_TEAM_HOST)?;
        emit(app, &json!({ "removed": removed }), |ui| {
            ui.print(&Callout::success(if removed {
                "Claude token removed"
            } else {
                "No Claude token was saved"
            }))
        });
        return Ok(ExitCode::SUCCESS);
    }
    // At a terminal the token is typed hidden; piped, it is read from stdin.
    let token = if app.ui.mode().interactive && !app.json {
        use ttm_ui::Term as _;
        ttm_ui::InquireTerm::for_mode(app.ui.mode()).secret("Token printed by `claude setup-token`:")?
    } else {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        line
    };
    let token = token.trim();
    if token.is_empty() {
        bail!("no token on stdin; run `claude setup-token` and pipe or paste its token");
    }
    let source = store.save(&account, CLAUDE_TEAM_HOST, &StoredToken::bare(token))?;
    emit(app, &json!({ "saved": true, "stored_in": source }), |ui| {
        ui.print(
            &Callout::success("Claude token saved for teammates' jobs")
                .detail(format!(
                    "stored in {source} · teammates' jobs run with an empty CLAUDE_CONFIG_DIR and this token"
                ))
                .next("ttm runner up"),
        )
    });
    Ok(ExitCode::SUCCESS)
}

pub fn apply_config(config: &mut RunnerConfig, args: RunnerConfigArgs) -> Result<()> {
    if let Some(n) = args.max_parallel {
        if n == 0 {
            bail!("--max-parallel must be at least 1");
        }
        config.max_parallel = Some(n);
    }
    if let Some(v) = args.accept_team_jobs {
        config.accept_team_jobs = v;
    }
    if let Some(root) = args.worktree_root {
        config.worktree_root = Some(root);
    }
    if let Some(days) = args.keep_days {
        config.keep_days = days;
    }
    if let Some(name) = args.name {
        config.name = name;
    }
    if let Some(auto) = args.auto_merge {
        config.auto_merge = auto;
    }
    for pair in args.repos {
        let (key, path) = pair
            .split_once('=')
            .ok_or_else(|| anyhow!("--repo expects KEY=PATH, got `{pair}`"))?;
        let path = ttm_core::worktree::expand_home(path);
        let path = std::fs::canonicalize(&path).unwrap_or(path);
        config.repos.insert(key.to_uppercase(), path);
    }
    Ok(())
}

async fn wait_for_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = async { match term.as_mut() { Some(t) => { t.recv().await; } None => std::future::pending::<()>().await } } => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

fn alive(pid: u32) -> bool {
    if cfg!(windows) {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
    } else {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|s| s.success())
    }
}

fn kill(pid: u32) -> bool {
    let status = if cfg!(windows) {
        std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string()])
            .status()
    } else {
        std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
    };
    status.is_ok_and(|s| s.success())
}

/// Writes and starts the service; returns what was installed and the `PATH` it carries (none on Windows, where the
/// task runs with the user's environment).
fn install_service(app: &App) -> Result<(String, Option<String>)> {
    let ttm = std::env::current_exe()?;
    let profile = app.profile_flag.then_some(app.ctx.profile_name.as_str());
    let home = ttm_core::paths::home_dir().context("no home directory")?;
    let path_var = service::service_path(std::env::var_os("PATH").as_deref(), &home);
    if cfg!(target_os = "macos") {
        let file = service::launchd_path(&home);
        write(
            &file,
            &service::launchd_plist(&ttm, &app.ctx.paths.cache_dir, profile, &path_var),
        )?;
        run_quiet("launchctl", &["unload", &file.display().to_string()]);
        run_checked("launchctl", &["load", "-w", &file.display().to_string()])?;
        Ok((file.display().to_string(), Some(path_var)))
    } else if cfg!(windows) {
        let args = service::schtasks_args(&ttm, profile);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_checked("schtasks", &refs)?;
        Ok((format!("Task Scheduler: {}", service::WINDOWS_TASK), None))
    } else {
        let file = service::systemd_path(&home);
        write(&file, &service::systemd_unit(&ttm, profile, &path_var))?;
        run_checked("systemctl", &["--user", "daemon-reload"])?;
        run_checked("systemctl", &["--user", "enable", "--now", service::SYSTEMD_UNIT])?;
        // `enable --now` does not restart a unit that was already running with the old definition.
        run_quiet("systemctl", &["--user", "restart", service::SYSTEMD_UNIT]);
        Ok((file.display().to_string(), Some(path_var)))
    }
}

/// Stops and removes the service; `None` when none was installed.
fn uninstall_service() -> Result<Option<String>> {
    let home = ttm_core::paths::home_dir().context("no home directory")?;
    if cfg!(target_os = "macos") {
        let file = service::launchd_path(&home);
        if !file.exists() {
            return Ok(None);
        }
        run_quiet("launchctl", &["unload", "-w", &file.display().to_string()]);
        std::fs::remove_file(&file).with_context(|| format!("removing {}", file.display()))?;
        Ok(Some(file.display().to_string()))
    } else if cfg!(windows) {
        let query = std::process::Command::new("schtasks")
            .args(["/Query", "/TN", service::WINDOWS_TASK])
            .output();
        if !query.is_ok_and(|o| o.status.success()) {
            return Ok(None);
        }
        let end = service::schtasks_end_args();
        run_quiet("schtasks", &end.iter().map(String::as_str).collect::<Vec<_>>());
        let delete = service::schtasks_delete_args();
        run_checked("schtasks", &delete.iter().map(String::as_str).collect::<Vec<_>>())?;
        Ok(Some(format!("Task Scheduler: {}", service::WINDOWS_TASK)))
    } else {
        let file = service::systemd_path(&home);
        if !file.exists() {
            return Ok(None);
        }
        run_quiet(
            "systemctl",
            &["--user", "disable", "--now", service::SYSTEMD_UNIT],
        );
        std::fs::remove_file(&file).with_context(|| format!("removing {}", file.display()))?;
        run_quiet("systemctl", &["--user", "daemon-reload"]);
        Ok(Some(file.display().to_string()))
    }
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn run_quiet(program: &str, args: &[&str]) {
    let _ = std::process::Command::new(program).args(args).output();
}

fn run_checked(program: &str, args: &[&str]) -> Result<()> {
    let out = std::process::Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("running {program}"))?;
    if !out.status.success() {
        bail!(
            "{program} {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_renewal_registers_with_the_current_runner_toml() {
        let base = RegisterRunner {
            name: "mac".into(),
            os: "macos".into(),
            arch: "aarch64".into(),
            version: "0.2.0".into(),
            harnesses: vec!["claude-code".into()],
            max_parallel: 2,
            accept_team_jobs: false,
            features: ttm_runner::features(false),
            local_mcp: vec![],
            projects: Some(vec!["AUR".into()]),
        };
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            RunnerConfig::path(dir.path()),
            "[repos]\nAUR = \"/code/aurora\"\nNOTE = \"/code/notes\"\n",
        )
        .unwrap();
        let body = register_body(&base, &RunnerConfig::load(dir.path()).unwrap());
        assert_eq!(
            body.projects,
            Some(vec!["AUR".to_string(), "NOTE".to_string()]),
            "the clone added since start"
        );
        assert_eq!(
            (body.name.as_str(), body.features.clone()),
            ("mac", base.features.clone())
        );
    }

    #[test]
    fn config_flags() {
        let mut c = RunnerConfig::default();
        let args = RunnerConfigArgs {
            max_parallel: Some(2),
            accept_team_jobs: Some(true),
            worktree_root: Some("~/orca/workspaces".into()),
            keep_days: Some(7),
            repos: vec!["aur=/tmp".into()],
            name: None,
            auto_merge: Some(true),
        };
        apply_config(&mut c, args).unwrap();
        assert_eq!(
            (c.max_parallel, c.accept_team_jobs, c.keep_days),
            (Some(2), true, 7)
        );
        assert!(c.repos.contains_key("AUR"));
        let bad = RunnerConfigArgs {
            max_parallel: None,
            accept_team_jobs: None,
            worktree_root: None,
            keep_days: None,
            repos: vec!["nope".into()],
            name: None,
            auto_merge: None,
        };
        assert!(apply_config(&mut c, bad).is_err());
    }

    #[test]
    fn env_pairs_and_secrets() {
        let ok = parse_env(&["GITHUB_TOKEN=env:GITHUB_TOKEN".into(), "MODE=ro".into()]).unwrap();
        assert_eq!(ok[0], ("GITHUB_TOKEN".into(), "env:GITHUB_TOKEN".into()));
        assert!(parse_env(&["nokey".into()]).is_err());
        assert!(parse_env(&["BAD-NAME=x".into()]).is_err());
        assert!(looks_secret("ghp_1234567890abcdefghijklmnop"));
        assert!(!looks_secret("ro") && !looks_secret("read only mode 12345678901"));
    }
}
