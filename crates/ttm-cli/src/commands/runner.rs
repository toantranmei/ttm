//! `ttm runner up | down | status | config | install-service`.

use crate::app::App;
use crate::cli::{RunnerCommand, RunnerConfigArgs};
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

/// The runner's own token is stored apart from the user's (`<profile>#runner@<host>`), and never read from TTM_TOKEN.
fn runner_store(app: &App) -> (CredentialStore, String) {
    let use_keyring = std::env::var("TTM_NO_KEYRING").map_or(true, |v| v.is_empty() || v == "0");
    (
        CredentialStore::new(&app.ctx.paths, use_keyring, None),
        format!("{}#runner", app.ctx.profile_name),
    )
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
        let reg = self
            .user
            .register_runner(&self.body)
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
        },
    };
    let fresh =
        |t: &StoredToken| !ttm_runner::connection::needs_renewal(t.expires_at, OffsetDateTime::now_utc());
    let (id, client, expires) = match (config.runner_id.clone(), saved) {
        (Some(id), Some(token)) if fresh(&token) => {
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
            eprintln!("Runner {runner_id} up: {capacity} agent(s) at once. Ctrl-C to stop.");
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
            emit(app.json, &json!({ "stopped": stopped, "pid": pid }), || {
                if stopped {
                    println!("Runner stopped.");
                } else {
                    println!(
                        "No running runner found (a service restarts it; uninstall the service to keep it off)."
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
                "worktree_root": config.worktree_root(app.ctx.config.worktree_root()),
            });
            emit(app.json, &out, || {
                println!(
                    "runner:    {} ({})",
                    config.name,
                    config.runner_id.as_deref().unwrap_or("not registered")
                );
                println!("running:   {}", if pid.is_some_and(alive) { "yes" } else { "no" });
                println!("capacity:  {}", capacity::effective(config.max_parallel));
                println!("harnesses: {}", harness::installed().join(", "));
                for (k, v) in &config.repos {
                    println!("repo:      {k} = {}", v.display());
                }
            });
        }
        RunnerCommand::Config(args) => {
            let mut config = RunnerConfig::load(&config_dir)?;
            apply_config(&mut config, args)?;
            config.save(&config_dir)?;
            emit(app.json, &config, || {
                println!("Saved {}", RunnerConfig::path(&config_dir).display())
            });
        }
        RunnerCommand::InstallService => {
            let file = install_service(&app)?;
            emit(app.json, &json!({ "installed": file }), || {
                println!("Runner service installed: {file}")
            });
        }
    }
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

fn install_service(app: &App) -> Result<String> {
    let ttm = std::env::current_exe()?;
    let profile = app.profile_flag.then_some(app.ctx.profile_name.as_str());
    let home = ttm_core::paths::home_dir().context("no home directory")?;
    if cfg!(target_os = "macos") {
        let path = service::launchd_path(&home);
        write(
            &path,
            &service::launchd_plist(&ttm, &app.ctx.paths.cache_dir, profile),
        )?;
        run_quiet("launchctl", &["unload", &path.display().to_string()]);
        run_checked("launchctl", &["load", "-w", &path.display().to_string()])?;
        Ok(path.display().to_string())
    } else if cfg!(windows) {
        let args = service::schtasks_args(&ttm, profile);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        run_checked("schtasks", &refs)?;
        Ok(format!("Task Scheduler: {}", service::WINDOWS_TASK))
    } else {
        let path = service::systemd_path(&home);
        write(&path, &service::systemd_unit(&ttm, profile))?;
        run_checked("systemctl", &["--user", "daemon-reload"])?;
        run_checked("systemctl", &["--user", "enable", "--now", service::SYSTEMD_UNIT])?;
        Ok(path.display().to_string())
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
}
