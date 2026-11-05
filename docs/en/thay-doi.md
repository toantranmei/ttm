---
title: Changelog
description: Release history of ttm and ttm-mcp.
order: 18
section: more
---

`ttm` and `ttm-mcp` are released under the same version. Downloads: <https://github.com/toantranmei/ttm/releases>.

## 0.2.0 (2026-11-05)

### Breaking changes

- **MCP:** list tools now return `structuredContent` as an object `{items: […], next_cursor?}` instead of an array (MCP
  requires an object when a tool has an `outputSchema`). Tools that already returned an object keep their keys.
- **Errors on stderr:** `error: <message>` became `✕ <summary>` / `rule:` / `Fix:` / `Help:` ([Error codes](ma-loi.md)).
  `--json` is unchanged (keys only added). Exit code 2 now also covers `missing_flag`, `confirmation_required`,
  `invalid_flag`, `unsupported_feature`.

### ttm

- One output style through `ttm-ui`: brand colours (truecolor → 256 → 16), tables cut with `…`, status glyphs, badges,
  `Next:`; global flags `--plain` (`TTM_PLAIN`) and `-y/--yes`; `TTM_COLUMNS` ([Command line interface](giao-dien.md)).
- `ttm project init` became a wizard (every question has a flag); new commands `workflow`, `automation`, `agent
  create|edit|pause|resume|archive|skills|mcp`, `skill`, `mcp-server`, `cr`, `inbox [--watch]`, `template`, `workspace
  members`.
- `ttm sync` syncs `.ttm/workflow.yaml`, `rules/`, `agents/`, `skills/` as change requests by kind, checks YAML, blocks
  secrets, checks skills ([Configuration as code](as-code.md)).
- Mei-er badges in `whoami`/`login` (+ `meta.membership` in `--json`), `workspace members`, `task show`, `agent list`.
- `ttm login` also asks for the `factory:admin` scope.
- `ttm workflow publish|upgrade` list the Automation rules left on removed, renamed or recategorised states and ask
  before acknowledging them; with `--json` they need `--acknowledge-rules`.

### ttm-mcp

- Every tool has an `outputSchema` and one Markdown shape; membership badges.
- New read tools (workflows, Automation, agents, skills, templates, inbox) and an `admin` toolset with two-step
  confirmations (`factory-confirm`, `factory-confirm-cancel`; elicitation over stdio). Agents running a job never see
  `admin` ([MCP tools](mcp-cong-cu.md)).

### ttm runner

- The agent's git identity, a `commit-msg` hook that removes every Claude/Anthropic line in every job; reviewed skills
  (SHA-256 checked); remote MCP through the gateway and local MCP `[mcp.local.*]`; isolation of teammates' jobs;
  `features` announced to the server ([Runner](runner.md)).
- The token budget counts cache tokens by price (cache writes ×1.25, cache reads ×0.1); runs report
  `cache_creation_tokens` and `cache_read_tokens`.
- The runner pushes and opens the pull request itself, titled `<KEY>: <task title>`; an open pull request on the
  branch is reused.

## 0.1.6 (2026-11-04)

### ttm runner

- The runner starts Claude Code with `--strict-mcp-config`: a job only sees the ttm MCP servers in its own config, never
  other MCP servers configured on the machine, even when a teammate delegated the job.

## 0.1.5 (2026-11-04)

The 0.1.3 and 0.1.4 tags exist but were never released (their tagged commits disabled CI, so the release workflow did not run); 0.1.5 is the first release with the changes below.

### Errors and error codes

- Server errors keep `rule`, `field`, `params`: `ttm` prints the `rule`/`field` in error messages; `--json` and MCP tool
  errors (`structuredContent.error`) also carry `status`, `rule`, `field`, `params`, so you can branch on `split-limit`,
  `rev-conflict`, `approval-required`, or read `params.required` of `insufficient_scope`. See [Error codes](ma-loi.md).
- The `factory-create-subtasks` tool description names the real rule `split-limit` (`409 conflict`) instead of
  `split_limit`.

### ttm

