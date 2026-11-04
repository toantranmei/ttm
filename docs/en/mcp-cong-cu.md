---
title: MCP tools
description: Every tool of ttm-mcp and the remote MCP - purpose, parameters, example calls and results, required scopes, errors.
order: 11
section: reference
---

This page applies to both `ttm-mcp` (stdio) and the remote MCP at `https://toantranmei.com/api/v1/mcp`: same tool names,
same parameters, same results. Version **0.1.4**: 26 tools in 3 toolsets and 1 resource.

## Conventions

- Parameters are a JSON object; **unknown parameters are rejected**. A `null` or omitted parameter uses its default.
- Every page/database parameter accepts its id or any of its URLs.
- Each tool returns **text content** (Markdown for the model, or JSON) and **`structuredContent`** (JSON).
- Annotations: read tools have `readOnlyHint: true`, `idempotentHint: true`; write tools have `readOnlyHint: false`;
  tools that can delete or overwrite data have `destructiveHint: true`. No tool reaches outside toantranmei.com
  (`openWorldHint: false`).

### Errors

| Form | When | What the model sees |
| --- | --- | --- |
| Protocol error `invalid_params` (-32602) | Unknown tool name; a parameter of the wrong type, missing or unknown; a local check fails (for example `give 1 to 20 pages`) | A JSON-RPC error message |
| Result with `isError: true` | The server refuses or fails | The text `<message> (<code>)`, and `structuredContent: {"error": {"code", "message", "status", "rule", "field", "params"}}` |

`code` is a server code (`unauthorized`, `invalid_token`, `insufficient_scope`, `not_found`, `too_many_blocks`…, see
[Error codes](ma-loi.md)) or a code from the tool itself:

| Code | Meaning |
| --- | --- |
| `no_run` | The tool needs an agent run (`TTM_RUN_ID` is not set) |
| `no_task` | No `task` passed and there is no current task (`TTM_TASK`) |
| `no_project` | No `project` passed and `TTM_PROJECT` is not set |
| `unsupported` | This server does not support the tool |
| `network`, `decode` | Could not reach the API / unexpected response |

A server error keeps its `status`, `rule` (the check that failed), `field` (the input) and `params` (the values involved)
when the server sends them; the text names the `rule` and `field` too. Agents branch on `rule` instead of the wording:
`split-limit` (split into fewer or shallower subtasks; `params` has the limits), `rev-conflict` (read the task again and
retry), `approval-required` (ask with `factory-request-approval`); `insufficient_scope` has the missing scope in
`params.required`.

401 errors come with the hint ``Sign in again with `ttm login`.``

## Resource

| URI | Name | Type | Content |
| --- | --- | --- | --- |
| `notee://spec/markdown` | `notee-markdown` | `text/markdown` | The Notee-flavored Markdown spec (available when the `notee` toolset is on) |

---

## workspace toolset

Always on. Scope: `workspace:read`.

### workspace-get-self

The signed-in user (id, name, email, plan) and the current workspace. Call it first.

Parameters: none.

```json
{
  "user": { "id": "0190f3a1-…", "email": "an@example.com", "name": "An Nguyễn", "avatar_url": null, "locale": "vi", "plan": "free" },
  "workspace": { "id": "0190f3a2-…", "name": "Team Aurora", "kind": "team", "role": "member", "slug": "aurora" }
}
```

Current workspace = the profile's workspace (`ttm workspace switch`), or the workspace the token is restricted to (remote
MCP); otherwise the personal workspace.

### workspace-list

The user's workspaces, with their kind (`personal`/`team`) and role (`owner`, `admin`, `member`, `guest`).

Parameters: none. Result: `{"workspaces": [{id, name, kind, role, slug?}]}`.

### workspace-get-users

The members (people) of a workspace and their roles, to find someone to mention or assign work to.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `workspace_id` | string | | Defaults to the current workspace |
| `query` | string | | Filter by name or email, case-insensitive |

```json
{ "query": "bình" }
```

