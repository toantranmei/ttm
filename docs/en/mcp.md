---
title: MCP
description: Use ttm-mcp with Claude Code, Claude Desktop, Cursor and Codex, or the remote MCP with nothing to install.
order: 4
---

# MCP

`ttm-mcp` is an MCP server running on your machine (stdio) with the token of `ttm login` (or `TTM_TOKEN`). The AI acts
on your behalf, within the scopes of the token.

## Quick setup

```sh
ttm login
ttm mcp install --client claude-code      # runs: claude mcp add --scope user ttm -- ttm-mcp
ttm mcp install --client claude-desktop   # edits claude_desktop_config.json
ttm mcp install --client cursor           # edits ~/.cursor/mcp.json
ttm mcp install --client codex            # edits ~/.codex/config.toml
```

Config files are **backed up** (`<file>.bak-<timestamp>`) before any change and **merged**: other MCP servers and
settings stay as they are. A broken file is left untouched. Preview with `--dry-run`.

## Manual configuration

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": [] }
  }
}
```

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.ttm]
command = "ttm-mcp"
args = []
```

## ttm-mcp options

| Flag | What it does |
| --- | --- |
| `--host <URL>` | Server (same default as `ttm`) |
| `--profile <name>` | Profile in `config.toml` |
| `--toolsets notee,factory` | Extra tool groups (`workspace` is always on) |
| `-v`, `-vv` | Logs on stderr |

## Tools

| Tool | What it does |
| --- | --- |
| `workspace-get-self` | The user and the current workspace |
| `workspace-list` | Workspaces |
| `workspace-get-users` | Workspace members (filter by name/e-mail) |

Notee and Factory tools arrive in later releases.

## Claude Desktop without a terminal

Download the `.mcpb` file for your OS from the Releases page, open it with Claude Desktop and paste a token created at
`/me/ket-noi`.

## Remote MCP

Nothing to install: add the remote MCP URL (shown on this page on the web) to Claude.ai or ChatGPT, sign in and allow.