- `ttm task split` accepts camelCase or snake_case keys (`acceptance_criteria`, `depends_on`, like the MCP tool) and
  warns about unknown keys.
- `ttm task approve --reject` prints `rejected` (it used to print `rejectd`).
- `ttm hook pre-bash` protects the branches in `hooks.pre_bash.protected_branches` of the cached rules, then the default
  branch in `.ttm/project.toml`, then `main`/`master` (it used to always be `main`/`master`).
- `ttm notes query --page-size` only accepts 1–100, with a clear error before calling the server.
- CLI sign-in: 7-day access token, renewed automatically; you stay signed in as long as you use `ttm` at least every 90
  days (comes with a server change shipping soon).

### Runner

- The service (launchd, systemd) carries the `PATH` of the shell that ran `install-service` plus `~/.local/bin`,
  `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin`…, so it finds `claude`, `gh`, `git`. New command
  `ttm runner uninstall-service`.
- A job's `worktree_hint` may only be a relative path under `worktree_root`; an absolute path, `..` or a symlink leading
  out is refused (`invalid_worktree`).
- Job long-poll with `wait=25` (the server's maximum); a pending request is no longer cancelled when a heartbeat is due.

### Release

- Claude Desktop `.mcpb` bundles for macOS (universal), Linux x64/arm64 and Windows x64 on GitHub Releases. The manifest
  version follows `Cargo.toml` (checked in CI).

## 0.1.2 (04/11/2026)

First release.

### Distribution channels

- Shell install script (macOS, Linux) and PowerShell (Windows), archives for 7 targets: macOS arm64/x64, Linux arm64/x64
  (glibc and musl), Windows x64; MSI for Windows x64.
- Homebrew: `brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp`.
- npm: `@toantranmei/cli` (the `ttm` command), `@toantranmei/mcp` (the `ttm-mcp` command), published with trusted
  publishing.
- SHA-256 for every file and GitHub artifact attestations.
- No Windows on ARM build (the x64 build runs under emulation).

### ttm

- Sign in: browser (loopback + PKCE), `--device` (RFC 8628), `--token` / stdin; `logout` revokes on the server;
  `whoami`, `status`, `doctor`.
- Profiles and `--host` / `TTM_HOST`, cached `/.well-known/ttm` discovery, tokens in the keyring or a 0600
  `credentials.toml`, `TTM_TOKEN` for CI.
- `workspace list/switch/current`; `token list/create/revoke` (opens the web page).
- Notee: `notes search/get/query/comments/create/edit/comment/rows upsert/rows delete/suggestions/resolve`, suggested
  edits with `--suggest`.
- Factory: `project init/link/open/status`, `sync` with change requests, `--check` for CI, `--resolve` to resolve
  conflicts with an agent in a separate worktree; `task list/show/claim/done/approve/split`; `rules apply` (managed block
  in `AGENTS.md`/`CLAUDE.md`, Claude Code hooks); `hook pre-bash/stop`.
- Runner: `runner up/down/status/config/install-service` (launchd, systemd `--user`, Task Scheduler), Claude Code
  harness, a worktree per task, live events, push and PR via `gh`, optional merge queue; runs jobs only for declared repos
  (`repo_not_configured`, `invalid_branch`), renews the runner token every 12 hours, stops when the token keeps being
  rejected.
- `agent run/logs/stop`.
- `mcp install --client claude-code|claude-desktop|cursor|codex` (merges configuration, backs up, `--dry-run`).
- `self-update` (verifies SHA-256 and attestation), `completion`.
- `TTM_HOME` overrides the home directory (Windows ignores `HOME`).

### ttm-mcp

- MCP server over stdio, 26 tools: `workspace` group (3), `notee` (15), `factory` (8); resource `notee://spec/markdown`;
  `--toolsets`.
- Same tool set as the remote MCP at `https://toantranmei.com/api/v1/mcp`.

## 0.1.1 (cancelled)

Tagged but not released: dist's Linux build machine could not build the MSI for Windows on ARM. Version 0.1.2 drops that
target.

## 0.1.0 (not released)

Tagged before the MSI definition existed, so the release workflow could not run; no packages were published.