```json
{ "users": [{ "user": { "id": "0190…", "name": "Bình", "email": "binh@example.com", "avatar_url": null }, "role": "member" }] }
```

Errors: `forbidden` (guests cannot see the member list).

---

## notee toolset

### notee-search

Search pages you can view by title and content (diacritic-insensitive). Scope `notee:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | yes | Words to search for; every word must be present; ≤ 8 words, ≤ 200 characters |
| `limit` | integer | | 1–20, default 10 |
| `workspace_id` | string | | Defaults to the current workspace |

```json
{ "query": "ke hoach quy 4", "limit": 5 }
```

Text: `- Kế hoạch quý 4 (id: 0190f3b0-…): …snippet…`. Structured:
`{"results": [{id, workspace_id, title, icon, snippet}]}` (`snippet` is empty when only the title matches).

Errors: `invalid_params` when `query` is empty; `bad_request` when the query is too long.

### notee-fetch

Read a page as [Notee-flavored Markdown](notee-markdown.md), or the properties and views of a database. Scope
`notee:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | yes | Id or URL of a page, database row or database |
| `block` | string | | An id from `unknown_block_ids` of a truncated page, to read that part |

Text for a page:

```text
<page id="0190f3b0-…" title="Kế hoạch quý 4" url="https://toantranmei.com/notes/0190f3b0-…">
# Mục tiêu
- Phát hành ttm 0.2
</page>
```

A truncated page adds the line ``The page was truncated. Read the missing parts with notee-fetch and `block` set to one
of: …``. A database row also has `<properties>{…}</properties>`. Structured: `{"kind": "page", id, title, icon, url,
parent, properties?, markdown, truncated, unknown_block_ids}` or `{"kind": "database", id, title, url, properties[],
views[]}`.

The text for a database is a `| Property | Type |` table.

Errors: `not_found` (does not exist or you cannot view it).

### notee-query-database

Query a database's rows with read-only SQL or a JSON filter. Scope `notee:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `database` | string | yes | Id or URL |
| `sql` | string | one of the two | `SELECT … FROM rows …`; columns are property names |
| `filter` | object | one of the two | `{"and": [{"property", "op", "value"}]}` |
| `sorts` | array | | With `filter`: `[{"property": "Due", "direction": "asc"}]` |
| `page_size` | integer | | 1–100 (default 100) |
| `cursor` | string | | `next_cursor` from the previous page |

```json
{ "database": "0190f3c0-…", "sql": "SELECT \"Name\", \"Status\" FROM rows WHERE \"Status\" = 'Done' ORDER BY \"Due\" LIMIT 20" }
```

```json
{ "columns": ["Name", "Status"], "rows": [{ "id": "", "values": { "Name": "Viết tài liệu", "Status": "Done" } }], "next_cursor": null }
```

`filter` operators: `eq`, `neq`, `contains`, `not_contains`, `starts_with`, `ends_with`, `gt`, `gte`, `lt`, `lte`,
`in`, `not_in`, `contains_all`, `is_empty`, `is_not_empty`, `between`, `is_relative`. SQL and filter rules:
[Notee](notee.md).

Errors: `invalid_params` when both are missing or both are given; `sql_syntax`, `sql_not_select`, `sql_table`,
`sql_function`, `sql_forbidden`, `sql_error`, `bad_request` (`unknown_prop`, `filter_shape`, `invalid_cursor`),
`query_timeout`, `result_too_large`, `query_busy`, `sql_sandbox_unavailable`.

### notee-get-comments

A page's comment threads, with the quote each thread is anchored to. Scope `notee:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `page` | string | yes | Id or URL |
| `include_resolved` | boolean | | Include resolved threads (default `false`) |

```json
{ "threads": [{ "id": "0190f3e0-…", "anchor": { "kind": "text", "block_id": "b7", "quote": "ra mắt tháng 12" },
  "quote": { "exact": "ra mắt tháng 12", "prefix": null, "suffix": null }, "resolved": false,
  "comments": [{ "id": "…", "author": "An Nguyễn", "text": "Chốt ngày cụ thể chưa?", "created_at": "2026-11-04T05:00:00Z" }] }] }
```

