---
title: Error codes
description: The stable error codes returned by the API, ttm and ttm-mcp, what they mean and how to handle them.
order: 14
section: reference
---

## What an error looks like

The server returns errors as `application/problem+json`:

```json
{
  "code": "insufficient_scope",
  "title": "this token lacks the scope notee:write",
  "status": 403,
  "params": { "required": "notee:write" }
}
```

| Field | Meaning |
| --- | --- |
| `code` | Stable code to branch on (tables below) |
| `title` | An English sentence for humans |
| `status` | HTTP status code |
| `field` | The input field in error (for `bad_request`, `conflict`), for example `anchor.quote.exact` |
| `rule` | The specific rule violated (for `bad_request`, `conflict`, `precondition_failed`), for example `split-limit` |
| `params` | Related values, for example `{"required": "notee:write"}`, `{"max_depth": 2, "per_run": 10}` |

`ttm` prints errors to stderr in one pattern (since 0.2.0):

```text
✕ <summary>
  rule: <rule>  field: <field>
  Fix: <how to fix>
  Help: https://toantranmei.com/help/ttm/ma-loi#<rule>
```

`rule` is the server's `rule`, else its `code`. Example:

```text
✕ At most 10 subtasks per run (10 already).
  rule: split-limit
  Fix: split into fewer subtasks (see params.per_run)
  Help: https://toantranmei.com/help/ttm/ma-loi#split-limit
```

Usage errors (`missing_flag`, `confirmation_required`, `unsupported_feature`, `invalid_flag`) exit with code **2**; other
errors with 1. With `--plain` the first line is `ERROR: <summary>`.

With `--json`, stdout also gets `{"error": {"code", "message", "status", "rule", "field", "params"}}` (the last three
only when the server sends them) so scripts can branch on `rule`. MCP tools return `isError: true`, the text
`<message> (<code>)` and `structuredContent.error` with the same fields: agents branch on `rule` (`split-limit`,
`rev-conflict`, `approval-required`) or read `params.required` of `insufficient_scope`.

> **Note:** for `bad_request`, `conflict` and `precondition_failed` errors, `code` is generic; the specific cause is in
> `rule` (the **Rule** column below) and the message.

## Authentication and permissions

| Code | HTTP | Meaning | How to handle |
| --- | --- | --- | --- |
| `invalid_token` | 401 | Token is wrong, expired or revoked | Run `ttm login` again; check for an old `TTM_TOKEN` |
| `unauthorized` | 401 | No token | `ttm login` or set `TTM_TOKEN` |
| `insufficient_scope` | 403 | Token lacks a scope; `params.required` names the scope needed | Sign in with that scope (`ttm login --scope …`) or create a new PAT |
| `token_not_allowed` | 403 | This route isn't open to tokens (managing tokens, sharing, billing, deleting pages, creating workspaces…) | Do it on the web |
| `forbidden` | 403 | You don't have permission on this resource | Ask the page/workspace owner for access |
| `upgrade_required` | 426 | `ttm` is older than the server's `min_cli` | `ttm self-update` |

## Signing in (OAuth)

| Code | Meaning | How to handle |
| --- | --- | --- |
| `access_denied` | You clicked **Deny** | Run `ttm login` again |
| `authorization_pending`, `slow_down` | (device flow) waiting for you to allow / polling too fast | `ttm` handles it, nothing to do |
| `expired_token` | Device code older than 10 minutes | Run `ttm login --device` again |
| `invalid_grant` | Code already used or expired; wrong PKCE; refresh token expired, nonexistent, or **reused** (the whole token family is revoked) | Run `ttm login` again |
| `invalid_client` | Client doesn't exist | Check `--host` |
| `invalid_scope` | Unknown scope in `--scope` | Use a correct scope name (see [Signing in and tokens](dang-nhap.md)) |
| `invalid_request` | Missing parameter | Report a bug if you hit it with `ttm` |
| `invalid_target` | (remote MCP) `resource` isn't `…/api/v1/mcp` | Use the correct MCP URL |

## General

