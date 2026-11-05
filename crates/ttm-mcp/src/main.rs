//! `ttm-mcp`: MCP server over stdio. Reads the token like `ttm` (TTM_TOKEN > keyring > credentials.toml); logs go to
//! stderr because stdout carries the protocol.

use anyhow::{Context as _, Result};
use clap::Parser;
use rmcp::ServiceExt;
use time::OffsetDateTime;
use ttm_api::Client;
use ttm_core::{Context, Discovery, Overrides, TokenSource};
use ttm_mcp_tools::{FactoryScope, HttpBackend, ToolSet, Toolset, Toolsets};

#[derive(Debug, Parser)]
#[command(
    name = "ttm-mcp",
    version,
    about = "MCP server (stdio) for toantranmei.com: workspace, Notee and Factory tools",
    long_about = "MCP server (stdio) for toantranmei.com.\n\nSign in first with `ttm login`, or set TTM_TOKEN.\n\
                  Add it to a client with `ttm mcp install --client claude-code|claude-desktop|cursor|codex`."
)]
struct Args {
    /// Server URL (also TTM_HOST).
    #[arg(long, value_name = "URL")]
    host: Option<String>,
    /// Profile in config.toml (also TTM_PROFILE).
    #[arg(long, value_name = "NAME")]
    profile: Option<String>,
    /// Extra toolsets, comma separated: notee, factory, admin (workspace is always on). Default: all. Inside an agent
    /// run (TTM_RUN_ID set) `admin` is always removed.
    #[arg(long, value_name = "LIST", env = "TTM_TOOLSETS")]
    toolsets: Option<String>,
    /// More logs on stderr (-v, -vv).
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_logging(args.verbose);
    let scope = FactoryScope::from_env();
    let toolsets = job_toolsets(args.toolsets.as_deref(), scope.run_id.is_some())?;
    let ctx = Context::load(&Overrides {
        host: args.host.clone(),
        profile: args.profile.clone(),
    })
    .context("loading the ttm configuration")?;
    let (client, features) = connect(&ctx).await?;
    let mut backend = HttpBackend::new(client, ctx.workspace()).with_factory_scope(scope);
    if let Some(features) = features {
        backend = backend.with_features(features);
    }
    // stdio is stateful: the client can be asked to confirm a staged change right away (elicitation).
    let server = ToolSet::new(backend, toolsets)
        .with_server_info("ttm", ttm_core::VERSION)
        .with_elicitation(true);
    tracing::info!(host = %ctx.host, profile = %ctx.profile_name, "ttm-mcp ready on stdio");
    let running = server
        .serve(rmcp::transport::stdio())
        .await
        .context("MCP handshake failed")?;
    running.waiting().await?;
    Ok(())
}

/// `--toolsets` (default all); an agent run never gets `admin`, even when asked.
fn job_toolsets(list: Option<&str>, in_run: bool) -> Result<Toolsets> {
    let toolsets = match list {
        Some(list) => Toolsets::parse(list).map_err(anyhow::Error::msg)?,
        None => Toolsets::all(),
    };
    if in_run && toolsets.contains(Toolset::Admin) {
        if list.is_some_and(|l| l.split(',').any(|p| p.trim().eq_ignore_ascii_case("admin"))) {
            tracing::warn!(
                "TTM_RUN_ID is set: the admin toolset is not available to an agent run; ignoring it"
            );
        }
        return Ok(toolsets.without(Toolset::Admin));
    }
    Ok(toolsets)
}

/// Discovery (falls back to `<host>/api/v1` when unreachable) and the stored token, refreshed when expired. Also
/// answers the server's `features` when discovery worked.
async fn connect(ctx: &Context) -> Result<(Client, Option<Vec<String>>)> {
    let probe = Client::new(format!("{}/api/v1", ctx.host), None)?;
    let (discovery, reached) =
        match ttm_core::discovery::discover(probe.http(), &ctx.host, &ctx.paths.cache_dir, false).await {
            Ok(d) => (d, true),
            Err(err) => {
                tracing::warn!("{err}; using default endpoints");
                (Discovery::default(), false)
            }
        };
    let features = reached.then(|| discovery.features.clone());
    if let Some(min) = discovery.upgrade_required(ttm_core::VERSION) {
        tracing::warn!("the server needs ttm-mcp >= {min}; run `ttm self-update`");
    }
    let api_base = discovery.api_base(&ctx.host);
    let token = match ctx.credentials.load(&ctx.profile_name, &ctx.host)? {
        None => {
            tracing::warn!(
                "not signed in to {}; run `ttm login` (tools will answer unauthorized)",
                ctx.host
            );
            None
        }
        Some((token, source))
            if source != TokenSource::Env && token.is_expired(OffsetDateTime::now_utc()) =>
        {
            match &token.refresh_token {
                Some(refresh) => {
                    let anon = Client::new(api_base.clone(), None)?;
                    match anon
                        .refresh_token(&discovery.token_endpoint(&ctx.host), refresh)
                        .await
                    {
                        Ok(fresh) => {
                            let stored = ttm_core::StoredToken {
                                access_token: fresh.access_token,
                                refresh_token: fresh.refresh_token.or(Some(refresh.clone())),
                                expires_at: fresh
                                    .expires_in
                                    .map(|s| OffsetDateTime::now_utc() + time::Duration::seconds(s)),
                                scope: fresh.scope,
                            };
                            ctx.credentials.save(&ctx.profile_name, &ctx.host, &stored)?;
                            Some(stored.access_token)
                        }
                        Err(err) => {
                            tracing::warn!(
                                "session expired and could not be renewed ({err}); run `ttm login`"
                            );
                            None
                        }
                    }
                }
                None => {
                    tracing::warn!("the saved token has expired; run `ttm login`");
                    None
                }
            }
        }
        Some((token, _)) => Some(token.access_token),
    };
    Ok((
        Client::new(api_base, token)?.with_cache_dir(&ctx.paths.cache_dir),
        features,
    ))
}

fn init_logging(verbose: u8) {
    let default = match verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_env("TTM_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_target(false)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn args_are_valid() {
        Args::command().debug_assert();
        let args = Args::try_parse_from([
            "ttm-mcp",
            "--host",
            "http://localhost:3000",
            "--toolsets",
            "notee",
        ])
        .unwrap();
        assert_eq!(args.toolsets.as_deref(), Some("notee"));
    }

    #[test]
    fn agent_runs_never_get_admin() {
        assert!(job_toolsets(None, false).unwrap().contains(Toolset::Admin));
        assert!(!job_toolsets(None, true).unwrap().contains(Toolset::Admin));
        let asked = job_toolsets(Some("factory,admin"), true).unwrap();
        assert!(asked.contains(Toolset::Factory) && !asked.contains(Toolset::Admin));
    }
}