### notee-create-pages

Create 1–20 pages under a page, as rows of a database, or at the top level of a workspace. Scope `notee:write`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `parent` | object | yes | Exactly **one** of `page_id`, `database_id`, `workspace_id` |
| `pages` | array | yes | 1–20 pages, ≤ 1,000 blocks in total |
| `pages[].title` | string | yes | Title |
| `pages[].icon` | string | | Emoji |
| `pages[].content` | string | | Markdown content |
| `pages[].properties` | object | | Column values by property name, when the parent is a database |

```json
{ "parent": { "page_id": "0190f3b0-…" }, "pages": [{ "title": "Biên bản 04/11", "icon": "📝", "content": "## Quyết định\n- Phát hành thứ Sáu" }] }
```

```json
{ "pages": [{ "id": "0190f3c4-…", "title": "Biên bản 04/11", "url": "https://toantranmei.com/notes/0190f3c4-…" }] }
```

Errors: `invalid_params` (`give 1 to 20 pages`, `parent needs exactly one of page_id, database_id, workspace_id`),
`too_many_blocks`, `markdown_too_large`, `rate_limited`, `forbidden`.

### notee-update-page

Edit a page: find-and-replace (`update_content`) or replace the whole content (`replace_content`); also changes the title
and the column values of a database row. `mode: "suggest"` creates a suggested edit instead of writing. Scope
`notee:comment` (suggestions) or `notee:write` (writes). Read the page with `notee-fetch` first.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `page` | string | yes | Id or URL |
| `update_content` | array | | `[{old_str, new_str, replace_all?}]`; `old_str` matches exactly, case-sensitive, in exactly one place unless `replace_all` |
| `replace_content` | string | | The entire new content; keep every `<unknown/>` tag you read |
| `allow_deleting_content` | boolean | | Allow dropping child pages, child databases, the `<truncated/>` part (default `false`) |
| `title` | string | | New title |
| `properties` | object | | Column values to change (database row) |
| `mode` | `"apply"` \| `"suggest"` | | Defaults to the workspace setting (`agent_write_mode`); always `suggest` if the token lacks `notee:write` |

At least one change is required; do not use `update_content` together with `replace_content`. Suggestions change content
only.

```json
{ "page": "0190f3b0-…", "update_content": [{ "old_str": "ra mắt tháng 11", "new_str": "ra mắt tháng 12" }], "mode": "suggest" }
```

Text: `Suggestion <id> created; people with edit access can accept or reject it.`, `Page updated.` or
`No change applied.`, plus `Warning: update_content[0]: old_str not found; skipped`. Structured:
`{applied, warnings, seq, snapshot_seq, suggestion_id?}`.

Errors: `invalid_params` (`nothing to change`, both methods used), `would_delete_children`, `would_delete_truncated`,
`too_many_blocks`, `markdown_too_large`, `insufficient_scope` (`mode: apply` without `notee:write`),
`bad_request` (`suggest_content_only`, `one_of`), `rate_limited`, `forbidden`.

### notee-move-pages

Move pages under another page, into a database or to the top level of a workspace. Scope `notee:write`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `pages` | array of string | yes | Ids or URLs of the pages |
| `parent` | object | yes | Exactly one of `page_id`, `database_id`, `workspace_id` |

Result: `{"pages": [{id, title, url}]}`. Pages are moved one by one; an error on a page stops there.

### notee-duplicate-page

Duplicate a page with its content. Scope `notee:write`.

| Parameter | Type | Required |
| --- | --- | --- |
| `page` | string | yes |

Result: `{id, title, url}` of the copy.

### notee-create-database

