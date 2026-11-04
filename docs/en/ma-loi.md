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

`ttm` prints `error: <title> (<code>, HTTP <status>) <params>` to stderr; with `--json` it also prints
`{"error": {"code", "message"}}` to stdout. MCP tools return `isError: true`, the text `<message> (<code>)` and
`structuredContent.error.code`.

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

These two codes are reported to the server as `error` when the run finishes.

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
| `not signed in to <host> (profile <p>); run ttm login` | No token for this (profile, host) |
| `the saved token has expired; run ttm login` | Token expired, no refresh token |
| `the session expired and could not be renewed; run ttm login` | Renewal failed |
| `the server rejected this token (…)` | `ttm login --token` with an invalid token |
| `cannot reach <host>` | Network, DNS, or wrong `--host` |
| `this directory is not linked to a project; run ttm project link <KEY>` | A Factory command run outside a linked repo |
| `no agent harness found; install Claude Code (claude) first` | `claude` isn't on the runner's `PATH` |
| `timed out after 5 minutes waiting for the browser` | **Allow** wasn't clicked in time |
| `invalid TOML in <file>: …` | `config.toml`/`credentials.toml`/`runner.toml` has a syntax error |
