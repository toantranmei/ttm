---
title: Notee
description: 通过 CLI 或 MCP 搜索、读取、创建和修改 Notee 页面，提出修改建议，查询和写入数据库，以及按引用片段锚定评论。
order: 7
section: guides
---

下面的所有操作都可以通过 **CLI**（`ttm notes …`）或 **MCP**（`notee-*` 工具）完成。两者调用同一套 API，因此规则、
限制和错误都相同。页面以 [Notee-flavored Markdown](notee-markdown.md) 格式读写。

| 操作 | CLI | MCP 工具 | Scope |
| --- | --- | --- | --- |
| 搜索页面 | `ttm notes search` | `notee-search` | `notee:read` |
| 读取页面或数据库 schema | `ttm notes get` | `notee-fetch` | `notee:read` |
| 创建页面 | `ttm notes create` | `notee-create-pages` | `notee:write` |
| 修改页面 | `ttm notes edit` | `notee-update-page` | `notee:write`（或用 `notee:comment` 提出建议） |
| 修改建议 | `ttm notes edit --suggest`、`notes suggestions`、`notes resolve` | `notee-update-page`（`mode: suggest`）、`notee-get-suggestions`、`notee-resolve-suggestion` | `notee:comment`，审批需要 `notee:write` |
| 查询数据库 | `ttm notes query` | `notee-query-database` | `notee:read` |
| 写入、删除行 | `ttm notes rows upsert/delete` | `notee-upsert-rows`、`notee-delete-rows` | `notee:write` |
| 评论 | `ttm notes comments`、`ttm notes comment` | `notee-get-comments`、`notee-create-comment` | `notee:read`、`notee:comment` |
| 移动、复制、创建/修改数据库 | （暂无命令） | `notee-move-pages`、`notee-duplicate-page`、`notee-create-database`、`notee-update-database` | `notee:write` |

## Id 与 URL

所有页面/数据库参数都接受**id**（UUID）或它的**任意 URL**：`ttm` 取路径中的最后一个 UUID，即使它位于 slug 的
末尾。

```sh
ttm notes get 0190f3b0-1111-7aaa-8bbb-0123456789ab
ttm notes get https://toantranmei.com/notes/0190f3b0-1111-7aaa-8bbb-0123456789ab
ttm notes get "https://toantranmei.com/notes/ke-hoach-quy-4-0190f3b0-1111-7aaa-8bbb-0123456789ab?x=1"
```

工作区则始终使用 id（见 `ttm workspace list`）。

## 搜索