Create a database. Scope `notee:write`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `parent` | object | yes | Exactly one of `page_id`, `database_id`, `workspace_id` |
| `title` | string | yes | |
| `properties` | array | | `[{"name": "Status", "type": "select", "options": [{"name": "Todo"}, {"name": "Done"}]}]`; types: `title`, `text`, `number`, `select`, `multi_select`, `status`, `date`, `person`, `checkbox`, `url`, `email`, `phone`, `relation` |

Result: `{id, title, url}`.

### notee-update-database

Add, change and remove properties and views, rename the database. Scope `notee:write`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `database` | string | yes | Id or URL |
| `changes` | object | yes | `{"add_properties": [...], "update_properties": [...], "remove_properties": ["Name"], "views": [...], "title": "…"}` |

Result: the server's response (JSON).

### notee-upsert-rows

Create or update 1–100 rows. Scope `notee:write`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `database` | string | yes | Id or URL |
| `rows` | array | yes | 1–100 items `{id?, values}` |
| `rows[].id` | string | | Present: update the row; absent: create a row |
| `rows[].values` | object | yes | Values by property name; options by name |

```json
{ "database": "0190f3c0-…", "rows": [{ "values": { "Name": "Viết changelog", "Status": "Todo" } }] }
```

Result: `{"ids": ["0190f3c7-…"]}`. Errors: `invalid_params` (`give 1 to 100 rows`), `row_deleted`, `bad_request`
(`row_elsewhere`).

### notee-delete-rows

Delete rows (move them to the trash). Scope `notee:write`. `destructiveHint: true`.

| Parameter | Type | Required |
| --- | --- | --- |
| `database` | string | yes |
| `ids` | array of string | yes (non-empty) |

### notee-create-comment

A comment anchored to a quote of the page, a comment on the whole page, or a reply to a thread. Scope `notee:comment`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `page` | string | yes | Id or URL |
| `text` | string | yes | Content (non-empty) |
| `quote` | object | | `{exact, prefix?, suffix?}`; omitted = a comment on the whole page |
| `quote.exact` | string | yes (within `quote`) | The text exactly as it appears in the page, 1–1,000 characters |
| `quote.prefix`, `quote.suffix` | string | | A few words right before / after, when `exact` appears more than once |
| `thread_id` | string | | Reply to this thread instead of opening a new one |

```json
{ "page": "0190f3b0-…", "text": "Nguồn số liệu?", "quote": { "exact": "tăng 30%", "suffix": " so với quý trước" } }
```

Errors: `anchor_not_found`, `ambiguous_anchor` (add `prefix`/`suffix`), `bad_request` (`quote_length`).

### notee-get-suggestions

Suggested edits on a page: author, agent, each change with its anchor and the Markdown before/after. Scope `notee:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `page` | string | yes | Id or URL |
| `include_resolved` | boolean | | Include accepted, rejected and stale suggestions (by default only `pending`) |

Each suggestion: `id`, `status` (`pending`, `accepted`, `rejected`, `stale`), `author`, `via_agent`, `created_at`,
`changes[]` (`id`, `kind` = `insert`/`delete`/`replace`, `anchor`, `before_nfm`, `after_nfm`).

### notee-resolve-suggestion

Accept (all or some of the changes) or reject a suggestion. Requires edit access to the page. Scope `notee:write`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `suggestion` | string | yes | Suggestion id |
| `decision` | `"accept"` \| `"reject"` | yes | |
| `change_ids` | array of string | | Accept only these changes (ignored when rejecting) |

Result: `{id, status, stale_change_ids}`; changes whose anchor is gone are `stale` and are not applied.

---

## factory toolset

For agents **inside a run** of a runner (variables `TTM_RUN_ID`, `TTM_TASK`, `TTM_PROJECT`, see
[Connecting MCP](mcp.md)). The job token has `factory:run` (which includes `factory:read`).

### factory-get-task

The current task (or another task by key): title, state, role, assignee, parent task. Scope `factory:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `task` | string | | Key (`AUR-12`) or id; defaults to the run's task |

Result: a task object (camelCase): `id` (key), `key`, `uuid`, `title`, `state`, `category`, `role`, `assignee`,
`assigneeMember{kind, id, name}`, `parentId`, `estimate`, `description`, `rev`… Errors: `no_task`, `not_found`.

