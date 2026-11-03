//! Command line definition (clap derive). Every command accepts `--json`.

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "ttm",
    version,
    about = "Command line for toantranmei.com: sign in, workspaces, tokens, MCP setup",
    long_about = "Command line for toantranmei.com.\n\nHost order: --host > TTM_HOST > profile > https://toantranmei.com.\n\
                  Token order: TTM_TOKEN > OS keyring > credentials.toml.\n\nDocs: https://toantranmei.com/help/ttm"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Server URL (also TTM_HOST), e.g. http://localhost:3000.
    #[arg(long, global = true, value_name = "URL")]
    pub host: Option<String>,
    /// Profile in config.toml (also TTM_PROFILE).
    #[arg(long, global = true, value_name = "NAME")]
    pub profile: Option<String>,
    /// Machine-readable JSON on stdout.
    #[arg(long, global = true)]
    pub json: bool,
    /// More logs on stderr (-v, -vv).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Sign in (browser on this machine by default; --device for SSH/containers; --token for a PAT).
    Login(LoginArgs),
    /// Sign out: revoke the token on the server and remove it from this machine.
    Logout,
    /// Show the signed-in user.
    Whoami,
    /// Host, profile, token and workspace in use.
    Status,
    /// Check host, discovery, token, server version, keyring, git and claude.
    Doctor,
    /// List, switch or show workspaces.
    #[command(subcommand)]
    Workspace(WorkspaceCommand),
    /// Personal access tokens (managed on the web: opens <host>/me/ket-noi).
    #[command(subcommand)]
    Token(TokenCommand),
    /// MCP client setup.
    #[command(subcommand)]
    Mcp(McpCommand),
    /// Download and install the latest release (SHA-256 checked).
    SelfUpdate(SelfUpdateArgs),
    /// Print a shell completion script.
    Completion {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Args)]
pub struct LoginArgs {
    /// Device flow: print a code to enter at <host>/activate (no browser needed here).
    #[arg(long, conflicts_with = "token")]
    pub device: bool,
    /// Save an existing personal access token; `-` reads it from stdin.
    #[arg(long, value_name = "PAT")]
    pub token: Option<String>,
    /// Space-separated scopes to request.
    #[arg(long, default_value = ttm_api::oauth::DEFAULT_SCOPES)]
    pub scope: String,
    /// Do not try to open a browser; just print the URL.
    #[arg(long)]
    pub no_browser: bool,
}

#[derive(Debug, Subcommand)]
pub enum WorkspaceCommand {
    /// Workspaces you belong to.
    List,
    /// Make a workspace current for this profile (id, slug or name).
    Switch { workspace: String },
    /// The current workspace of this profile.
    Current,
}

#[derive(Debug, Subcommand)]
pub enum TokenCommand {
    /// Open the token list on the web (the API does not list tokens to Bearer clients).
    List,
    /// Open the web page that creates a token (shown once there).
    Create {
        /// Scopes to preselect (comma or space separated).
        #[arg(long)]
        scope: Option<String>,
        /// Lifetime, e.g. 30d (7d, 30d, 90d, 365d).
        #[arg(long, value_name = "DAYS")]
        expires: Option<String>,
        /// Token name to prefill.
        #[arg(long)]
        name: Option<String>,
    },
    /// Open the web page to revoke a token.
    Revoke { id: Option<String> },
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Point an MCP client at `ttm-mcp` (config files are backed up and merged, never overwritten).
    Install(McpInstallArgs),
}

#[derive(Debug, Args)]
pub struct McpInstallArgs {
    #[arg(long, value_enum)]
    pub client: McpClient,
    /// Server name in the client config.
    #[arg(long, default_value = "ttm")]
    pub name: String,
    /// Path or name of the ttm-mcp binary (default: next to ttm, else `ttm-mcp`).
    #[arg(long, value_name = "PATH")]
    pub command: Option<String>,
    /// Comma-separated toolsets passed as --toolsets.
    #[arg(long)]
    pub toolsets: Option<String>,
    /// Show what would change without writing.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpClient {
    ClaudeCode,
    ClaudeDesktop,
    Cursor,
    Codex,
}

#[derive(Debug, Args)]
pub struct SelfUpdateArgs {
    /// Only report whether a newer version exists.
    #[arg(long)]
    pub check: bool,
    /// Install this version (e.g. 0.2.0) instead of the latest.
    #[arg(long)]
    pub version: Option<String>,
    /// GitHub repository to download from.
    #[arg(long, default_value = "toantranmei/ttm", hide = true)]
    pub repo: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn global_flags_after_subcommand() {
        let cli =
            Cli::try_parse_from(["ttm", "whoami", "--json", "--host", "http://localhost:3000"]).unwrap();
        assert!(cli.global.json);
        assert_eq!(cli.global.host.as_deref(), Some("http://localhost:3000"));
    }

    #[test]
    fn login_variants() {
        let cli = Cli::try_parse_from(["ttm", "login", "--device"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Login(LoginArgs { device: true, .. })
        ));
        assert!(Cli::try_parse_from(["ttm", "login", "--device", "--token", "x"]).is_err());
    }

    #[test]
    fn mcp_install_clients() {
        let cli = Cli::try_parse_from(["ttm", "mcp", "install", "--client", "claude-desktop"]).unwrap();
        let Command::Mcp(McpCommand::Install(args)) = cli.command else {
            panic!()
        };
        assert_eq!(args.client, McpClient::ClaudeDesktop);
        assert!(Cli::try_parse_from(["ttm", "mcp", "install", "--client", "vim"]).is_err());
    }
}
