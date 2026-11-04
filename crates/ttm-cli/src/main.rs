//! `ttm`: the command line of toantranmei.com.

mod app;
mod cli;
mod commands;
mod output;
mod util;

use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    init_logging(cli.global.verbose);
    let json = cli.global.json;
    let ui = ttm_ui::Ui::detect(cli.global.plain, json);
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(err) => {
            output::print_error(
                &ui,
                json,
                &anyhow::anyhow!("cannot start the async runtime: {err}"),
            );
            return ExitCode::FAILURE;
        }
    };
    match runtime.block_on(commands::run(cli, ui.clone())) {
        Ok(code) => code,
        Err(err) => output::print_error(&ui, json, &err),
    }
}

/// Logs go to stderr. Default `warn` (deprecation notices), `-v` info, `-vv` debug; `TTM_LOG` overrides.
fn init_logging(verbose: u8) {
    let default = match verbose {
        0 => "warn",
        1 => "info,ttm=debug",
        _ => "debug",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_env("TTM_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .try_init();
}
