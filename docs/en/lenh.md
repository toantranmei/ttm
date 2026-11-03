---
title: Commands
description: ttm commands, global flags and JSON output.
order: 3
---

# Commands

Flags for every command: `--host <URL>`, `--profile <name>`, `--json` (JSON output for scripts and agents), `-v` /
`-vv` (detailed logs on stderr).

| Command | What it does |
| --- | --- |
| `ttm login [--device] [--token <PAT>]` | Sign in |
| `ttm logout` | Revoke the token and remove it from this machine |
| `ttm whoami` | The signed-in user |
| `ttm status` | Server, profile, token and workspace in use |
| `ttm doctor` | Check server, discovery, token, version, keyring, git, claude |
| `ttm workspace list` | Your workspaces (`*` marks the current one) |
| `ttm workspace switch <id|slug|name>` | Choose the workspace for the profile |
| `ttm workspace current` | The current workspace |
| `ttm token list` / `create` / `revoke [id]` | Open `/me/ket-noi` (see below) |
| `ttm notes search|get|query|comments` | Search, read pages (Markdown), query databases (SQL or JSON filter), read comments |
| `ttm notes create|edit|comment|rows` | Create a page from a file, edit (`--old/--new`, `--replace`, `--suggest`), quoted comments, database rows |
| `ttm notes suggestions|resolve` | List and accept/reject suggested edits |
| `ttm project init|link|open|status` | Link the repository to a Factory project |
| `ttm sync [--check] [--resolve]` | Sync the plan in `.ttm/plan`, send change requests |
| `ttm task list|show|claim|done|approve|split` | Project tasks |
| `ttm rules apply` | Write the rules into AGENTS.md/CLAUDE.md and the agent hooks |
| `ttm runner up|down|status|config|install-service` | Run agent jobs from the server on this machine |
| `ttm agent run|logs|stop` | Run a task now, follow its log, stop it |
| `ttm mcp install --client <client>` | Configure an MCP client to use `ttm-mcp` |
| `ttm self-update [--check]` | Update ttm |
| `ttm completion <bash|zsh|fish|powershell|elvish>` | Shell completion script |

## Tokens

The token management API only accepts the web session, not a token (a token cannot mint other tokens). So
`ttm token create|list|revoke` open `<server>/me/ket-noi` in the browser; a new token is shown there once.

```sh
ttm token create --scope notee:read,workspace:read --expires 30d --name "CI"
```

## Completion

```sh
ttm completion zsh > "${fpath[1]}/_ttm"
ttm completion bash > ~/.local/share/bash-completion/completions/ttm
ttm completion fish > ~/.config/fish/completions/ttm.fish
```

## JSON

With `--json` the result is JSON on stdout; errors look like `{"error": {"code": "...", "message": "..."}}` with a
non-zero exit code.
