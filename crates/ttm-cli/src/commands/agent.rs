//! `ttm agent run <key> | logs <run> [-f] | stop <run>`.

use super::runner::{ensure_registered, executor};
use crate::app::App;
use crate::cli::AgentCommand;
use crate::output::{emit, emit_line, waiting};
use anyhow::Result;
use serde_json::json;
use std::process::ExitCode;
use std::time::Duration;
use ttm_runner::RunnerConfig;
use ttm_ui::{Callout, Token};

pub async fn run(app: App, command: AgentCommand) -> Result<ExitCode> {
    if !matches!(
        command,
        AgentCommand::Run { .. } | AgentCommand::Logs { .. } | AgentCommand::Stop { .. }
    ) {
        return super::agent_admin::run(app, command).await;
    }
    match command {
        AgentCommand::Run { key } => {
            let mut config = RunnerConfig::load(&app.ctx.paths.config_dir)?;
            let (runner_id, runner_client, _, _) = ensure_registered(&app, &mut config).await?;
            let (user, _) = app.api().await?;
            let spinner = waiting(&app.ui, &format!("Starting a run for {key}…"));
            let job = user.start_task_run(&key, &runner_id).await;
            spinner.clear();
            let job = job?;
            app.ui.eline(&app.ui.paint(
                Token::Muted,
                &format!("Run {} for {} on branch {}", job.run_id, job.task_key, job.branch),
            ));
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
            emit(&app, &out, |ui| {
                let headline = format!("Run {} {}", outcome.run_id, outcome.status);
                let mut callout = if ok {
                    Callout::success(headline)
                } else if outcome.paused_until.is_some() {
                    Callout::warning(headline)
                } else {
                    Callout::error(headline)
                };
                if let Some(w) = &outcome.worktree {
                    callout = callout.detail(format!("worktree {}", w.display()));
                }
                if let Some(pr) = &outcome.pr_url {
                    callout = callout.detail(format!("pull request {}", ui.paint(Token::Blue, pr)));
                }
                if let Some(until) = &outcome.paused_until {
                    callout = callout.detail(format!("agent usage limit reached; paused until {until}"));
                }
                let next = if ok {
                    format!("ttm task show {key}")
                } else {
                    format!("ttm agent logs {}", outcome.run_id)
                };
                ui.print(&callout.next(next));
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
                        // NDJSON, one event per line, unchanged from 0.1.6.
                        emit_line(&app.ui, e);
                    } else {
                        app.ui.line(&ttm_runner::say::event_line(&app.ui, e));
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
            emit(&app, &json!({ "run_id": run, "stopping": true }), |ui| {
                ui.print(
                    &Callout::success(format!("Stop requested for run {run}"))
                        .next(format!("ttm agent logs {run}")),
                )
            });
        }
        _ => unreachable!("handled by agent_admin"),
    }
    Ok(ExitCode::SUCCESS)
}
