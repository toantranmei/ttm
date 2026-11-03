//! Command dispatch.

mod doctor;
mod login;
mod logout;
mod status;
mod whoami;

use crate::app::App;
use crate::cli::{Cli, Command};
use anyhow::Result;
use std::process::ExitCode;

pub async fn run(cli: Cli) -> Result<ExitCode> {
    let app = App::load(&cli.global)?;
    match cli.command {
        Command::Login(args) => login::run(app, args).await,
        Command::Logout => logout::run(app).await,
        Command::Whoami => whoami::run(app).await,
        Command::Status => status::run(app).await,
        Command::Doctor => doctor::run(app).await,
        other => anyhow::bail!("`{other:?}` is not implemented yet"),
    }
}
