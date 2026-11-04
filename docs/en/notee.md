---
title: Notee
description: Search, read, create, and edit Notee pages, suggest edits, query and write databases, and comment on quoted text, from the CLI or MCP.
order: 7
section: guides
---

Everything below works from the **CLI** (`ttm notes …`) or **MCP** (the `notee-*` tools). Both call the same API, so the
rules, limits, and errors are the same. Pages are read and written as [Notee-flavored Markdown](notee-markdown.md).

| Task | CLI | MCP tool | Scope |
| --- | --- | --- | --- |
| Search pages | `ttm notes search` | `notee-search` | `notee:read` |
| Read a page or database schema | `ttm notes get` | `notee-fetch` | `notee:read` |
| Create pages | `ttm notes create` | `notee-create-pages` | `notee:write` |
| Edit a page | `ttm notes edit` | `notee-update-page` | `notee:write` (or `notee:comment` to suggest) |
| Suggest edits | `ttm notes edit --suggest`, `notes suggestions`, `notes resolve` | `notee-update-page` (`mode: suggest`), `notee-get-suggestions`, `notee-resolve-suggestion` | `notee:comment`, `notee:write` to review |
| Query a database | `ttm notes query` | `notee-query-database` | `notee:read` |
| Write and delete rows | `ttm notes rows upsert/delete` | `notee-upsert-rows`, `notee-delete-rows` | `notee:write` |
| Comment | `ttm notes comments`, `ttm notes comment` | `notee-get-comments`, `notee-create-comment` | `notee:read`, `notee:comment` |
| Move, duplicate, create/update databases | (no command yet) | `notee-move-pages`, `notee-duplicate-page`, `notee-create-database`, `notee-update-database` | `notee:write` |

## IDs and URLs

Every page/database argument accepts the **id** (UUID) or **any URL** of it: `ttm` takes the last UUID in the path, even
when it sits at the end of a slug.

```sh
ttm notes get 0190f3b0-1111-7aaa-8bbb-0123456789ab
ttm notes get https://toantranmei.com/notes/0190f3b0-1111-7aaa-8bbb-0123456789ab
ttm notes get "https://toantranmei.com/notes/ke-hoach-quy-4-0190f3b0-1111-7aaa-8bbb-0123456789ab?x=1"
```

A workspace is always an id (see `ttm workspace list`).

## Search

