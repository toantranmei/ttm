# ttm

Command line and MCP server for [toantranmei.com](https://toantranmei.com): sign in, switch workspaces, and give AI
agents (Claude Code, Claude Desktop, Cursor, Codex, …) access to your Notee pages and Factory projects.

Two binaries ship from this repository, with the same version:

| Binary | What it is |
| --- | --- |
| `ttm` | The CLI: sign in, workspaces, tokens, diagnostics, MCP client setup, Notee, Factory plan sync and rules, and the local agent runner |
| `ttm-mcp` | An [MCP](https://modelcontextprotocol.io) server over stdio that exposes your workspace to AI agents |

Full documentation (Vietnamese, English, Chinese): <https://toantranmei.com/help/ttm>. The sources live in [`docs/`](docs).

## Install

| Channel | Command |
| --- | --- |
| Shell (macOS, Linux) | `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh \| sh` |
| PowerShell (Windows) | `powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.ps1 \| iex"` |
| Homebrew | `brew install toantranmei/tap/ttm` |
| npm (soon) | `npm install -g @toantranmei/cli` (and `npx @toantranmei/mcp` for the MCP server) |
| Cargo | `cargo install ttm-cli ttm-mcp` |

The MCP server has its own installers (`ttm-mcp-installer.sh`, `ttm-mcp-installer.ps1`, `brew install
toantranmei/tap/ttm-mcp`). Binaries are not code-signed; every release carries SHA-256 checksums and a GitHub artifact
attestation (`gh attestation verify <file> --repo toantranmei/ttm`). Upgrade with `ttm self-update`.

## Sign in

```sh
ttm login              # opens the browser, comes back to a local loopback page (PKCE)
ttm login --device     # no browser on this machine (SSH, containers): enter the code at /activate
ttm login --token ttm_pat_…   # or pipe a personal access token: echo $PAT | ttm login --token -
ttm whoami
ttm doctor             # host, discovery, token, keyring, git, claude
```

Tokens live in the OS keyring (service `ttm`), or in `~/.config/ttm/credentials.toml` (mode 0600) when no keyring is
available. `TTM_TOKEN` overrides both, which is handy in CI.

Pick the server with `--host`, `TTM_HOST` or a profile (`ttm --profile work login --host https://example.com`); the
default is `https://toantranmei.com`.

## Examples

```sh
ttm workspace list
ttm workspace switch "Team Aurora"
ttm status --json
ttm token create        # opens <host>/me/ket-noi in the browser
ttm completion zsh > ~/.zfunc/_ttm

ttm notes search "kế hoạch"
ttm notes edit <page> --old "Q3" --new "Q4" --suggest
ttm project link AUR && ttm sync && ttm rules apply
ttm runner config --repo AUR=~/code/aurora && ttm runner up
```

Every command accepts `--json`.

## Use with AI clients (MCP)

```sh
ttm mcp install --client claude-code      # runs: claude mcp add ttm -- ttm-mcp
ttm mcp install --client claude-desktop   # edits claude_desktop_config.json (backed up first)
ttm mcp install --client cursor           # edits ~/.cursor/mcp.json
ttm mcp install --client codex            # edits ~/.codex/config.toml
```

Or configure by hand:

```json
{ "mcpServers": { "ttm": { "command": "ttm-mcp", "args": [] } } }
```

`ttm-mcp` reuses the token saved by `ttm login` (or `TTM_TOKEN`). Options: `--host`, `--profile`, `--toolsets
workspace,notee,factory`. No terminal? Add the remote MCP URL shown in the help center to Claude.ai or ChatGPT instead.

## Repository layout

| Crate | Role |
| --- | --- |
| `ttm-api` | HTTP client for `/api/v1`: retries, User-Agent, `Deprecation` warnings, 426 upgrade errors, problem+json |
| `ttm-core` | Config, profiles, host selection, discovery cache, credential storage |
| `ttm-mcp-tools` | MCP tools over a `Backend` trait (shared with the hosted MCP server) |
| `ttm-runner` | Local agent runner: job polling, worktrees, harness adapters (Claude Code first), event stream |
| `ttm-cli` | The `ttm` binary |
| `ttm-mcp` | The `ttm-mcp` binary (stdio) |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Licensed under [MIT](LICENSE).
