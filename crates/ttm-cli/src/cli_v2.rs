//! Command line of the v2 commands (workflow, automation, agent admin, skill, mcp-server, cr, inbox, template).
//! Every wizard question has a flag; `--yes` takes defaults and confirms.

use clap::{Args, Subcommand, ValueEnum};

/// `--workspace` (default: the current one).
#[derive(Debug, Clone, Default, Args)]
pub struct WsArg {
    /// Workspace id (default: the current one).
    #[arg(long, value_name = "ID")]
    pub workspace: Option<String>,
}

/// `--project` (default: the project linked to this directory).
#[derive(Debug, Clone, Default, Args)]
pub struct ProjectArg {
    /// Project key or id (default: the linked project).
    #[arg(long, value_name = "KEY")]
    pub project: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AgentsChoice {
    /// Create the template's agents.
    Create,
    /// Reuse agents with the same role when the workspace has them.
    Reuse,
    /// No agents.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RunnerChoice {
    /// Use this machine as the runner of the project (writes runner.toml).
    This,
    None,
}

#[derive(Debug, Args)]
pub struct ProjectInitArgs {
    /// Project name (default: the directory name).
    #[arg(long)]
    pub name: Option<String>,
    /// Task key prefix, e.g. AUR (2 to 10 capitals or digits, starting with a letter).
    #[arg(long)]
    pub key: Option<String>,
    /// Workspace id (default: the current one).
    #[arg(long)]
    pub workspace: Option<String>,
    /// Repository URL (default: git remote origin).
    #[arg(long = "repo", alias = "repo-url", value_name = "URL")]
    pub repo_url: Option<String>,
    /// Provider of the repository: github, gitlab or local (default: from the URL's host). Needed for a GitHub
    /// Enterprise or other self-hosted host the URL does not name.
    #[arg(long, value_name = "PROVIDER", value_parser = ["github", "gitlab", "local"])]
    pub provider: Option<String>,
    /// professional, balanced, kanban, scrum, autopilot or a template slug.
    #[arg(long)]
    pub workflow: Option<String>,
    /// Agents of the template.
    #[arg(long, value_enum)]
    pub agents: Option<AgentsChoice>,
    /// Runner for the project.
    #[arg(long, value_enum)]
    pub runner: Option<RunnerChoice>,
}

#[derive(Debug, Subcommand)]
pub enum WorkflowCommand {
    /// States and transitions of the project's workflow.
    Show {
        #[command(flatten)]
        project: ProjectArg,
        /// A version number or `draft` (default: the version the project uses).
        #[arg(long)]
        version: Option<String>,
    },
    /// Check a workflow: `.ttm/workflow.yaml` with --file, else the draft on the server (exit 1 on issues).
    Validate {
        #[command(flatten)]
        project: ProjectArg,
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
    },
    /// What would happen to a task moved to a status.
    Simulate {
        task: String,
        /// Target status id.
        #[arg(long)]
        to: String,
        #[arg(long)]
        version: Option<String>,
        /// Roles of the actor (comma separated).
        #[arg(long, value_delimiter = ',')]
        roles: Vec<String>,
    },
    /// Changes between two versions (default: the project's version and the draft).
    Diff {
        #[command(flatten)]
        project: ProjectArg,
        from: Option<String>,
        to: Option<String>,
    },
    /// Publish the draft as a new version.
    Publish {
        #[command(flatten)]
        project: ProjectArg,
        /// Release note (≤ 280 characters).
        #[arg(long)]
        note: Option<String>,
    },
    /// Move the project to a newer workflow version (tasks are remapped as previewed).
    Upgrade {
        #[command(flatten)]
        project: ProjectArg,
        #[arg(long)]
        to: Option<i64>,
    },
    /// Copy the workflow into an editable one of this workspace.
    Copy {
        #[command(flatten)]
        project: ProjectArg,
        #[arg(long)]
        name: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum AutomationCommand {
    /// Rules of the project.
    List {
        #[command(flatten)]
        project: ProjectArg,
    },
    /// When / if / then of a rule.
    Show {
        rule: String,
        #[command(flatten)]
        project: ProjectArg,
    },
    /// Dry run a rule on a task (nothing changes).
    Test {
        rule: String,
        #[arg(long)]
        task: String,
        #[command(flatten)]
        project: ProjectArg,
    },
    Enable {
        rule: String,
        #[command(flatten)]
        project: ProjectArg,
    },
    Disable {
        rule: String,
        #[command(flatten)]
        project: ProjectArg,
    },
    /// Recent runs (of one rule or of the project).
    Runs {
        rule: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[command(flatten)]
        project: ProjectArg,
    },
}

/// Fields of `agent create` / `agent edit`.
#[derive(Debug, Clone, Default, Args)]
pub struct AgentFields {
    #[arg(long)]
    pub name: Option<String>,
    /// Mention handle, e.g. dev (default: from the name).
    #[arg(long)]
    pub handle: Option<String>,
    /// Work role, e.g. DEV, QA, UX.
    #[arg(long)]
    pub role: Option<String>,
    /// opus, sonnet, haiku or a model id.
    #[arg(long)]
    pub model: Option<String>,
    /// Token budget per run.
    #[arg(long)]
    pub budget: Option<i64>,
    /// Token budget per day.
    #[arg(long)]
    pub daily_budget: Option<i64>,
    /// Turns per run.
    #[arg(long)]
    pub turns: Option<i64>,
    /// Tool permission: read, edit, full or custom.
    #[arg(long)]
    pub permission: Option<String>,
    /// Skill slugs (comma separated).
    #[arg(long, value_delimiter = ',')]
    pub skills: Vec<String>,
    /// Instructions file (`-` for stdin).
    #[arg(long, value_name = "FILE")]
    pub instructions_file: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum AgentSkillsCommand {
    /// Attach a skill (`slug` or `slug@version` to pin).
    Add {
        agent: String,
        skill: String,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Detach a skill.
    Rm {
        agent: String,
        skill: String,
        #[command(flatten)]
        ws: WsArg,
    },
}

#[derive(Debug, Subcommand)]
pub enum AgentMcpCommand {
    /// Grant an MCP server (`<name>` of the workspace or `local:<name>` of a runner).
    Add {
        agent: String,
        server: String,
        /// Tools (comma separated, `*` for all).
        #[arg(long, value_delimiter = ',')]
        tools: Vec<String>,
        /// Run only on runners that can provide it (local MCP).
        #[arg(long)]
        required: bool,
        #[command(flatten)]
        ws: WsArg,
    },
    Rm {
        agent: String,
        server: String,
        #[command(flatten)]
        ws: WsArg,
    },
}

#[derive(Debug, Subcommand)]
pub enum SkillReviewCommand {
    /// Versions waiting for review.
    List {
        #[command(flatten)]
        ws: WsArg,
    },
    /// Approve `slug@version` (shows the diff with the current version first).
    Approve {
        target: String,
        #[arg(long)]
        note: Option<String>,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Reject `slug@version` (a note is required).
    Reject {
        target: String,
        #[arg(long)]
        note: String,
        #[command(flatten)]
        ws: WsArg,
    },
}

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    List {
        #[arg(long)]
        tag: Option<String>,
        /// Include built-in skills.
        #[arg(long)]
        builtin: bool,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Check a skill directory, upload it as a new version.
    Push {
        dir: std::path::PathBuf,
        /// Skill slug (default: the directory name).
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        note: Option<String>,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Download a version and unpack it (default: .ttm/skills/<slug> in a linked repo, else ./<slug>).
    Pull {
        slug: String,
        #[arg(long)]
        version: Option<i64>,
        #[arg(long)]
        dir: Option<std::path::PathBuf>,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Versions and review state.
    Show {
        slug: String,
        #[arg(long)]
        version: Option<i64>,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Review queue: list, approve, reject.
    #[command(subcommand)]
    Review(SkillReviewCommand),
    /// Import a skill from GitHub or GitLab.
    Import {
        #[arg(long)]
        git: String,
        #[arg(long)]
        path: String,
        #[arg(long = "ref", default_value = "main")]
        git_ref: String,
        #[arg(long)]
        slug: Option<String>,
        #[command(flatten)]
        ws: WsArg,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpTransport {
    Http,
    Sse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum McpAuth {
    /// No credential.
    None,
    /// One credential for the workspace (OAuth or a secret header).
    Workspace,
    /// Each delegator connects their own account.
    Delegator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ConnectFor {
    Workspace,
    Me,
}

#[derive(Debug, Subcommand)]
pub enum McpServerCommand {
    /// Remote MCP servers of the workspace.
    List {
        #[command(flatten)]
        ws: WsArg,
    },
    /// Add a remote MCP server (a secret only through --secret-env or a hidden prompt).
    Add {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long, value_enum)]
        transport: Option<McpTransport>,
        #[arg(long, value_enum)]
        auth: Option<McpAuth>,
        /// Header carrying the secret, e.g. Authorization.
        #[arg(long)]
        secret_header: Option<String>,
        /// Environment variable holding the secret value.
        #[arg(long, value_name = "VAR")]
        secret_env: Option<String>,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Connect by OAuth in the browser, then wait until it is connected.
    Connect {
        name: String,
        #[arg(long = "for", value_enum, default_value = "me")]
        for_: ConnectFor,
        /// Print the URL only.
        #[arg(long)]
        no_browser: bool,
        #[command(flatten)]
        ws: WsArg,
    },
    Remove {
        name: String,
        #[command(flatten)]
        ws: WsArg,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CrKind {
    Plan,
    Workflow,
    Rule,
    Agent,
    Skill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CrStatus {
    Pending,
    Applied,
    Rejected,
    All,
}

#[derive(Debug, Subcommand)]
pub enum CrCommand {
    /// Plan and configuration change requests.
    List {
        #[command(flatten)]
        project: ProjectArg,
        #[arg(long, value_enum)]
        kind: Option<CrKind>,
        #[arg(long, value_enum, default_value = "pending")]
        status: CrStatus,
    },
    Show {
        id: String,
    },
    Approve {
        id: String,
    },
    Reject {
        id: String,
        #[arg(long)]
        note: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum InboxSub {
    /// Mark notifications read.
    Read {
        ids: Vec<String>,
        #[arg(long, conflicts_with = "ids")]
        all: bool,
    },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct InboxArgs {
    #[command(subcommand)]
    pub sub: Option<InboxSub>,
    /// Only unread.
    #[arg(long)]
    pub unread: bool,
    /// Kinds (comma separated), e.g. factory.
    #[arg(long, value_delimiter = ',')]
    pub kind: Vec<String>,
    /// Keep printing new notifications (Ctrl-C to stop).
    #[arg(long)]
    pub watch: bool,
    /// Seconds between polls with --watch (at least 2).
    #[arg(long, default_value_t = 5)]
    pub interval: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TemplateKind {
    Workflow,
    Agent,
    Skill,
    RulePack,
    Bundle,
}

impl TemplateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TemplateKind::Workflow => "workflow",
            TemplateKind::Agent => "agent",
            TemplateKind::Skill => "skill",
            TemplateKind::RulePack => "rule_pack",
            TemplateKind::Bundle => "bundle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PublishFrom {
    Workflow,
    Agent,
    Skill,
}

#[derive(Debug, Subcommand)]
pub enum TemplateCommand {
    /// Search the public template library.
    List {
        #[arg(long, value_enum)]
        kind: Option<TemplateKind>,
        #[arg(long)]
        query: Option<String>,
    },
    /// Publish a workflow, agent or skill as a template.
    Publish {
        #[arg(long, value_enum)]
        from: PublishFrom,
        /// The workflow/agent/skill (id, handle or slug).
        source: String,
        /// Price in VND (set on the web: the command prints the link).
        #[arg(long)]
        price: Option<u64>,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[command(flatten)]
        ws: WsArg,
    },
    /// Preview then install a template into a workspace.
    Install {
        slug: String,
        #[command(flatten)]
        ws: WsArg,
    },
}