```sh
ttm notes search "ke hoach" --limit 5
ttm notes search "OKR" --workspace 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

- 在你可查看的页面的标题和内容中搜索，**不区分变音符号和大小写**；所有词都必须出现。
- 标题匹配的排在前面，其次是最近修改的页面。服务器最多返回 20 条结果；`--limit`（默认 10）会进一步截取。
- 默认在 profile 的当前工作区中搜索；尚未选择工作区时搜索所有工作区。
- 搜索语句最多 8 个词、200 个字符（`400 bad_request: query too long`）。

## 读取

```sh
ttm notes get <page>                    # Markdown 输出到 stdout
ttm notes get <page> -o page.md         # 写入文件，打印 "Saved <tiêu đề> (page.md)."
ttm notes get <page> --json             # 额外包含 id、title、icon、url、parent、properties、truncated、unknown_block_ids
ttm notes get <database>                # 标题和 property 列表
```

```text
# Việc nhóm
- Name (title)
- Status (status)
- Due (date)
- Owner (person)
```

- Id 是**数据库行**时：`get` 返回该行的页面内容；各列的值位于 `properties` 中（仅在 `--json` 下提供）。
- **长页面**（超过 20,000 个节点）会被截断：Markdown 的最后一行是 `<truncated url="…" blocks="n"/>`，结果中带有
  `truncated: true` 和 `unknown_block_ids`。CLI 会在 stderr 上提示：
  `note: page truncated; read the rest with --block <id>: b1, b2`。用 `--block <id>` 分段读取（MCP：`block`）。
- Markdown 无法表示的 block（sketch、embed、synced block…）显示为 `<unknown url="…" alt="…"/>`；它们的 id
  也包含在 `unknown_block_ids` 中。

在 MCP 中，`notee-fetch` 为模型返回包裹在 `<page id="…" title="…" url="…">…</page>` 中的 Markdown，并为客户端返回
结构化数据（`kind: "page"` 或 `kind: "database"`）。

## 创建页面

```sh
ttm notes create --parent <page> -f ke-hoach.md
ttm notes create --workspace <workspace-id> -f note.md --title "Ghi chú họp" --icon "📝"
ttm notes create --database <database> -f task.md --title "Viết tài liệu"     # 创建一行
cat note.md | ttm notes create --parent <page> -f -
```

```text
Created Ghi chú họp (https://toantranmei.com/notes/0190f3c4-…)
```

- 只能选择**一个**放置位置：`--parent`（父页面）、`--database`（成为数据库的一行）或 `--workspace`（工作区的
  顶层）。
- 标题：取 `--title`；没有则取文件中第一行 `# `；再没有则取文件名（去掉扩展名）。该 `# ` 行仍保留在内容中；
  如果不想标题重复出现，请从文件中删除它。
- 每个页面最多 1,000 个 block、2 MB Markdown。

MCP `notee-create-pages` 一次创建 1–20 个页面（总计 ≤ 1,000 个 block），每个页面包含 `title`、`icon`、`content`（Markdown）和
`properties`（父级为数据库时，按列名给出的列值）：

```json
{
  "parent": { "database_id": "0190f3c0-…" },
  "pages": [
    { "title": "Viết tài liệu", "properties": { "Status": "Doing", "Due": "2026-11-15" } },
    { "title": "Phát hành 0.2", "icon": "🚀", "content": "## Việc cần làm\n- [ ] Changelog" }
  ]
}
```

## 修改页面

### 查找替换（推荐）

```sh
ttm notes edit <page> --old "Q3" --new "Q4"
ttm notes edit <page> --old "Trạng thái: nháp" --new "Trạng thái: duyệt" --old "v0.1" --new "v0.2"
ttm notes edit <page> --old "TODO" --new "DONE" --replace-all
```

- 每个 `--old` 与相同位置的 `--new` 配对；两者数量必须相等。各对**按顺序**应用到当前 Markdown 上。
- `--old` 按原文精确匹配，**区分大小写**，并且必须**恰好匹配一处**。`--replace-all` 替换所有匹配处（作用于该命令的
  所有替换对）。
- 不匹配的替换对（或在没有 `--replace-all` 时匹配多处的）会被**跳过**，不影响其他替换对；服务器返回
  警告：

```text
Page updated.
warning: update_content[1]: old_str matches 3 places; give more context or replace_all; skipped
```

查找替换会保持正在打开该页面的人的光标位置，并生成一条较小的历史记录。请先用 `ttm notes get` 读取页面，以便
准确复制 `--old`（包括 `\*` 之类的转义字符和缩进用的 tab）。

### 替换全部内容

```sh
ttm notes get <page> -o page.md
$EDITOR page.md
ttm notes edit <page> --replace -f page.md
```

保留你读取到的所有 `<unknown …/>`、`<truncated …/>`、`<page>` 和 `<database>` 标签：写入时 `<unknown>` 和
`<truncated>` 标签会从原文中恢复；删除它们就是删除对应的 block。

### 写入规则

| 情况 | 结果 |
| --- | --- |
| 新内容丢掉了子页面或子数据库 | `409 would_delete_children`，除非带有 `--allow-deleting-content` |
| 去掉了长页面的 `<truncated/>` 行 | `409 would_delete_truncated`，除非带有 `--allow-deleting-content` |
| 新增、修改或删除超过 1,000 个 block | `413 too_many_blocks`：拆分写入（带 `--allow-deleting-content` 时删除的 block 不计入） |
| Markdown 超过 2 MB | `413 markdown_too_large` |
| 一小时内写入超过 60 次（按 token 计） | `429 rate_limited` |
| 没有任何变化 | `No change applied.` |

每次通过 API 写入时，服务器会先**保存页面的一个版本**，再以按 block diff 的方式应用修改。版本历史中记录的作者为
“<bạn> (via agent <tên>)”（CLI 使用名称 `ttm-cli`，MCP 使用客户端应用的名称），并提供撤销 agent 修改的
按钮。

## 修改建议（suggestion）

```sh
ttm notes edit <page> --old "ra mắt tháng 11" --new "ra mắt tháng 12" --suggest
```

```text
Suggestion 0190f3d1-… created.
```

修改建议不会改动页面；编辑器会将其显示为 tracked change，供有编辑权限的人逐项或整体**接受**或**拒绝**。

- 未传 `--suggest`（MCP：未传 `mode`）时，token 遵循工作区的 **agent_write_mode** 设置：
  `direct`（默认）直接写入，`suggest` 则 agent 的所有写入都变成修改建议。
- 只有 `notee:comment`（没有 `notee:write`）的 token **总是**提出修改建议；强制 `mode: apply` 会得到 `403 insufficient_scope`。
- 修改建议只能修改内容；MCP 在 `mode: suggest` 中附带 `title` 或 `properties` 会被拒绝（`suggest_content_only`）。

查看与审批：

```sh
ttm notes suggestions <page>          # 待处理的修改建议
ttm notes suggestions <page> --all    # 包括已接受、已拒绝、已过时的
```

```text
0190f3d1-…  pending  An Nguyễn (via claude-code)  1 change(s)
    [c1] replace
      - ra mắt tháng 11
      + ra mắt tháng 12
```

```sh
ttm notes resolve <suggestion> --accept                       # 接受所有修改
ttm notes resolve <suggestion> --accept --change c1 --change c3
ttm notes resolve <suggestion> --reject
```

```text
Suggestion 0190f3d1-…: accepted
Stale (anchor text gone): c3
```

锚点片段已不在页面中（提出建议后又被修改）的修改会以 **stale** 返回，且不会被应用。审批需要
`notee:write` 和页面的编辑权限。

## 数据库

### 查看 schema

```sh
ttm notes get <database> --json
```

Schema 包含 `properties`（`id`、`name`、`type`、`options` 即 select/multi-select/status 的选项名、`config`）和
`views`。Property 类型：`title`、`text`、`number`、`select`、`multi_select`、`status`、`date`、`person`、`files`、
`checkbox`、`url`、`email`、`phone`、`relation`，以及计算类型（`formula`、`rollup`、`created_time`、`last_edited_time`、
`created_by`、`last_edited_by`、`unique_id`）。指向你无权查看的数据库的 property 会被隐藏。

### 用 SQL 查询

```sh
ttm notes query <database> --sql 'SELECT "Name", "Status", "Due" FROM rows WHERE "Status" = '\''Doing'\'' ORDER BY "Due" LIMIT 20'
```

```text
0190f3c5-…  {"Due":"2026-11-15","Name":"Viết tài liệu","Status":"Doing"}
0190f3c6-…  {"Due":"2026-11-20","Name":"Phát hành 0.2","Status":"Doing"}
```

规则（与视图的 SQL 模式相同，在只读模式的 Postgres 上运行）：

- 恰好**一条** `SELECT` 语句（允许最外层使用 `WITH`）。不允许 `INSERT/UPDATE/DELETE`、`SELECT INTO`、
  `FOR UPDATE`、参数 `$1`、`LATERAL`，也不允许在子查询中使用 `WITH`。
- 唯一的表是 **`rows`**（以及你自己定义的 CTE，名称不能为 `rows`），外加表函数 `jsonb_array_elements`、
  `jsonb_array_elements_text`、`jsonb_each`、`jsonb_each_text`、`jsonb_object_keys`、`unnest`。不允许带 schema 的名称。
- **列 = property 名称**，含大写字母、空格或越南语变音符号时需用双引号括起来：`"Trạng thái"`。另有系统列
  `id`、`position`、`created_time`、`last_edited_time`、`created_by`、`last_edited_by`（除非有同名的
  property）。
- 值的形式：select/status 为**选项名**；multi-select 为选项名组成的 JSON 数组；日期为 ISO 8601 字符串；
  checkbox 为 boolean；formula 和 rollup 为计算后的值。
- 只能使用白名单中的纯函数：字符串（`lower`、`upper`、`trim`、`concat`、`replace`、`split_part`、
  `regexp_replace`、`to_char`…）、数值（`abs`、`round`、`floor`、`power`…）、日期（`now`、`current_date`、`date_trunc`、
  `date_part`、`extract`、`age`、`make_date`…）、聚合（`count`、`sum`、`avg`、`min`、`max`、`string_agg`、
  `array_agg`、`jsonb_agg`、`percentile_cont`…）、窗口（`row_number`、`rank`、`lag`、`lead`…）、JSON 与数组
  （`jsonb_array_length`、`jsonb_extract_path_text`、`to_jsonb`、`cardinality`…）。禁止 `pg_*`、`set_config`、I/O、sleep
  类函数。
- 每个查询最多运行 **3 秒**，每页结果最多 **4 MB**，每人最多**同时运行 2 个查询**。

| 错误 | 含义 |
| --- | --- |
| `sql_syntax` | 语法错误（提示信息附带位置） |
| `sql_not_select` | 不是恰好一条 SELECT 语句 |
| `sql_table` | 读取了 `rows` 以外的表 |
| `sql_function` | 使用了不允许的函数 |
| `sql_forbidden` | 使用了不允许的子句（`SELECT INTO`、`FOR UPDATE`、参数、名为 `rows` 的 CTE…） |
| `sql_error` | Postgres 执行时拒绝（类型错误、除以 0…） |
| `query_timeout` (408) | 超过 3 秒 |
| `result_too_large` (413) | 单页超过 4 MB：少选几列或减小 `--page-size` |
| `query_busy` (429) | 同时运行的查询过多：稍等片刻再试 |
| `sql_sandbox_unavailable` (503) | 服务器尚未准备好 SQL；请使用 JSON 过滤器 |

### 用 JSON 过滤器查询

```sh
ttm notes query <database> --filter filter.json
echo '{"property": "Status", "op": "eq", "value": "Done"}' | ttm notes query <database> --filter -
```

`filter.json` 是一个单独的过滤条件，或包含 `filter` 和 `sorts` 的对象：

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

- 条件：`{"property", "op", "value"}`（也可以用 `prop`，`op` 默认为 `eq`）；用 `{"and": [...]}`、
  `{"or": [...]}` 嵌套。
- select、multi-select、status 的选项用**名称**表示。
- `sorts`：`[{"property", "direction": "asc" | "desc"}]`（默认 `asc`）。

| 运算符 | 适用于 |
| --- | --- |
| `eq`、`neq` | 所有类型（对 multi-select、person、relation：包含 / 不包含该值） |
| `contains`、`not_contains` | text；multi-select、person、relation |
| `starts_with`、`ends_with` | text |
| `gt`、`gte`、`lt`、`lte` | number、date（早于/晚于某日期） |
| `in`、`not_in` | select、status（属于其中之一 / 不属于）；multi-select（`in` = 包含任意一个） |
| `contains_all` | multi-select、person、relation |
| `is_empty`、`is_not_empty` | 所有类型 |
| `between` | number、date |
| `is_relative` | date：`today`、`this_week`… |

Property 不存在：`400 bad_request`（`rule: unknown_prop`）；过滤器格式错误：`rule: filter_shape`。

### 分页

```sh
ttm notes query <database> --sql 'SELECT * FROM rows' --page-size 50
# stderr: more rows: --cursor eyJvIjo1MH0
ttm notes query <database> --sql 'SELECT * FROM rows' --page-size 50 --cursor eyJvIjo1MH0
```

每页 1–100 行（默认 100）。JSON 结果：`columns`、`rows`（`id`、按列名给出的 `values`）、`next_cursor`（最后一页
为 `null`）。使用 SQL 时，如果 `SELECT` 语句没有选择 `id`，`id` 列为空。

在 MCP 中，向 `notee-query-database` 传入 `sql` 或 `filter` 中的**恰好一个**。

### 写入和删除行

`rows.json`：

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

- 没有 `id` 时创建新行；有 `id` 时只修改列出的列。
- 每次 1–100 行。值按 **property 名称**给出，选项按**名称**给出。
- 删除的行会移到回收站。修改已在回收站中的行：`409 row_deleted`；id 属于其他数据库：`row_elsewhere`。
- 不允许通过 SQL 写入行。

## 评论

```sh
ttm notes comments <page>          # 未关闭的 thread
ttm notes comments <page> --all    # 包括已关闭的 thread
```

```text
Thread 0190f3e0-… on "ra mắt tháng 12"
  An Nguyễn: Chốt ngày cụ thể chưa?
  Bình: Ngày 15.
```

发表评论：

```sh
ttm notes comment <page> -m "Cần thêm số liệu"                                  # 针对整个页面的评论
ttm notes comment <page> --quote "ra mắt tháng 12" -m "Chốt ngày cụ thể?"       # 锚定到引用片段
ttm notes comment <page> --quote "9h" --suffix " sáng thứ Hai" -m "Đổi sang 10h"
ttm notes comment <page> --thread 0190f3e0-… -m "Đồng ý"                        # 回复 thread
```

- 按**文本片段**锚定（W3C `TextQuoteSelector` 风格）：`exact`（1–1,000 个字符）必须在页面中恰好匹配
  一处。
- 找不到：`422 anchor_not_found`。匹配多处：`422 ambiguous_anchor`；在紧邻其前（`--prefix`）或紧邻其后
  （`--suffix`）补充几个字。
- 周围文本变化时锚点依然有效。被提及的人和关注该页面的人会收到通知。
- `--thread` 不能与 `--quote` 同时使用。

MCP `notee-create-comment` 接受 `page`、`text`、`quote`（`{exact, prefix?, suffix?}`）或 `thread_id`。

## 给 agent 的示例

安装 MCP 后可以这样向 Claude 提问：

- *搜索页面“周会”，读取它，然后用 update_content 在页面末尾添加一节“下周事项”。*
- *在页面 X 中，把所有写着“第 3 季度”的地方以修改建议（mode suggest）的方式改为“第 4 季度”，不要直接写入。*
- *在数据库“团队事项”中，用 SQL 列出状态为 Doing 且已逾期的事项，然后在每个页面上发表评论提醒
  负责人。*
