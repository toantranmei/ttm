---
title: Connecting MCP
description: Use ttm-mcp with Claude Code, Claude Desktop, Cursor, VS Code, Codex and Windsurf, or the remote MCP for Claude.ai and ChatGPT.
order: 6
section: guides
---

There are two ways to let AI apps work with your Notee and Factory:

| | Local MCP (`ttm-mcp`) | Remote MCP |
| --- | --- | --- |
| Where it runs | On your machine; the app starts `ttm-mcp` over stdio | On the server; the app calls `https://toantranmei.com/api/v1/mcp` |
| Install | `ttm`, `ttm-mcp` | Nothing |
| Sign in | The token from `ttm login` (or `TTM_TOKEN`) | OAuth in the app; click **Allow** on the web |
| Good for | Claude Code, Claude Desktop, Cursor, VS Code, Codex, Windsurf, runners | Claude.ai, ChatGPT, apps that support MCP by URL |
| Tools | The same (same source code) | The same |

The AI always acts **on your behalf**, within the token's scopes and your permissions on each page.

## Local MCP

### Prerequisites

```sh
ttm login          # ttm-mcp reuses this token
ttm doctor         # the ttm-mcp line must be [ok  ]
```

`ttm-mcp` reads the token the same way `ttm` does: `TTM_TOKEN`, then the keyring, then `credentials.toml`, for the
selected profile and host. If the token has expired and there is a refresh token, it renews it automatically. If you are
not signed in, the server still starts but every tool returns `unauthorized` (log:
`` not signed in to <host>; run `ttm login` (tools will answer unauthorized) ``).

### Automatic install

```sh
ttm mcp install --client claude-code
ttm mcp install --client claude-desktop
ttm mcp install --client cursor
ttm mcp install --client codex
```

| Client | What `ttm mcp install` does |
| --- | --- |
| `claude-code` | Runs `claude mcp add --scope user <name> -- <ttm-mcp> [args]`. If the name already exists, removes it (`claude mcp remove --scope user <name>`) and adds it again. Requires `claude` on `PATH`. |
| `claude-desktop` | Writes `mcpServers.<name>` to `claude_desktop_config.json` |
| `cursor` | Writes `mcpServers.<name>` to `~/.cursor/mcp.json` |
| `codex` | Writes `[mcp_servers.<name>]` to `$CODEX_HOME/config.toml` (default `~/.codex/config.toml`) |

For config files:

- **Merge, never overwrite**: other MCP servers, other keys, key order, the existing `env` of the `ttm` entry and (TOML)
  comments are kept; only `command` and `args` of the `ttm` entry are reset.
- **Backup**: the old file is copied to `<file>.bak-<unix seconds>` before it is changed.
- **Broken file** (invalid JSON/TOML): nothing is written and an error is reported; fix the file and run again.
- Running again when nothing has changed prints ``<client> already uses <command> for `<name>`.`` and creates no backup.

| Flag | Default | Purpose |
| --- | --- | --- |
| `--client <name>` | (required) | `claude-code`, `claude-desktop`, `cursor`, `codex` |
| `--name <name>` | `ttm` | Server name in the client config |
| `--command <path>` | `ttm-mcp` next to `ttm`, then on `PATH`, then the literal `ttm-mcp` | The command the client runs. An absolute path, because GUI apps do not see your shell's `PATH`. |
| `--toolsets <list>` | (not passed) | Adds `--toolsets <list>` to `args` |
| `--dry-run` | | Prints what would be written (or the `claude` command that would run); writes nothing |
| `--host`, `--profile` (global flags) | | Pinned into `args` only when you pass them explicitly (or set `TTM_HOST`) |

```sh
ttm mcp install --client cursor --dry-run
```

```text
--- /Users/an/.cursor/mcp.json (dry run)
{
  "mcpServers": {
    "ttm": {
      "command": "/Users/an/.cargo/bin/ttm-mcp",
      "args": []
    }
  }
}

Dry run: nothing written.
```

```sh
ttm --profile dev --host http://localhost:3000 mcp install --client claude-code --name ttm-dev
```

```text
Claude Code now starts `/Users/an/.cargo/bin/ttm-mcp` as MCP server `ttm-dev`.
Restart the client to load it. Sign in first with `ttm login` if you have not.
```

Restart the app after installing.

### Claude Code

Manual install (user scope, all projects):

```sh
claude mcp add --scope user ttm -- ttm-mcp
claude mcp add --scope user ttm-notee -- ttm-mcp --toolsets notee
```