| Code | HTTP | Meaning |
| --- | --- | --- |
| `bad_request` | 400 | Invalid input; see `rule` and `field` |
| `not_found` | 404 | Doesn't exist, or you can't see it (the server doesn't distinguish the two) |
| `conflict` | 409 | State conflict; see `rule` |
| `precondition_failed` | 412 | Data changed since you read it (`rule: rev-conflict`); read again and retry |
| `rate_limited` | 429 | Over a limit (for example 60 Notee writes per hour per token); wait and retry |
| `internal` | 500 | Server error; retry later, report a bug if it keeps happening |
| `http_<code>` | any | A response that isn't problem+json (proxy, 502 gateway…) |

Local `ttm` codes: `network` (couldn't connect), `decode` (unexpected response), `invalid_url`, `error` (anything else:
not signed in, a file that can't be read…).

## Notee: writing pages

| Code / Rule | HTTP | Meaning | How to handle |
| --- | --- | --- | --- |
| `too_many_blocks` | 413 | The write adds, changes or deletes more than 1,000 blocks; or repeats `<unknown>`/`<truncated>` too much | Split it up (several `update_content`, or create and then add gradually) |
| `markdown_too_large` | 413 | Markdown over 2 MB | Split it up |
| `would_delete_children` | 409 | The new content drops a child page or child database | Keep the `<page>`/`<database>` tags, or add `allow_deleting_content` / `--allow-deleting-content` if you really want to delete them |
| `would_delete_truncated` | 409 | Drops the `<truncated/>` line of a long page | Keep that line at the end, or `allow_deleting_content` |
| rule `one_of` | 400 | Nothing to change, or both `update_content` and `replace_content` used | Pick one |
| rule `suggest_content_only` | 400 | A suggestion with `title`/`properties` | Change the title and columns with `mode: apply` |
| rule `mode` | 400 | `mode` other than `apply`/`suggest` | |
| rule `is_database` | 400 | Reading a database as a page | Use `notee-fetch` (returns the schema) or `GET /databases/{id}/schema` |
| rule `batch_size` | 400 | Creating 0 or more than 20 pages at once | 1–20 |
| rule `parent_shape` | 400 | `parent` has more than one of `page_id`, `database_id`, `workspace_id` | Exactly one |
| `rate_limited` | 429 | Over 60 agent writes per hour (per token) | Batch changes into fewer writes |

Warnings (not errors, found in `warnings`): `update_content[i]: old_str not found; skipped`,
`… old_str matches N places; give more context or replace_all; skipped`, `old_str is empty`, dropped tags
(`<unknown url=…> is not a block of this page; left out`, `<page> without the url of an existing page; left out`…).

## Notee: comments and suggestions

| Code / Rule | HTTP | Meaning | How to handle |
| --- | --- | --- | --- |
| `anchor_not_found` | 422 | The quote isn't in the page (or the resource has no text) | Copy the exact text from `notee-fetch`/`ttm notes get` |
| `ambiguous_anchor` | 422 | The quote appears in several places | Add `prefix` and/or `suffix` |
| rule `quote_length` | 400 | `exact` is empty or longer than 1,000 characters | |
| `stale` (in `stale_change_ids`) | | A change in the suggestion whose anchor is gone | Suggest again on the new content |

## Notee: databases

| Code / Rule | HTTP | Meaning | How to handle |
| --- | --- | --- | --- |
| `sql_syntax` | 400 | SQL syntax error | See the position in the message |
| `sql_not_select` | 400 | Not exactly one `SELECT` statement (has DML, multiple statements) | A single `SELECT` |
| `sql_table` | 400 | Reads a table other than `rows` | Only `FROM rows` and your own CTEs |
| `sql_function` | 400 | Function not allowed | See the list of functions in [Notee](notee.md) |
| `sql_forbidden` | 400 | Forbidden clause (`SELECT INTO`, `FOR UPDATE`, `$1` parameters, a CTE named `rows`, nested `WITH`, `LATERAL`, `reg*` casts) | Remove that clause |
| `sql_error` | 400 | Postgres reported an error while running it | Fix according to the message |
| `query_timeout` | 408 | The query took longer than 3 seconds | Filter more, add `LIMIT` |
| `result_too_large` | 413 | One page of results is over 4 MB | Select fewer columns, lower `page_size` |
| `query_busy` | 429 | More than 2 of your queries at once, or the server is busy | Retry in a moment |
| `sql_sandbox_unavailable` | 503 | The server hasn't enabled safe SQL | Use the JSON filter |
| rule `sql_or_filter` | 400 | Both `sql` and `filter` given | One or the other |
| rule `unknown_prop` | 400 | A property in `filter`/`sorts` doesn't exist | Read the schema |
| rule `filter_shape` | 400 | Malformed filter | `{"property", "op", "value"}`, `{"and": []}`, `{"or": []}` |
| rule `invalid_cursor` | 400 | Broken cursor | Use `next_cursor` as given |
| `row_deleted` | 409 | The row is in the trash | Restore it on the web or create a new row |
| rule `row_elsewhere` | 400 | The id is a row of another database | |

## Factory

Most are `409 conflict` with the following `rule`:

| Rule | Meaning | How to handle |
| --- | --- | --- |
| `split-limit` | Exceeds the subtask limits (depth, per run, per root task) or the role isn't allowed to split; `params` has the current limits | Split into fewer or shallower subtasks |
| `approval-required` | The status change needs approval; or the change request must be approved by another role | `ttm task approve` by someone with the role |
| `tests-required`, `ci-required` | The last run has no passing tests / CI | Run tests, attach `test_result` |
| `blocked-by-dependency` | A dependency or subtask isn't done | |
| `wip-limit` | The target column is at its WIP limit | |
| `sprint-required` | The task must be added to a sprint | |
| `budget-exceeded` | The task's token budget is used up | |
| `transition-not-allowed` | The workflow doesn't allow the transition (or the status is unknown) | Check the workflow on the web |
| `rev-conflict` (412) | The task changed since you read it | Run the command again |
| `run-active` | The task already has a run in progress | Wait or `ttm agent stop` |
| `run-finished` | The run has already finished | |
| `not-offered` | The run is no longer waiting to be claimed | |
| `no-approval-pending` | The task isn't awaiting approval | |
| `not-pending`, `not-author` | The change request has already been decided / the author can't approve their own | |
| `merge-finished` | The merge queue entry is already done | |
| `key_taken` | The project key is already in use | Choose a different `--key` |
| `ambiguous-key` | The key exists in several projects | Use the task id |
| `dependency-cycle`, `not-found` (`field: dependencies`) | Circular or nonexistent dependency | |

`ttm sync` receives a `409` with a list of `conflicts` (task, field, base, local, server) and prints them; see
[Factory](factory.md).

## Runner

| Code | Meaning | How to handle |
| --- | --- | --- |
| `repo_not_configured` | The job's project has no clone in `runner.toml`, or the clone's `origin` doesn't match the job's repo | `ttm runner config --repo KEY=<clone path>` |
| `invalid_branch` | The job's branch name isn't valid for git | Fix the task/branch on the web |
| `invalid_worktree` | The job's `worktree_hint` is an absolute path, has `..`, or leads out of `worktree_root` | Report it as a server bug; the runner never creates a worktree outside `worktree_root` |
| `ttm_mcp_missing` | The runner cannot find the `ttm-mcp` binary (next to `ttm` or on `PATH`), which the agent's `ttm` MCP server runs | Reinstall ttm (it ships `ttm` and `ttm-mcp`); `ttm runner up` refuses to start without it, and a running runner takes no job until it is back |

These codes are reported to the server as `error` when the run finishes.

## MCP tools

| Code | Meaning |
| --- | --- |
| `no_run` | The tool needs an agent run (`TTM_RUN_ID`) |
| `no_task` | `task` is missing and `TTM_TASK` isn't set |
| `no_project` | `project` is missing and `TTM_PROJECT` isn't set |
| `unsupported` | The server doesn't support this tool |

## Common ttm messages

| Message | Meaning |
| --- | --- |
| `` not signed in to <host> (profile <p>); run `ttm login` `` | No token for this (profile, host) |
| `the saved token has expired; run ttm login` | Token expired, no refresh token |
| `the session expired and could not be renewed; run ttm login` | Renewal failed |
| `the server rejected this token (…)` | `ttm login --token` with an invalid token |
| `cannot reach <host>` | Network, DNS, or wrong `--host` |
| `this directory is not linked to a project; run ttm project link <KEY>` | A Factory command run outside a linked repo |
| ``no agent harness found; install Claude Code (`claude`) first`` | `claude` isn't on the runner's `PATH` |
| `timed out after 5 minutes waiting for the browser` | **Allow** wasn't clicked in time |
| `invalid TOML in <file>: …` | `config.toml`/`credentials.toml`/`runner.toml` has a syntax error |

## Rules and fixes

Each entry below is the target of a `Help:` line. **Fix** is the sentence `ttm` prints.

### `missing_flag`

A required flag is missing and nobody can be asked (`--yes`, or not a terminal). Exit code 2.

**Fix:** pass the flag shown in field, or run in a terminal to be asked

### `confirmation_required`

The action needs a confirmation but there is no terminal and no `--yes`. Exit code 2.

**Fix:** confirm in a terminal, or pass --yes

### `invalid_flag`

A flag value is invalid (for example a key that does not match `^[A-Z][A-Z0-9]{1,9}$`). Exit code 2.

**Fix:** fix the value of the flag shown in field (ttm <command> --help lists the choices)

### `unsupported_feature`

The server (or runner) lacks this feature (`discovery.features` misses its string, or a job `requires` a feature the runner does not have). `ttm` does not call the API. Exit code 2.

**Fix:** this server is older than this ttm: update the server or use another host

### `rev-conflict`

The object changed since the version you read (stale `If-Match`/`rev`).

**Fix:** someone changed it meanwhile: fetch it again (ttm sync) and redo the change

### `draft-stale`

The workflow draft changed since you read it (stale `base_version`).

**Fix:** the draft changed since you read it: show it again and reapply your edit

### `upgrade-preview-stale`

The workflow changed since the upgrade preview (`preview_hash` does not match).

**Fix:** the workflow changed since the preview: run the upgrade preview again

### `no-upgrade`

The project already uses the latest workflow version.

**Fix:** the project already uses the latest workflow version

### `dry-run-required`

The rule must be dry-run before it is enabled.

**Fix:** test the rule first: ttm automation test <rule> --task <KEY>

### `rule-published`

A published rule cannot be changed directly.

**Fix:** a published rule cannot change: edit the draft and publish again

### `input_required`

More input is needed (template, rule).

**Fix:** this needs more input: pass the missing value shown in field

### `stale`

The change request is out of date compared with the server.

**Fix:** the change request is out of date: sync again and send a new one

### `secret_in_file`

An as-code file (`.ttm/agents/*.yaml`) holds a secret-like key (`token`, `*_token`, `secret`, `password`, `api_key`).

**Fix:** remove the secret from the file and use a secret reference instead

### `invalid_file`

An as-code file is invalid for the server.

**Fix:** fix the file at the path shown (ttm workflow validate --file … lists every issue)

### `yaml`

A YAML file in `.ttm/` does not parse (checked locally before `ttm sync` sends it; with the line number).

**Fix:** fix the YAML at the line shown, then run ttm sync again

### `purchase_required`

The template is paid and you have not bought it.

**Fix:** buy the template on the web first, then install it again

### `sponsors_only`

The template is for Sponsors only.

**Fix:** this template is for Sponsors: see https://toantranmei.com/mei-er

### `insufficient_scope`

The token lacks a scope (`params.required`). Tokens made before 0.2.0 have no `factory:admin`.

**Fix:** sign in again with the missing scope: ttm login --scope '<scopes>'

### `unauthorized`

No token, or an invalid one.

**Fix:** sign in again: ttm login

### `forbidden`

Your role in the workspace does not allow it.

**Fix:** your role in this workspace does not allow it: ask an owner or admin

### `token_not_allowed`

This route is not open to tokens (web only).

**Fix:** this action is not open to tokens: do it on the web

### `job_token_not_allowed`

The token of an agent running a job cannot do this (administration, confirmations).

**Fix:** agents running a job cannot do this: run it yourself with your own token

### `not_found`

The object does not exist (or is not yours).

**Fix:** check the id or key (list commands show them)

### `seat_limit`

The workspace has no free seat for a member or agent.

**Fix:** the workspace has no free seat: free one or upgrade the plan on the web

### `plan_limit`

A plan limit is reached (MCP servers, rules…).

**Fix:** the plan limit is reached: upgrade the plan on the web

### `handle_taken`

The agent handle is already used in the workspace.

**Fix:** pick another --handle

### `skill_unchanged`

The new skill version is identical to the latest one (same SHA-256).

**Fix:** nothing changed since the last version: edit the skill, then push again

### `skill_limit`

Too many skills on the agent or workspace.

**Fix:** too many skills on this agent or workspace: remove one first

### `self_review`

You cannot review your own skill version (the workspace has 2 or more admins).

**Fix:** someone else must review your own skill version

### `skill_md_missing`

The skill has no `SKILL.md` at its root.

**Fix:** add a SKILL.md with a name and description front matter

### `front_matter`

The `SKILL.md` front matter lacks `name`/`description` or has invalid `ttm.tags`.

**Fix:** fix the front matter of SKILL.md

### `path_unsafe`

A path in the skill is unsafe (`..`, absolute, `\`, `.git/`, symlink).

**Fix:** remove the file or rename it to a plain relative path

### `too_large`

The skill is over 5 MB unpacked (or compresses more than 100×).

**Fix:** make the skill smaller (at most 5 MB)

### `too_many_files`

The skill has more than 200 files.

**Fix:** keep at most 200 files (list build output in .gitignore)

### `zip_invalid`

The skill bundle is not a valid zip.

**Fix:** pack the skill again with ttm skill push <dir>

### `skill_hash_mismatch`

The runner downloaded a skill whose SHA-256 does not match the job: the job is refused.

**Fix:** the skill download is corrupt: run the job again

### `ttm_mcp_missing`

The runner cannot find an executable `ttm-mcp` (next to `ttm` or on `PATH`). Without it the agent has no Factory tools, so the job is refused before any agent starts; `ttm runner up` refuses to start, and a runner already up takes no job until the binary is back.

**Fix:** ttm-mcp is missing next to ttm: reinstall ttm (it ships both), then restart the runner

### `secrets_disabled`

Secrets are turned off on the server.

**Fix:** secrets are turned off on this server: ask its operator

### `storage_disabled`

File storage is turned off on the server.

**Fix:** file storage is turned off on this server: ask its operator

### `oauth_client_unavailable`

This MCP server cannot sign in with OAuth.

**Fix:** this MCP server cannot sign in by OAuth: use --auth workspace with a secret

### `repo_not_configured`

The runner has no clone of the project in `runner.toml`, or its `origin` does not match.

**Fix:** link the repository first: ttm project link <KEY>

### `invalid_branch`

The job's branch name is invalid.

**Fix:** fix the task or branch on the web

### `invalid_worktree`

The job's `worktree_hint` leaves `worktree_root`.

**Fix:** report it: the runner never creates worktrees outside its root

### `confirm_expired`

The confirmation is older than 10 minutes.

**Fix:** the confirmation expired (10 minutes): start the action again

### `confirm_used`

The confirmation was already applied (each runs once, even when the action failed).

**Fix:** this confirmation was already applied: start the action again for a new one

### `confirm_cancelled`

The confirmation was cancelled.

**Fix:** this confirmation was cancelled: start the action again

### `confirm_tampered`

The staged request does not match the confirmation's hash.

**Fix:** the staged request does not match its confirmation: start the action again

### `confirm_stale`

(ttm only) the request changed between confirmation and apply. The server does not send it: a changed draft makes the target route answer `draft-stale`/`upgrade-preview-stale`.

**Fix:** the request changed since it was confirmed: start the action again

### `bearer_required`

Two-step confirmations only accept a Bearer token (not the browser cookie).

**Fix:** confirmations need a token: use ttm or an MCP client, not the browser session

### `not_confirmable`

`(method, path)` is not in the list of confirmable actions.

**Fix:** this change cannot go through a confirmation: run it directly

### `payload_too_large`

Body over 256 KiB or preview over 64 KiB.

**Fix:** the request is too large: send a smaller change

### `split-limit`

Over the subtask split limit (`params.per_run`).

**Fix:** split into fewer subtasks (see params.per_run)

### `approval-required`

A person must approve this step.

**Fix:** a person must approve this step: ttm task approve <KEY>

### `agent-own-runner`

A full/custom-tool agent runs only on its delegator's runners.

**Fix:** this agent runs only on its delegator's runners: start one with ttm runner up

### `too_many_blocks`

The page is too long for one call.

**Fix:** the page is too long for one call: read it by parts (--block)

### `upgrade_required`

`ttm` is older than the server's `min_cli`.

**Fix:** update ttm: ttm self-update

### `network`

Network error or wrong host.

**Fix:** check the network and the host (ttm doctor)

### `rate_limited`

Too many requests (for example over 30 confirmations a minute).

**Fix:** wait a little and retry

### `conflict`

The object changed while you edited it.

**Fix:** it changed meanwhile: fetch it again and retry

### `bad_request`

Invalid input (see `field`, `rule`).

**Fix:** fix the input named in field

### `authorization_pending`

The device sign-in is not approved yet.

**Fix:** finish signing in on the web page, then retry

### `invalid_grant`

The code or refresh token expired or was used.

**Fix:** sign in again: ttm login
