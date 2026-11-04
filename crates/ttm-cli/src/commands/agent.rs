//! `ttm agent run <key> | logs <run> [-f] | stop <run>`.

use super::runner::{ensure_registered, executor};
use crate::app::App;
use crate::cli::AgentCommand;
use crate::output::emit;
use anyhow::Result;
use serde_json::json;
use std::process::ExitCode;
use std::time::Duration;
use ttm_runner::RunnerConfig;

pub async fn run(app: App, command: AgentCommand) -> Result<ExitCode> {
    match command {
        AgentCommand::Run { key } => {
            let mut config = RunnerConfig::load(&app.ctx.paths.config_dir)?;
            let (runner_id, runner_client, _, _) = ensure_registered(&app, &mut config).await?;
            let (user, _) = app.api().await?;
            let job = user.start_task_run(&key, &runner_id).await?;
            eprintln!("Run {} for {} on branch {}", job.run_id, job.task_key, job.branch);
            let exec = executor(
                &app,
                ttm_runner::SharedClient::new(runner_client),
                config,
                !app.json,
            )?;
            let (tx, rx) = tokio::sync::watch::channel(false);
            tokio::spawn(async move {
                let _ = tokio::signal::ctrl_c().await;
                let _ = tx.send(true);
            });
            let outcome = exec.run(job, rx).await;
            let out = json!({
                "run_id": outcome.run_id, "status": outcome.status, "branch": outcome.branch,
                "worktree": outcome.worktree, "pr_url": outcome.pr_url, "paused_until": outcome.paused_until
            });
            let ok = outcome.status == "succeeded";
            emit(&app, &out, |_ui| {
                println!("Run {}: {}", outcome.run_id, outcome.status);
                if let Some(w) = &outcome.worktree {
                    println!("worktree: {}", w.display());
                }
                if let Some(pr) = &outcome.pr_url {
                    println!("pull request: {pr}");
                }
            });
            return Ok(if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE });
        }
        AgentCommand::Logs { run, follow } => {
            let (client, _) = app.api().await?;
            // `since` is the id of the last event printed.
            let mut last: Option<i64> = None;
            loop {
                let events = client.run_events(&run, last).await?;
                for e in &events {
                    if app.json {
                        println!("{}", serde_json::to_string(e)?);
                    } else {
                        println!("{} [{}] {}", e.at.as_deref().unwrap_or(""), e.kind, e.message);
                    }
                }
                if let Some(id) = events.iter().filter_map(|e| e.id).max() {
                    last = Some(id);
                }
                if !follow {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
        AgentCommand::Stop { run } => {
            let (client, _) = app.api().await?;
            client.stop_run(&run).await?;
            emit(&app, &json!({ "run_id": run, "stopping": true }), |_ui| {
                println!("Stop requested for run {run}.")
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}
