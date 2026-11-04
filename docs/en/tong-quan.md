---
title: Overview
description: What ttm and ttm-mcp are, the key concepts, and how the pieces fit together.
order: 1
section: start
---

**ttm** is an open-source (MIT) toolkit from [toantranmei.com](https://toantranmei.com) made up of two programs released
under the same version:

| Program | What it is | Use it to |
| --- | --- | --- |
| `ttm` | Command-line interface (CLI) | Sign in, switch workspaces, read and edit Notee pages, link a repo to a Factory project, sync the plan, enforce rules for agents, run agents on your machine, install MCP into AI apps |
| `ttm-mcp` | An [MCP](https://modelcontextprotocol.io) server that runs on your machine over stdio | Let Claude Code, Claude Desktop, Cursor, Codex, VS Code… read and write Notee and work on Factory tasks on your behalf |

Source code: <https://github.com/toantranmei/ttm>. Current version: **0.1.6**.

If you don't use a terminal, you don't need to install anything: paste the **remote MCP URL**
`https://toantranmei.com/api/v1/mcp` into Claude.ai or ChatGPT (see [Connecting MCP](mcp.md)).

## Concepts

| Concept | Meaning |
| --- | --- |
| **Host** | The server `ttm` talks to. Defaults to `https://toantranmei.com`; change it with `--host`, `TTM_HOST`, or a profile (for example `http://localhost:3000` during development). |
| **Discovery** | The `GET <host>/.well-known/ttm` document that tells ttm the API, OAuth, and MCP addresses, the API version, `min_cli`, and features (`notee`, `factory`). ttm hard-codes no URL other than the host. The cached copy lives for 1 hour. |
| **Profile** | A named (host, current workspace) pair in `config.toml`, for example `default`, `work`. Each profile has its own token. |
| **Workspace** | A workspace (`personal` or `team`) that holds Notee pages and Factory projects. Your role is `owner`, `admin`, `member`, or `guest`. The current workspace is stored per profile. |
| **Token** | A secret string starting with `ttm_`: `ttm_pat_` (a personal access token created on the web), `ttm_oat_` (an OAuth access token, received by `ttm login` or a remote MCP app), `ttm_ort_` (refresh token), `ttm_job_` (the token for the runner and for each agent run). |
| **Scope** | A token's permissions: `workspace:read`, `notee:read`, `notee:write`, `notee:comment`, `factory:read`, `factory:write`, `factory:run`, `offline_access`. Effective access = the intersection of the scopes and your permissions on each resource. See [Signing in and tokens](dang-nhap.md). |
| **Notee** | The notes app: pages, databases, comments, suggested edits. Agents read and write pages as [Notee-flavored Markdown](notee-markdown.md). |
| **Factory** | The project management app for people and agents: projects, tasks, workflows, approval gates, change requests, merge queue. The web is the source of truth; the repo keeps a copy of the plan in `.ttm/plan`. |
| **Runner** | `ttm runner up`: a process on your dev machine that receives *jobs* from the server, creates git worktrees, runs Claude Code with your Claude account, sends events to the Execution Map, pushes branches, and opens pull requests. |
| **Harness** | An adapter for running a coding agent. Currently `claude-code`. |
| **Local MCP and remote MCP** | Local MCP is `ttm-mcp` on your machine (stdio, using the token from `ttm login`). Remote MCP is the `https://toantranmei.com/api/v1/mcp` endpoint (Streamable HTTP, OAuth 2.1), with the same set of tools. |

## Architecture

```text
                      ┌──────────────── toantranmei.com (server) ─────────────────┐
 Claude.ai, ChatGPT ─▶│ /api/v1/mcp        remote MCP (Streamable HTTP, OAuth 2.1) │
                      │ /.well-known/ttm   discovery                               │
 ttm, ttm-mcp ───────▶│ /api/v1            API (Bearer token, scope, ACL)          │
   (Bearer)           │ /ket-noi/cli       sign-in consent page                    │
                      │ /activate          device code entry                       │
                      │ /me/ket-noi        manage tokens, apps, runners            │
                      └────────────────────────────────────────────────────────────┘
                              ▲ long-poll + heartbeat (outbound connections only)
                              │
 dev machine: ttm runner ── git worktree per task ── claude -p (your account)
                                                      │ stdio
                                                      ▼
                                                 ttm-mcp (the run's token)
```

- **The "brain" lives on the server**: splitting tasks, choosing models, assembling context and prompts. `ttm` only
  authenticates, creates worktrees, runs the harness for each job, sends events, and enforces rules locally.
- **Agents run only on your machine**, with your Claude plan or API key. The server never holds a Claude key.
- **The same tool set** for local and remote MCP: the server reuses the `ttm-mcp-tools` crate, so tool names, parameters,
  and results are identical.

## What's in the repo

| Crate | Role |
| --- | --- |
| `ttm-api` | HTTP client for `/api/v1`: retries, User-Agent, `Deprecation` warnings, 426 errors, problem+json |
| `ttm-core` | Configuration, profiles, host selection, discovery, token storage, project files, worktrees |
| `ttm-mcp-tools` | MCP tools on the `Backend` trait (shared with remote MCP) |
| `ttm-runner` | Runner: receiving jobs, worktrees, the Claude Code harness, event stream, merge queue |
| `ttm-cli` | The `ttm` binary |
| `ttm-mcp` | The `ttm-mcp` binary |

## New in 0.2

- **One output style**: brand colours, tables, status glyphs, `Next:` lines; `--plain` for logs, `--json` for scripts
  ([Command line interface](giao-dien.md)).
- **Wizards with flags**: `ttm project init`, `ttm agent create`, `ttm mcp-server add`… ask in a terminal, `--yes` for CI.
- **Factory v2 in the CLI**: workflows, Automation, agents, skills, MCP servers, change requests, inbox, templates
  ([Factory](factory.md)); configuration as files in `.ttm/` ([Configuration as code](as-code.md)).
- **MCP v2**: `outputSchema`, one Markdown shape, admin tools with two-step confirmations ([MCP tools](mcp-cong-cu.md)).
- **Runner**: commits carry the agent's identity (never Claude/Anthropic lines), reviewed skills, MCP through the gateway
  and local MCP ([Runner](runner.md)).

## Next steps

- [Installation](cai-dat.md), then [Quickstart](bat-dau-nhanh.md) (5 minutes).
- [Connecting MCP](mcp.md) to use it with Claude, Cursor, Codex.
- [Command reference](lenh.md) and [MCP tools](mcp-cong-cu.md) when you need to look something up.
