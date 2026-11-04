---
title: Changelog
description: Release history of ttm and ttm-mcp.
order: 18
section: more
---

`ttm` and `ttm-mcp` are released under the same version. Downloads: <https://github.com/toantranmei/ttm/releases>.

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
