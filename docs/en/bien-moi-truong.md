---
title: Environment variables
description: Every environment variable that ttm, ttm-mcp, and the runner read, with defaults and precedence.
order: 13
section: reference
---

An empty variable is treated as unset (unless noted otherwise).

## Connection and authentication

| Variable | Read by | Default | Meaning |
| --- | --- | --- | --- |
| `TTM_HOST` | `ttm`, `ttm-mcp` | the profile's host, then `https://toantranmei.com` | The server. Loses to `--host`, wins over the profile. When set, `ttm login` writes this host to the profile and `ttm mcp install` pins `--host` in the client configuration. |
| `TTM_PROFILE` | `ttm`, `ttm-mcp` | `default_profile`, then `default` | Profile in `config.toml`. Loses to `--profile`. |
| `TTM_TOKEN` | `ttm`, `ttm-mcp` | (none) | Token used for every request; wins over the keyring and `credentials.toml`. Never refreshed automatically. The runner does not use it for its own runner token. |
| `TTM_NO_KEYRING` | `ttm`, `ttm-mcp`, runner | (keyring enabled) | Any non-empty value other than `0`: never read or write the keyring; tokens go to `credentials.toml`. |
| `TTM_NO_BROWSER` | `ttm` | (opens the browser) | When set: don't open the browser (`ttm login`, `ttm token …`, `ttm project open`); only print the URL. |

## Directories

| Variable | Default | Meaning |
| --- | --- | --- |
| `TTM_CONFIG_DIR` | `~/.config/ttm` (`%APPDATA%\ttm`) | Directory for `config.toml`, `credentials.toml`, `runner.toml` |
| `TTM_CACHE_DIR` | `~/.cache/ttm` (`%LOCALAPPDATA%\ttm\cache`) | Discovery cache, runner pid and logs, worktree registry |
| `TTM_HOME` | the operating system's user home directory | Overrides the home directory: `~/` expansion, `~/.ttm/worktrees`, client configuration locations for `ttm mcp install`, runner service files. Use it for portable installs and tests (on Windows `HOME` is ignored). |
| `XDG_CONFIG_HOME` | `~/.config` | (macOS, Linux) root of the configuration directory; also the root for Claude Desktop configuration on Linux and the runner's systemd file |
| `XDG_CACHE_HOME` | `~/.cache` | (macOS, Linux) root of the cache directory |
| `APPDATA`, `LOCALAPPDATA` | set by Windows | (Windows) configuration and cache directories; `APPDATA` is also where Claude Desktop configuration is looked up |
| `CODEX_HOME` | `~/.codex` | The Codex configuration directory that `ttm mcp install --client codex` edits |

## Logging

| Variable | Meaning |
| --- | --- |
| `TTM_LOG` | Log filter (`tracing_subscriber::EnvFilter`); overrides `-v`/`-vv`. For example `TTM_LOG=debug`, `TTM_LOG=ttm_api=debug,warn`. |

## ttm-mcp

| Variable | Default | Meaning |
| --- | --- | --- |
| `TTM_TOOLSETS` | all groups | Same as the `--toolsets` flag: comma-separated tool groups to enable (`notee`, `factory`; `workspace` is always on). The flag wins over the variable. |
| `TTM_RUN_ID` | (none) | Agent run id. Required for `factory-report-progress`, `factory-request-approval`, `factory-attach-artifact`, `factory-renew-lease`; `factory-get-context` uses the run's context. |
| `TTM_TASK` | (none) | Default task key (`AUR-12`) for `factory-get-task`, `factory-get-context`, `factory-create-subtasks`. |
| `TTM_PROJECT` | (none) | Default project (id or key) for `factory-get-rules`. |

The runner sets `TTM_TOKEN` (the job's token), `TTM_HOST`, `TTM_NO_KEYRING=1`, `TTM_RUN_ID`, `TTM_TASK`, and `TTM_PROJECT`
for the `ttm-mcp` it starts for the agent. `ttm hook` also reads `TTM_TASK` and `TTM_RUN_ID` to attach rule results to the
right run.

## Runner

| Variable | Meaning |
| --- | --- |
| `HOSTNAME`, `COMPUTERNAME` | The runner's default machine name (when `runner.toml` has no `name`); if unset, the `hostname` command is run |
| `PATH` (`PATHEXT` on Windows) | Where `claude`, `git`, `gh`, `ttm-mcp` are looked up |

## Build time

| Variable | Meaning |
| --- | --- |
| `TTM_DEFAULT_HOST` | Default host embedded in the binary (`cargo build`). Release builds use `https://toantranmei.com`. |

## Network

`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`, `NO_PROXY` are honored by the HTTP client as usual.