For a single project, put a `.mcp.json` file at the repo root:

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": ["--toolsets", "notee,factory"] }
  }
}
```

To check: type `/mcp` in Claude Code, or run `claude mcp list`.

### Claude Desktop

Config file:

| Operating system | Path |
| --- | --- |
| macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| Windows | `%APPDATA%\Claude\claude_desktop_config.json` |
| Linux | `$XDG_CONFIG_HOME/Claude/claude_desktop_config.json` (default `~/.config/Claude/…`) |

```json
{
  "mcpServers": {
    "ttm": {
      "command": "/Users/an/.cargo/bin/ttm-mcp",
      "args": []
    }
  }
}
```

On Windows, use paths with `\\`: `"C:\\Users\\an\\.cargo\\bin\\ttm-mcp.exe"`. Quit Claude Desktop completely, then open
it again.

#### `.mcpb` bundle (one-click install)

Since 0.1.4, every release on [GitHub Releases](https://github.com/toantranmei/ttm/releases) has a Claude Desktop bundle
per operating system, with a `.sha256` file:

| Bundle | Machine |
| --- | --- |
| `ttm-mcp-darwin.mcpb` | macOS (Apple Silicon and Intel, universal binary) |
| `ttm-mcp-win32-x64.mcpb` | Windows x64 (Windows on ARM runs it through emulation) |
| `ttm-mcp-linux-x64.mcpb`, `ttm-mcp-linux-arm64.mcpb` | Linux (static musl binary) |

Open the file with Claude Desktop (double-click it, or **Settings → Extensions → Install Extension…**). The bundle runs
`ttm-mcp --toolsets notee` (the `workspace` and `notee` toolsets, no `factory`) and asks for two values: **Access token**
(a `ttm_pat_…` PAT created at `https://toantranmei.com/me/ket-noi`; leave it empty to use the token saved by
`ttm login`) and **Server** (default `https://toantranmei.com`). No separate `ttm-mcp` install is needed; update by
installing the bundle of the new release.

### Cursor

`ttm mcp install --client cursor` writes `~/.cursor/mcp.json` (all projects). For a single project: `.cursor/mcp.json` in
the repo, same format:

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": [] }
  }
}
```

### VS Code (GitHub Copilot, agent mode)

There is no `--client vscode` yet; add it by hand to the workspace's `.vscode/mcp.json` (or with the
**MCP: Open User Configuration** command):

```json
{
  "servers": {
    "ttm": {
      "type": "stdio",
      "command": "ttm-mcp",
      "args": ["--toolsets", "notee"]
    }
  }
}
```

### Codex

`~/.codex/config.toml` (or `$CODEX_HOME/config.toml`):

```toml
[mcp_servers.ttm]
command = "/Users/an/.cargo/bin/ttm-mcp"
args = ["--toolsets", "notee"]
```

### Windsurf and other apps

Windsurf: `~/.codeium/windsurf/mcp_config.json`. Most apps use the same `mcpServers` block:

```json
{
  "mcpServers": {
    "ttm": {
      "command": "ttm-mcp",
      "args": [],
      "env": { "TTM_PROFILE": "work" }
    }
  }
}
```

Pass configuration through `env` when needed: `TTM_PROFILE`, `TTM_HOST`, `TTM_TOOLSETS`, or `TTM_TOKEN` (machines without
a keyring). To run through npm with nothing installed: `"command": "npx", "args": ["-y", "@toantranmei/mcp"]`.

### ttm-mcp options

```text
Usage: ttm-mcp [OPTIONS]
```

| Flag | Variable | Purpose |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | Server |
| `--profile <NAME>` | `TTM_PROFILE` | Profile in `config.toml` |
| `--toolsets <LIST>` | `TTM_TOOLSETS` | Comma-separated toolsets to enable: `notee`, `factory`. `workspace` is always on. Unset = all. |
| `-v`, `-vv` | `TTM_LOG` | Log to stderr |
| `-V`, `--version` | | Print the version |

`ttm-mcp` has no `--json` and no subcommands: it only speaks the MCP protocol on stdin/stdout. If discovery is
unreachable, it uses the default endpoint `<host>/api/v1`.

## Toolsets

| Toolset | Tools | Scope required |
| --- | --- | --- |
| `workspace` (always on) | `workspace-get-self`, `workspace-list`, `workspace-get-users` | `workspace:read` |
| `notee` (alias `notes`) | `notee-search`, `notee-fetch`, `notee-query-database`, `notee-get-comments`, `notee-create-pages`, `notee-update-page`, `notee-move-pages`, `notee-duplicate-page`, `notee-create-database`, `notee-update-database`, `notee-upsert-rows`, `notee-delete-rows`, `notee-create-comment`, `notee-get-suggestions`, `notee-resolve-suggestion`; resource `notee://spec/markdown` | `notee:read` / `notee:comment` / `notee:write` |
| `factory` | `factory-get-task`, `factory-get-context`, `factory-report-progress`, `factory-request-approval`, `factory-create-subtasks`, `factory-attach-artifact`, `factory-get-rules`, `factory-renew-lease` | `factory:read` / `factory:run` |