### factory-get-context

The context the server has assembled for the task: description, acceptance criteria, related Notee pages, sibling tasks,
suggested files. Read it before you start. Scope `factory:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `task` | string | | Defaults to the run's task |

Without `task` inside a run: the run's context; otherwise: the task's context. When the server returns `markdown`, the
model receives that Markdown.

### factory-report-progress

Report progress to the Execution Map (shown live to the whole team). Scope `factory:run`. Requires a run.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `message` | string | yes | One or two sentences: what was done / what is in progress |
| `percent` | integer | | 0–100 |

Result: `{"ok": true}`, text `Progress reported.` Errors: `invalid_params` (`percent must be 0-100`), `no_run`,
`conflict` (`run-finished`).

### factory-request-approval

Ask a person (or the role the workflow specifies) for approval before continuing. Scope `factory:run`. Requires a run.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `reason` | string | yes | What needs approval and why |

Text: `Approval requested. Wait for the decision before doing what needs approval.` The approver gets a notification.

### factory-create-subtasks

Split the task into real subtasks. Scope `factory:run` or `factory:write`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `parent` | string | | Parent task key; defaults to the run's task |
| `tasks` | array | yes | Non-empty; at most 10 per run |
| `tasks[].title` | string | yes | |
| `tasks[].role` | string | | `PM`, `BA`, `UI/UX`, `DEV`, `GUARDIAN`, `QC` or a workspace role |
| `tasks[].description` | string | | |
| `tasks[].estimate` | integer | | Points or hours, whichever the project uses |
| `tasks[].type` | string | | `feature`, `bug`, `chore`… |
| `tasks[].priority` | string | | |
| `tasks[].labels` | array of string | | |
| `tasks[].depends_on` | array of string | | Keys of existing tasks, or `#<n>` = the nth subtask (from 0) of this call |
| `tasks[].acceptance_criteria` | array of string | | |

```json
{ "tasks": [{ "title": "API callback", "role": "DEV", "estimate": 2 }, { "title": "Test đăng nhập", "role": "QC", "depends_on": ["#0"] }] }
```

Result: `{tasks: [...], warnings: [...]}`. Limits: 10 per run, 30 under one root task, 2 levels deep, roles that are
allowed to split. Exceeding them: a `conflict` error (rule `split-limit`) with a message and `params` (`max_depth`,
`per_run`, `per_root`, `depth`, `run_count`, `root_count`) so the agent can split again.

### factory-attach-artifact

Attach a result to the run: pull request, test result, screenshot, link, note. Scope `factory:run`. Requires a run.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `kind` | string | yes | `pr` (recorded as the run's PR), `test_result`, `ci`, `screenshot`, `link`, `note` |
| `url` | string | | |
| `text` | string | | Content (test log, note) |
| `title` | string | | |
| `passed` | boolean | | For `test_result` and `ci` |

At least one of `url`, `text`, `passed` is required.

```json
{ "kind": "test_result", "title": "cargo test", "passed": true, "text": "test result: ok. 42 passed" }
```

### factory-get-rules

The project's rules for agents (branches, commits, tests before done). Scope `factory:read`.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `project` | string | | Key or id; defaults to the run's project (`TTM_PROJECT`) |

Text: the managed Markdown block (the same content `ttm rules apply` writes to `AGENTS.md`). Structured: `rev`,
`managed_markdown`, `hooks`, `checks[{id, description, command, enabled}]`.

### factory-renew-lease

Take or renew a lease on an area of code (glob) so other agents do not edit it at the same time. Scope `factory:run`.
Requires a run.

| Parameter | Type | Required | Description |
| --- | --- | --- | --- |
| `paths` | array of string | yes | Globs, for example `src/auth/**` |

Result: `{granted, expires_at, held_by, paths}`. If another run holds the area: `granted: false` and `held_by` says who
holds it; work on something else, or wait and call again.
