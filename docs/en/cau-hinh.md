---
title: Configuration
description: config.toml and profiles, how the host is chosen, file locations on each operating system, JSON output, logs and networking.
order: 5
section: guides
---

`ttm` works without any configuration: with no file, it uses the `default` profile and the host `https://toantranmei.com`.
The files below are created when you sign in, switch workspaces or configure the runner.

## File locations

| Directory | macOS, Linux | Windows | Override with |
| --- | --- | --- | --- |
| Config | `$XDG_CONFIG_HOME/ttm`, default `~/.config/ttm` | `%APPDATA%\ttm` | `TTM_CONFIG_DIR` |
| Cache | `$XDG_CACHE_HOME/ttm`, default `~/.cache/ttm` | `%LOCALAPPDATA%\ttm\cache` | `TTM_CACHE_DIR` |
| Worktrees | `~/.ttm/worktrees` | `%USERPROFILE%\.ttm\worktrees` | `worktree_root` in `config.toml` or `runner.toml` |

> **Note:** on macOS, ttm also uses `~/.config/ttm` and `~/.cache/ttm` (not `~/Library`).

| File | Where | Contents |
| --- | --- | --- |
| `config.toml` | config directory | Profiles, hosts, current workspace, `worktree_root` |
| `credentials.toml` | config directory | Tokens when the keyring isn't available (mode 0600) |
| `runner.toml` | config directory | Runner configuration (see [Runner](runner.md)) |
| `discovery-<host>.json` | cache | The host's discovery, reused for 1 hour; the old copy is still used when the host doesn't respond |
| `deprecations` | cache | Warnings about soon-to-be-removed endpoints already shown today |
| `runner.pid`, `runner.log`, `runner-worktrees.json` | cache | The runner's process, log (macOS service) and worktree ledger |
| `skills/<sha256>.zip` | cache | Skills downloaded by the runner, reused by SHA-256 |
| `mcp-<run>.json`, `settings-<run>.json`, `hooks/<run>/`, `claude-team/<run>/` | cache | Temporary files of one run (0600), removed when the run ends |
| `.ttm/workflow.yaml`, `.ttm/rules/`, `.ttm/agents/`, `.ttm/skills/` | in the repo | The project's as-code configuration (see [As-code](as-code.md)) |
| `.ttm/project.toml` | in the repo | Links the repo to a Factory project (see [Factory](factory.md)) |

`ttm status` prints the path of the `config.toml` in use.

## config.toml

```toml
# Profile used when there is no --profile or TTM_PROFILE.
default_profile = "work"

# Worktree directory for `ttm sync --resolve` and the runner (default ~/.ttm/worktrees). `~/` is expanded.
worktree_root = "~/orca/workspaces"

[profiles.default]
host = "https://toantranmei.com"

[profiles.work]
host = "https://toantranmei.com"
workspace = "0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71"

[profiles.dev]
host = "http://localhost:3000"
```

| Key | Type | Written by | Meaning |
| --- | --- | --- | --- |
| `default_profile` | string | you | Default profile; `default` if absent |
| `worktree_root` | string | you | Root of the worktree directory |
| `profiles.<name>.host` | URL | `ttm login` | The profile's server |
| `profiles.<name>.workspace` | id | `ttm workspace switch` | The profile's current workspace |

`ttm` writes this file atomically (writes a temp file, then renames it), so comments you add are lost on the next write.
A file with a syntax error makes every command (except `ttm completion`) stop with `invalid TOML in <path>: …`.

## Profiles

A profile is a named (host, workspace) pair. Tokens are stored per (profile, host) pair.

Order for choosing the **profile**:

1. `--profile <name>`
2. `TTM_PROFILE`
3. `default_profile` in `config.toml`
4. `default`

Order for choosing the **host**:

1. `--host <URL>`
2. `TTM_HOST`
3. The profile's `host`
4. `https://toantranmei.com` (a build-time value, changeable with `TTM_DEFAULT_HOST` at build time)

The host is normalized: the trailing `/` is dropped, the domain is lowercased, and `https://` is added if missing
(`http://` for `localhost` and `127.0.0.1`). Only `http` and `https` are accepted; anything else reports `invalid host`.

```sh
ttm --host toantranmei.com status        # → https://toantranmei.com
ttm --host localhost:3000 status         # → http://localhost:3000
```

Order for choosing the **workspace** for commands that need one (`notes search`, `notes create --workspace`,
`project init`, MCP tools): the command's argument, then the profile's workspace. When no workspace is chosen, `ttm-mcp`
uses your personal workspace.

See everything your machine is using:

```sh
ttm status
```

```text
ttm 0.2.0
host:      https://toantranmei.com
profile:   work (/Users/an/.config/ttm/config.toml)
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
workspace: 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
server:    API v1 (min ttm 0.1.0), features: notee, factory
```

`ttm status` doesn't fail when you're not signed in or offline (`server: unreachable`).

## Environment variables

The most commonly used variables:

