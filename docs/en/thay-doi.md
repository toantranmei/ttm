---
title: Changelog
description: Release history of ttm and ttm-mcp.
order: 18
section: more
---

`ttm` and `ttm-mcp` are released under the same version. Downloads: <https://github.com/toantranmei/ttm/releases>.

## 0.1.4 (2026-11-04)

0.1.3 was tagged but never released (its tagged commit carried `[skip ci]`, so the release workflow did not run); 0.1.4 is the first release with the changes below.

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
