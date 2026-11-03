//! Command dispatch.

mod agent;
mod doctor;
mod hook;
mod login;
mod logout;
mod mcp;
mod notes;
mod project;
mod rules;
mod runner;
mod self_update;
mod status;
mod sync;
mod task;
mod token;
mod whoami;
mod workspace;

use crate::app::App;
use crate::cli::{Cli, Command};
use anyhow::Result;
use std::process::ExitCode;

pub async fn run(cli: Cli) -> Result<ExitCode> {
    // Needs no configuration, so it works even with a broken config file.
    if let Command::Completion { shell } = cli.command {
        let mut command = <Cli as clap::CommandFactory>::command();
        clap_complete::generate(shell, &mut command, "ttm", &mut std::io::stdout());
        return Ok(ExitCode::SUCCESS);
    }
    let app = App::load(&cli.global)?;
    match cli.command {
        Command::Login(args) => login::run(app, args).await,
        Command::Logout => logout::run(app).await,
        Command::Whoami => whoami::run(app).await,
        Command::Status => status::run(app).await,
        Command::Doctor => doctor::run(app).await,
        Command::Workspace(command) => workspace::run(app, command).await,
        Command::Token(command) => token::run(app, command).await,
        Command::SelfUpdate(args) => self_update::run(app, args).await,
        Command::Completion { .. } => Ok(ExitCode::SUCCESS),
        Command::Notes(command) => notes::run(app, command).await,
        Command::Project(command) => project::run(app, command).await,
        Command::Sync(args) => sync::run(app, args).await,
        Command::Task(command) => task::run(app, command).await,
        Command::Rules(command) => rules::run(app, command).await,
        Command::Hook(command) => hook::run(app, command).await,
        Command::Runner(command) => runner::run(app, command).await,
        Command::Agent(command) => agent::run(app, command).await,
        Command::Mcp(command) => mcp::run(app, command).await,
    }
}