| Variable | What it does |
| --- | --- |
| `TTM_HOST` | Server (after `--host`) |
| `TTM_PROFILE` | Profile (after `--profile`) |
| `TTM_TOKEN` | Token, wins over the keyring and the file |
| `TTM_NO_KEYRING=1` | Don't use the keyring, only `credentials.toml` |
| `TTM_LOG` | Log level (`tracing` syntax), for example `debug` |
| `TTM_PLAIN=1` | Plain text output (like `--plain`) |
| `TTM_COLUMNS` | Output width (40–200) |
| `NO_COLOR` | No colours |

Full list (including `ttm-mcp` and runner variables): [Environment variables](bien-moi-truong.md).

## Output for people

Without `--json`, `ttm` prints for people in one pattern: the result on the first line, then details, then `Next:
<suggested command>`. Colours turn off when piped, with a non-empty `NO_COLOR` or `TERM=dumb`; `--plain` (or
`TTM_PLAIN=1`) also drops frames, glyphs and emoji, for logs. Width: `TTM_COLUMNS` > `COLUMNS` > terminal size > 80,
clamped to 40–200. Tables never wrap; long cells are cut with `…`. Details, glyphs and badges: [Command line
interface](giao-dien.md).

Errors go to **stderr** in this pattern:

```text
✕ <summary>
  rule: <rule>  field: <field>
  Fix: <how to fix it>
  Help: https://toantranmei.com/help/ttm/ma-loi#<rule>
```

Exit codes: `0` success, `1` error, `2` usage (unknown flag, a missing flag `missing_flag`, `--yes` needed
`confirmation_required`, a feature the server lacks `unsupported_feature`, a bad flag value `invalid_flag`).

## JSON output

Every command accepts `--json` (anywhere on the command line): the result is pretty-printed JSON on **stdout**, and
secondary messages still go to stderr.

```sh
ttm whoami --json
```

```json
{
  "host": "https://toantranmei.com",
  "profile": "default",
  "user": {
    "id": "0190f3a1-0000-7000-8000-000000000001",
    "email": "an@example.com",
    "name": "An Nguyễn",
    "avatar_url": null,
    "locale": "vi",
    "plan": "free"
  },
  "workspace": null
}
```

On error, `--json` also prints an error object to stdout (and the `✕ …` error block to stderr), with a non-zero exit code:

```json
{
  "error": {
    "code": "insufficient_scope",
    "message": "this token lacks the scope notee:write (insufficient_scope, HTTP 403) [requires scope notee:write]",
    "status": 403,
    "params": { "required": "notee:write" }
  }
}
```

An error from the server also has `status` and, when the server sends them, `rule` (for example `split-limit`,
`rev-conflict`), `field` and `params`.
`code` is the server's stable code (see [Error codes](ma-loi.md)), an OAuth code (`access_denied`, `invalid_grant`…), or
a local code: `network` (couldn't connect), `decode` (unexpected response), `upgrade_required` (ttm is too old),
`invalid_url`, `error` (anything else, such as not signed in or a file that can't be read), and the usage codes
`missing_flag`, `confirmation_required`, `unsupported_feature`, `invalid_flag` (with `rule` and `field`). JSON shapes stay
as in 0.1.6; 0.2.0 only adds keys (for example `meta.membership` of `whoami`/`login`).

Examples for scripts:

```sh
ttm notes search "OKR" --json | jq -r '.[].id'
ttm notes query "$DB" --sql 'SELECT "Name" FROM rows' --json | jq '.rows[].values'
```

## runner.toml: local MCP

Since 0.2.0, `runner.toml` has a local MCP section, edited with `ttm runner config mcp …` (comments are kept):

```toml
[mcp.local.github-local]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }   # "env:VAR" = read the runner's environment at run time
allow = ["dev", "reviewer"]                    # agent handles allowed to use it
team_jobs = false                              # also for jobs delegated by someone else
```

Names follow `^[a-z0-9][a-z0-9-]{0,31}$` and are never `ttm`. The server is only told names, `allow` and `team_jobs`.
See [Runner](runner.md#local-mcp-ttm-runner-config-mcp).

## Logs

Logs go to **stderr** (`ttm-mcp`'s stdout is reserved for the MCP protocol).

| Flag | Level |
| --- | --- |
| (none) | `warn`: warnings, for example soon-to-be-removed endpoints, the server needs a newer version |
| `-v` | `info` (and `debug` for ttm's modules) |
| `-vv` | `debug` |
| `TTM_LOG=<filter>` | Override, `tracing_subscriber::EnvFilter` syntax, for example `TTM_LOG=ttm_api=debug,warn` |

Tokens never appear in logs: everywhere a token is displayed, only the prefix and at most 4 characters are kept.

## Networking

- Each request has a **30-second** timeout and sends `User-Agent: ttm/<version> (<os>; <arch>)`.
- Connection errors, timeouts and 5xx errors on idempotent requests (GET, PUT, DELETE) are retried up to **3 times**
  (waiting 0.5 s, 1 s, 2 s).
- `429` is retried up to 3 times for every kind of request, waiting per `Retry-After` (at most 60 seconds each time).
- An endpoint with a `Deprecation` header produces one warning per day, with the removal date (`Sunset`) if present.
- Proxy: follows the standard `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY` variables.
