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
    /// Notee pages and databases: search, read, query, comments.
    #[command(subcommand)]
    Notes(NotesCommand),
    /// Link this repository to a Factory project.
    #[command(subcommand)]
    Project(ProjectCommand),
    /// Pull the plan into .ttm/plan and send local edits as a change request.
    Sync(SyncArgs),
    /// Factory tasks.
    #[command(subcommand)]
    Task(TaskCommand),
    /// Project rules for local agents (AGENTS.md / CLAUDE.md block, agent hooks).
    #[command(subcommand)]
    Rules(RulesCommand),
    /// Hook entry points called by coding agents (reads the hook JSON on stdin).
    #[command(subcommand)]
    Hook(HookCommand),
    /// Run agent jobs from the server on this machine.
    #[command(subcommand)]
    Runner(RunnerCommand),
    /// Agent runs: start one now, follow its log, stop it.
    #[command(subcommand)]
    Agent(AgentCommand),
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
pub enum NotesCommand {
    /// Find pages by title and content.
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Workspace id (default: the current one).
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Print a page as Notee-flavored Markdown (or a database schema).
    Get {
        /// Page or database id, or URL.
        id: String,
        /// Read one part of a truncated page.
        #[arg(long)]
        block: Option<String>,
        /// Write the Markdown to a file instead of stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<std::path::PathBuf>,
    },
    /// Query a database with SQL or a JSON filter file.
    Query {
        /// Database id or URL.
        database: String,
        #[arg(long, conflicts_with = "filter", required_unless_present = "filter")]
        sql: Option<String>,
        /// JSON file with `{"filter": …, "sorts": …}` or just the filter (`-` for stdin).
        #[arg(long, value_name = "FILE")]
        filter: Option<String>,
        /// Rows per page, 1 to 100 (server default 100).
        #[arg(long, value_name = "N", value_parser = page_size)]
        page_size: Option<u32>,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Comment threads of a page.
    Comments {
        page: String,
        /// Include resolved threads.
        #[arg(long)]
        all: bool,
    },
    /// Create a page from a Markdown file.
    Create(NotesCreateArgs),
    /// Edit a page: find/replace (--old/--new, repeatable) or replace the body (--replace -f file.md).
    Edit(NotesEditArgs),
    /// Comment on a page, anchored to a quote or on the whole page.
    Comment(NotesCommentArgs),
    /// Database rows.
    #[command(subcommand)]
    Rows(RowsCommand),
    /// Suggested edits on a page (pending only unless --all).
    Suggestions {
        page: String,
        #[arg(long)]
        all: bool,
    },
    /// Accept or reject a suggestion.
    Resolve {
        suggestion: String,
        #[arg(long, conflicts_with = "reject", required_unless_present = "reject")]
        accept: bool,
        #[arg(long)]
        reject: bool,
        /// Accept only these change ids (repeatable).
        #[arg(long = "change", requires = "accept")]
        changes: Vec<String>,
    },
}

#[derive(Debug, Args)]
pub struct NotesCreateArgs {
    /// Parent page id or URL.
    #[arg(long, value_name = "PAGE", required_unless_present_any = ["database", "workspace"])]
    pub parent: Option<String>,
    /// Create the page as a row of this database.
    #[arg(long, value_name = "DATABASE", conflicts_with_all = ["parent", "workspace"])]
    pub database: Option<String>,
    /// Create at the top of this workspace.
    #[arg(long, value_name = "WORKSPACE", conflicts_with = "parent")]
    pub workspace: Option<String>,
    /// Markdown file (`-` for stdin).
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: String,
    /// Title (default: the first `# ` heading, else the file name).
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub icon: Option<String>,
}

#[derive(Debug, Args)]
pub struct NotesEditArgs {
    /// Page id or URL.
    pub page: String,
    /// Text to find (exactly once unless --replace-all); pair with --new.
    #[arg(long = "old", value_name = "TEXT")]
    pub old: Vec<String>,
    /// Replacement for the --old at the same position.
    #[arg(long = "new", value_name = "TEXT")]
    pub new: Vec<String>,
    #[arg(long)]
    pub replace_all: bool,
    /// Replace the whole body with the file given by -f.
    #[arg(long, requires = "file", conflicts_with_all = ["old", "new"])]
    pub replace: bool,
    #[arg(short = 'f', long = "file", value_name = "FILE")]
    pub file: Option<String>,
    /// Propose the change instead of applying it.
    #[arg(long)]
    pub suggest: bool,
    /// Allow removing child pages or databases.
    #[arg(long)]
    pub allow_deleting_content: bool,
}

#[derive(Debug, Args)]
pub struct NotesCommentArgs {
    /// Page id or URL.
    pub page: String,
    /// Exact text of the page to anchor to (omit for a page comment).
    #[arg(long)]
    pub quote: Option<String>,
    #[arg(long, requires = "quote")]
    pub prefix: Option<String>,
    #[arg(long, requires = "quote")]
    pub suffix: Option<String>,
    /// Comment text.
    #[arg(short = 'm', long = "message")]
    pub message: String,
    /// Reply in this thread.
    #[arg(long, conflicts_with = "quote")]
    pub thread: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum RowsCommand {
    /// Create or update rows from a JSON file: `[{"id"?: …, "values": {…}}]` (`-` for stdin).
    Upsert {
        database: String,
        #[arg(short = 'f', long = "file")]
        file: String,
    },
    /// Delete rows.
    Delete {
        database: String,
        #[arg(required = true)]
        ids: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    /// Create a project on the server for this repository and link it.
    Init {
        /// Task key prefix, e.g. AUR.
        #[arg(long)]
        key: String,
        #[arg(long)]
        name: Option<String>,
        /// Repository URL (default: git remote origin).
        #[arg(long)]
        repo_url: Option<String>,
        /// Workspace id (default: the current one).
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Link this repository to an existing project (key or id) and pull its plan.
    Link { project: String },
    /// Open the project on the web.
    Open,
    /// Show the link and sync state.
    Status,
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Only report whether local and server differ (exit 1 when they do).
    #[arg(long, conflicts_with = "resolve")]
    pub check: bool,
    /// On conflicts, let a local agent merge the versions in a separate worktree.
    #[arg(long)]
    pub resolve: bool,
}

#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    /// Tasks of the linked project.
    List {
        /// Only this state (or category).
        #[arg(long)]
        state: Option<String>,
        /// Only tasks assigned to me.
        #[arg(long)]
        mine: bool,
    },
    Show {
        key: String,
    },
    /// Assign the task to yourself.
    Claim {
        key: String,
    },
    /// Move the task to done (approval gates still apply).
    Done {
        key: String,
    },
    /// Approve (or --reject) the pending transition of a task.
    Approve {
        key: String,
        #[arg(long)]
        reject: bool,
    },
    /// Create subtasks from a JSON file (`[{"title": …, "role": …, "estimate": …}]`).
    Split {
        key: String,
        #[arg(short = 'f', long = "file")]
        file: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum RulesCommand {
    /// Write the managed block in AGENTS.md and CLAUDE.md and install the agent hooks.
    Apply,
}

#[derive(Debug, Subcommand)]
pub enum HookCommand {
    /// PreToolUse(Bash): block pushes to main, --no-verify and non-conventional commits (exit 2).
    PreBash,
    /// Stop: require the project's test command before finishing (exit 2).
    Stop,
}

#[derive(Debug, Subcommand)]
pub enum RunnerCommand {
    /// Run in the foreground (Ctrl-C to stop).
    Up,
    /// Stop the runner started by `up` or the service.
    Down,
    /// Registration, capacity, harnesses, process state.
    Status,
    /// Change runner settings.
    Config(RunnerConfigArgs),
    /// Start the runner at login (launchd, systemd --user, Task Scheduler), with this shell's PATH.
    InstallService,
    /// Stop the runner service and remove it (plist, unit or scheduled task).
    UninstallService,
}

#[derive(Debug, Args)]
pub struct RunnerConfigArgs {
    /// Agents at once (default: from CPU and free RAM).
    #[arg(long)]
    pub max_parallel: Option<usize>,
    /// Take jobs for teammates' tasks too.
    #[arg(long)]
    pub accept_team_jobs: Option<bool>,
    /// Where worktrees go, e.g. ~/orca/workspaces.
    #[arg(long)]
    pub worktree_root: Option<String>,
    /// Days to keep finished worktrees.
    #[arg(long)]
    pub keep_days: Option<u32>,
    /// Local clone of a project: KEY=PATH (repeatable).
    #[arg(long = "repo", value_name = "KEY=PATH")]
    pub repos: Vec<String>,
    /// Machine name shown on the web.
    #[arg(long)]
    pub name: Option<String>,
    /// Merge the head of each project's merge queue locally and push it.
    #[arg(long)]
    pub auto_merge: Option<bool>,
}

#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    /// Run a task now on this machine and stream its activity.
    Run { key: String },
    /// Events of a run.
    Logs {
        run: String,
        /// Keep printing new events.
        #[arg(short, long)]
        follow: bool,
    },
    /// Stop a run (kill switch).
    Stop { run: String },
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
    fn notes_query_needs_sql_or_filter() {
        assert!(Cli::try_parse_from(["ttm", "notes", "query", "db"]).is_err());
        assert!(
            Cli::try_parse_from([
                "ttm", "notes", "query", "db", "--sql", "SELECT 1", "--filter", "f.json"
            ])
            .is_err()
        );
        assert!(Cli::try_parse_from(["ttm", "notes", "query", "db", "--sql", "SELECT * FROM rows"]).is_ok());
    }

    #[test]
    fn notes_edit_forms() {
        let cli = Cli::try_parse_from([
            "ttm",
            "notes",
            "edit",
            "p",
            "--old",
            "a",
            "--new",
            "b",
            "--suggest",
        ])
        .unwrap();
        let Command::Notes(NotesCommand::Edit(args)) = cli.command else {
            panic!()
        };
        assert_eq!((args.old.len(), args.new.len(), args.suggest), (1, 1, true));
        assert!(
            Cli::try_parse_from(["ttm", "notes", "edit", "p", "--replace"]).is_err(),
            "--replace needs -f"
        );
        assert!(
            Cli::try_parse_from(["ttm", "notes", "create", "-f", "a.md"]).is_err(),
            "needs a parent"
        );
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

/// `--page-size`: a whole number from 1 to 100 (the server's bounds).
fn page_size(value: &str) -> Result<u32, String> {
    match value.trim().parse::<i64>() {
        Ok(n @ 1..=100) => Ok(n as u32),
        Ok(n) => Err(format!(
            "{n} is out of range: --page-size takes 1 to 100 rows per page"
        )),
        Err(_) => Err(format!(
            "`{value}` is not a number: --page-size takes 1 to 100 rows per page"
        )),
    }
}

#[cfg(test)]
mod page_size_tests {
    use super::*;

    #[test]
    fn page_size_bounds() {
        assert_eq!(page_size("1"), Ok(1));
        assert_eq!(page_size("100"), Ok(100));
        assert!(page_size("0").unwrap_err().contains("1 to 100"));
        assert!(page_size("101").unwrap_err().contains("out of range"));
        assert!(page_size("-5").is_err());
        assert!(page_size("ten").unwrap_err().contains("not a number"));
        let err = Cli::try_parse_from([
            "ttm",
            "notes",
            "query",
            "db1",
            "--sql",
            "SELECT 1",
            "--page-size",
            "500",
        ])
        .unwrap_err()
        .to_string();
        assert!(err.contains("--page-size takes 1 to 100"), "{err}");
    }
}