```sh
ttm-mcp --toolsets notee            # workspace + notee
ttm-mcp --toolsets ""               # workspace only
```

Parameters, results and errors for each tool: [MCP tools](mcp-cong-cu.md). Fewer tools help the model pick the right
one and use less context; enable `factory` mainly for agents running in a runner.

The server introduces itself to the client as `ttm`, with the instructions: *Start with workspace-get-self to learn who
the user is and which workspace is current.* The client app's name (for example `claude-code`) is recorded in the page
history when a tool edits a page: “An (via agent claude-code)”.

## Agent run variables

When the runner runs a task, it starts `ttm-mcp` with:

| Variable | Value |
| --- | --- |
| `TTM_TOKEN` | The job token (`ttm_job_…`), usable only for that run |
| `TTM_HOST` | The runner's host |
| `TTM_NO_KEYRING` | `1` |
| `TTM_RUN_ID` | The run id |
| `TTM_TASK` | The task key, for example `AUR-12` |
| `TTM_PROJECT` | The project id (or key) |

This way the `factory-*` tools know "this run's task" without the agent passing a key. Outside a run, the tools that need
a run (`factory-report-progress`, `factory-request-approval`, `factory-attach-artifact`, `factory-renew-lease`) return the
error `no_run`.

## Remote MCP

URL: **`https://toantranmei.com/api/v1/mcp`** (MCP Streamable HTTP, stateless).

### Claude.ai

1. **Settings → Connectors → Add custom connector**.
2. Any name (for example *toantranmei*), URL `https://toantranmei.com/api/v1/mcp`, click **Add**, then **Connect**.
3. The browser opens the toantranmei.com consent page; sign in if needed and click **Allow**.

### ChatGPT

In **Settings → Connectors** (developer mode must be on for custom connectors), create a new connector with the URL
above, choose OAuth authentication and complete the consent step.

### What the consent page shows

- The heading **“<app name> wants to access your account”**, and the line *External app. Only allow it if you trust this
  app.*
- A red warning if the app name **resembles** the name of a toantranmei app but is not the official app.
- Where the browser returns after you approve (*After you allow, the browser returns to <host>.*).
- The list of requested permissions. An app that requests no specific scopes gets every scope except `offline_access`.
- The **Limit to workspace** field when you belong to several workspaces.
- The **Allow** / **Deny** buttons.

Tokens for remote apps last **1 hour**; an app gets a refresh token (90 days, rotated on every use) only when it requests
`offline_access`. Reusing an old refresh token revokes the whole token family. Disconnect at `<host>/me/ket-noi`, in the
**Connected apps** tab.

### Details for client authors

| Item | Value |
| --- | --- |
| No token | `401` with `WWW-Authenticate: Bearer resource_metadata="https://toantranmei.com/.well-known/oauth-protected-resource/api/v1/mcp"` |
| Protected Resource Metadata (RFC 9728) | `/.well-known/oauth-protected-resource` (and the `/api/v1/mcp` suffix) |
| Authorization Server Metadata (RFC 8414) | `/.well-known/oauth-authorization-server` |
| Consent page | `/ket-noi/cli` |
| Token, revocation, device | `/api/v1/oauth/token`, `/api/v1/oauth/revoke`, `/api/v1/oauth/device_authorization` |
| Client registration | Client ID Metadata Document (`client_id` is a URL), or Dynamic Client Registration `/api/v1/oauth/register` |
| PKCE | Required, `S256` |
| `resource` | Must be `https://toantranmei.com/api/v1/mcp`; anything else gives `invalid_target` |

Every tool call on the remote MCP goes through the same token, scope and permission checks as an API request. The remote
MCP enables every toolset; Factory tools that need a run return `no_run` there.

## Limits

| Limit | Value |
| --- | --- |
| Agent Notee writes | 60 per hour per token (creating pages, creating rows as pages, editing pages); beyond that, `429 rate_limited` |
| One write | 1,000 blocks (nested blocks included), 2 MB of Markdown |
| `notee-create-pages` | 1–20 pages per call |
| `notee-upsert-rows` | 1–100 rows per call |
| `notee-search` | At most 20 results; query ≤ 8 words, ≤ 200 characters |
| `notee-query-database` | 1–100 rows per page (default 100), 3 seconds, 4 MB per page, 2 concurrent queries per user |
| Long pages | Cut off after 20,000 nodes (`truncated`) |
| Comment quote | 1–1,000 characters |
| `factory-create-subtasks` | 10 subtasks per run, 30 under one root task, at most 2 levels deep |

## Testing the server with MCP Inspector

```sh
npx @modelcontextprotocol/inspector ttm-mcp --toolsets notee
```

Inspector opens a web UI where you can list tools, read the `notee://spec/markdown` resource and try each tool.