```sh
ttm notes search "ke hoach" --limit 5
ttm notes search "OKR" --workspace 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

- Searches the titles and content of pages you can view, **ignoring diacritics and case**; every word must be present.
- Title matches rank first, then recently edited pages. The server returns at most 20 results; `--limit` (default 10)
  trims them.
- By default the search runs in the profile's current workspace; with no workspace selected, it searches every workspace.
- A query is at most 8 words and 200 characters (`400 bad_request: query too long`).

## Read

```sh
ttm notes get <page>                    # Markdown to stdout
ttm notes get <page> -o page.md         # write to a file, prints "Saved <title> (page.md)."
ttm notes get <page> --json             # adds id, title, icon, url, parent, properties, truncated, unknown_block_ids
ttm notes get <database>                # title and property list
```

```text
# Việc nhóm
- Name (title)
- Status (status)
- Due (date)
- Owner (person)
```

- If the id is a **database row**, `get` returns the row's page content; the column values are in `properties` (only
  with `--json`).
- **Long pages** (more than 20,000 nodes) are truncated: the last Markdown line is `<truncated url="…" blocks="n"/>`, and
  the result has `truncated: true` and `unknown_block_ids`. The CLI prints a reminder on stderr:
  `note: page truncated; read the rest with --block <id>: b1, b2`. Read the rest piece by piece with `--block <id>`
  (MCP: `block`).
- Blocks that Markdown cannot represent (sketches, embeds, synced blocks…) show up as `<unknown url="…" alt="…"/>`; their
  ids are also in `unknown_block_ids`.

Over MCP, `notee-fetch` returns Markdown wrapped in `<page id="…" title="…" url="…">…</page>` for the model, and
structured data (`kind: "page"` or `kind: "database"`) for the client.

## Create pages

```sh
ttm notes create --parent <page> -f ke-hoach.md
ttm notes create --workspace <workspace-id> -f note.md --title "Ghi chú họp" --icon "📝"
ttm notes create --database <database> -f task.md --title "Viết tài liệu"     # creates a row
cat note.md | ttm notes create --parent <page> -f -
```

```text
Created Ghi chú họp (https://toantranmei.com/notes/0190f3c4-…)
```

- Choose exactly **one** location: `--parent` (parent page), `--database` (becomes a row of the database), or
  `--workspace` (top level of the workspace).
- Title: `--title`, otherwise the file's first `# ` line, otherwise the file name (without extension). That `# ` line
  stays in the content; remove it from the file if you don't want the title repeated.
- At most 1,000 blocks and 2 MB of Markdown per page.

MCP `notee-create-pages` creates 1–20 pages at once (≤ 1,000 blocks in total). Each page has `title`, `icon`, `content`
(Markdown), and `properties` (column values by name when the parent is a database):

```json
{
  "parent": { "database_id": "0190f3c0-…" },
  "pages": [
    { "title": "Viết tài liệu", "properties": { "Status": "Doing", "Due": "2026-11-15" } },
    { "title": "Phát hành 0.2", "icon": "🚀", "content": "## Việc cần làm\n- [ ] Changelog" }
  ]
}
```

## Edit a page

### Find and replace (recommended)

```sh
ttm notes edit <page> --old "Q3" --new "Q4"
ttm notes edit <page> --old "Trạng thái: nháp" --new "Trạng thái: duyệt" --old "v0.1" --new "v0.2"
ttm notes edit <page> --old "TODO" --new "DONE" --replace-all
```

- Each `--old` pairs with the `--new` in the same position; the counts must match. Pairs apply **in order** to the
  current Markdown.
- `--old` matches literally, **case-sensitively**, and must match **exactly one place**. `--replace-all` replaces every
  occurrence (it applies to every pair in the command).
- A pair that doesn't match (or matches several places without `--replace-all`) is **skipped** without affecting the
  other pairs; the server returns a warning:

```text
Page updated.
warning: update_content[1]: old_str matches 3 places; give more context or replace_all; skipped
```

Find and replace keeps the cursors of people who have the page open and creates a small history entry. Read the page
with `ttm notes get` first so you copy `--old` exactly (including escapes like `\*` and tab indentation).

### Replace the whole content

```sh
ttm notes get <page> -o page.md
$EDITOR page.md
ttm notes edit <page> --replace -f page.md
```

Keep every `<unknown …/>`, `<truncated …/>`, `<page>`, and `<database>` tag you read: `<unknown>` and `<truncated>` tags
are restored from the original on write; deleting them deletes the corresponding blocks.

### Write rules

| Situation | Result |
| --- | --- |
| The new content drops a child page or child database | `409 would_delete_children`, unless you pass `--allow-deleting-content` |
| The `<truncated/>` line of a long page is dropped | `409 would_delete_truncated`, unless you pass `--allow-deleting-content` |
| More than 1,000 blocks added, changed, or deleted | `413 too_many_blocks`: split the write (deleted blocks don't count with `--allow-deleting-content`) |
| Markdown larger than 2 MB | `413 markdown_too_large` |
| More than 60 writes in an hour (per token) | `429 rate_limited` |
| Nothing changed | `No change applied.` |

On every write through the API, the server **snapshots a version** of the page first and then applies the change as a
block-level diff. Version history records the author as “<you> (via agent <name>)” (the CLI uses the name `ttm-cli`,
MCP uses the client application's name) and offers a button to undo the agent's change.

## Suggest edits (suggestions)

```sh
ttm notes edit <page> --old "ra mắt tháng 11" --new "ra mắt tháng 12" --suggest
```

```text
Suggestion 0190f3d1-… created.
```

A suggestion doesn't change the page; the editor shows it as a tracked change so that someone with edit access can
**accept** or **reject** it, change by change or as a whole.

- When you don't pass `--suggest` (MCP: don't pass `mode`), the token follows the workspace's **agent_write_mode**
  setting: `direct` (default) writes directly, `suggest` turns every agent write into a suggestion.
- A token with only `notee:comment` (no `notee:write`) **always** suggests; forcing `mode: apply` fails with
  `403 insufficient_scope`.
- Suggestions only change content; an MCP call that sends `title` or `properties` with `mode: suggest` is rejected
  (`suggest_content_only`).

View and review:

```sh
ttm notes suggestions <page>          # pending suggestions
ttm notes suggestions <page> --all    # also accepted, rejected, and stale ones
```

```text
0190f3d1-…  pending  An Nguyễn (via claude-code)  1 change(s)
    [c1] replace
      - ra mắt tháng 11
      + ra mắt tháng 12
```

```sh
ttm notes resolve <suggestion> --accept                       # accept every change
ttm notes resolve <suggestion> --accept --change c1 --change c3
ttm notes resolve <suggestion> --reject
```

```text
Suggestion 0190f3d1-…: accepted
Stale (anchor text gone): c3
```

A change whose anchor text is no longer in the page (it was edited after the suggestion) is returned as **stale** and
isn't applied. Reviewing needs `notee:write` and edit access to the page.

## Databases

### View the schema

```sh
ttm notes get <database> --json
```

The schema has `properties` (`id`, `name`, `type`, `options` with the option names of select/multi-select/status,
`config`) and `views`. Property types: `title`, `text`, `number`, `select`, `multi_select`, `status`, `date`, `person`,
`files`, `checkbox`, `url`, `email`, `phone`, `relation`, and the computed types (`formula`, `rollup`, `created_time`,
`last_edited_time`, `created_by`, `last_edited_by`, `unique_id`). Properties that point to a database you can't view are
hidden.

### Query with SQL

```sh
ttm notes query <database> --sql 'SELECT "Name", "Status", "Due" FROM rows WHERE "Status" = '\''Doing'\'' ORDER BY "Due" LIMIT 20'
```

```text
0190f3c5-…  {"Due":"2026-11-15","Name":"Viết tài liệu","Status":"Doing"}
0190f3c6-…  {"Due":"2026-11-20","Name":"Phát hành 0.2","Status":"Doing"}
```

Rules (the same as the view's SQL mode, running on Postgres in read-only mode):

- Exactly **one** `SELECT` statement (`WITH` is allowed at the top level). No `INSERT/UPDATE/DELETE`, no `SELECT INTO`,
  no `FOR UPDATE`, no `$1` parameters, no `LATERAL`, no `WITH` in subqueries.
- The only table is **`rows`** (plus CTEs you define yourself, except one named `rows`), along with the table functions
  `jsonb_array_elements`, `jsonb_array_elements_text`, `jsonb_each`, `jsonb_each_text`, `jsonb_object_keys`, `unnest`.
  No schema-qualified names.
- **Columns = property names**, in double quotes when they contain uppercase letters, spaces, or Vietnamese diacritics:
  `"Trạng thái"`. The system columns `id`, `position`, `created_time`, `last_edited_time`, `created_by`,
  `last_edited_by` are also available (unless a property has the same name).
- Value shapes: select/status is the **option name**; multi-select is a JSON array of option names; dates are ISO 8601
  strings; checkbox is a boolean; formula and rollup are computed values.
- Only pure functions on the allowlist: strings (`lower`, `upper`, `trim`, `concat`, `replace`, `split_part`,
  `regexp_replace`, `to_char`…), numbers (`abs`, `round`, `floor`, `power`…), dates (`now`, `current_date`,
  `date_trunc`, `date_part`, `extract`, `age`, `make_date`…), aggregates (`count`, `sum`, `avg`, `min`, `max`,
  `string_agg`, `array_agg`, `jsonb_agg`, `percentile_cont`…), window functions (`row_number`, `rank`, `lag`, `lead`…),
  JSON and arrays (`jsonb_array_length`, `jsonb_extract_path_text`, `to_jsonb`, `cardinality`…). `pg_*` functions,
  `set_config`, I/O, and sleep are forbidden.
- Each query runs for at most **3 seconds**, each result page is at most **4 MB**, and each user can run at most
  **2 queries at once**.

| Error | Meaning |
| --- | --- |
| `sql_syntax` | Syntax error (the message includes the position) |
| `sql_not_select` | Not exactly one SELECT statement |
| `sql_table` | Reads a table other than `rows` |
| `sql_function` | Function not allowed |
| `sql_forbidden` | Clause not allowed (`SELECT INTO`, `FOR UPDATE`, parameters, a CTE named `rows`…) |
| `sql_error` | Postgres rejected it at run time (type mismatch, division by zero…) |
| `query_timeout` (408) | Took longer than 3 seconds |
| `result_too_large` (413) | A page exceeds 4 MB: select fewer columns or lower `--page-size` |
| `query_busy` (429) | Too many queries at once: retry in a moment |
| `sql_sandbox_unavailable` (503) | The server isn't ready for SQL; use a JSON filter |

### Query with a JSON filter

```sh
ttm notes query <database> --filter filter.json
echo '{"property": "Status", "op": "eq", "value": "Done"}' | ttm notes query <database> --filter -
```

`filter.json` is a bare filter or an object with `filter` and `sorts`:

```json
{
  "filter": {
    "and": [
      { "property": "Status", "op": "in", "value": ["Todo", "Doing"] },
      { "property": "Due", "op": "lte", "value": "2026-11-30" },
      { "or": [
        { "property": "Owner", "op": "is_not_empty" },
        { "property": "Priority", "op": "eq", "value": "High" }
      ] }
    ]
  },
  "sorts": [{ "property": "Due", "direction": "asc" }]
}
```

- Condition: `{"property", "op", "value"}` (`prop` works too, `op` defaults to `eq`); nest with `{"and": [...]}`,
  `{"or": [...]}`.
- Write select, multi-select, and status options by **name**.
- `sorts`: `[{"property", "direction": "asc" | "desc"}]` (default `asc`).

| Operator | Applies to |
| --- | --- |
| `eq`, `neq` | every type (for multi-select, person, relation: contains / doesn't contain the value) |
| `contains`, `not_contains` | text; multi-select, person, relation |
| `starts_with`, `ends_with` | text |
| `gt`, `gte`, `lt`, `lte` | number, date (before/after a date) |
| `in`, `not_in` | select, status (one of / none of); multi-select (`in` = contains any) |
| `contains_all` | multi-select, person, relation |
| `is_empty`, `is_not_empty` | every type |
| `between` | number, date |
| `is_relative` | date: `today`, `this_week`… |

A property that doesn't exist: `400 bad_request` (`rule: unknown_prop`); a malformed filter: `rule: filter_shape`.

### Pagination

```sh
ttm notes query <database> --sql 'SELECT * FROM rows' --page-size 50
# stderr: more rows: --cursor eyJvIjo1MH0
ttm notes query <database> --sql 'SELECT * FROM rows' --page-size 50 --cursor eyJvIjo1MH0
```

A page has 1–100 rows (default 100). The JSON result has `columns`, `rows` (`id`, `values` by column name), and
`next_cursor` (`null` on the last page). With SQL, the `id` column is empty if the `SELECT` doesn't select `id`.

Over MCP, pass exactly **one** of `sql` or `filter` to `notee-query-database`.

### Write and delete rows

`rows.json`:

```json
[
  { "values": { "Name": "Viết changelog", "Status": "Todo", "Due": "2026-11-20" } },
  { "id": "0190f3c5-…", "values": { "Status": "Done" } }
]
```

```sh
ttm notes rows upsert <database> -f rows.json
ttm notes rows delete <database> 0190f3c5-… 0190f3c6-…
```

```text
2 row(s) saved: 0190f3c7-…, 0190f3c5-…
```

- Without an `id`, a new row is created; with an `id`, only the given columns change.
- 1–100 rows per call. Values are keyed by **property name**, options by **name**.
- Deleting a row moves it to the trash. Editing a row that is already in the trash: `409 row_deleted`; an id that
  belongs to another database: `row_elsewhere`.
- Writing rows through SQL isn't allowed.

## Comments

```sh
ttm notes comments <page>          # open threads
ttm notes comments <page> --all    # also resolved threads
```

```text
Thread 0190f3e0-… on "ra mắt tháng 12"
  An Nguyễn: Chốt ngày cụ thể chưa?
  Bình: Ngày 15.
```

Write a comment:

```sh
ttm notes comment <page> -m "Cần thêm số liệu"                                  # comment on the whole page
ttm notes comment <page> --quote "ra mắt tháng 12" -m "Chốt ngày cụ thể?"       # anchor to quoted text
ttm notes comment <page> --quote "9h" --suffix " sáng thứ Hai" -m "Đổi sang 10h"
ttm notes comment <page> --thread 0190f3e0-… -m "Đồng ý"                        # reply to a thread
```

- Comments anchor to a **text quote** (W3C `TextQuoteSelector` style): `exact` (1–1,000 characters) must match exactly
  one place in the page.
- Not found: `422 anchor_not_found`. Several matches: `422 ambiguous_anchor`; add a few words right before (`--prefix`)
  or right after (`--suffix`).
- Anchors survive changes to the surrounding text. Mentioned people and the page's followers get notified.
- `--thread` can't be combined with `--quote`.

MCP `notee-create-comment` takes `page`, `text`, `quote` (`{exact, prefix?, suffix?}`) or `thread_id`.

## Examples for agents

Prompts to try with Claude once MCP is installed:

- *Find the page “Họp tuần”, read it, then add a “Việc tuần sau” section at the end of the page with update_content.*
- *Suggest edits (mode suggest) that change every “quý 3” to “quý 4” in page X; don't write directly.*
- *In the “Việc nhóm” database, list the overdue tasks with status Doing using SQL, then comment on each page to remind
  the owner.*
