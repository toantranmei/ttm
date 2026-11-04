---
title: MCP 工具
description: ttm-mcp 与远程 MCP 的全部工具：用途、参数、调用示例与结果、所需 scope、错误。
order: 11
section: reference
---

本页同时适用于 `ttm-mcp`（stdio）和远程 MCP `https://toantranmei.com/api/v1/mcp`：工具名、参数和结果都相同。
版本 **0.1.4**：3 个组共 26 个工具，以及 1 个 resource。

## 约定

- 参数是 JSON 对象；**未知参数会被拒绝**。参数为 `null` 或省略时使用默认值。
- 所有页面/数据库参数都接受其 id 或它的任意 URL。
- 每个工具都返回 **文本内容**（给模型看的 Markdown，或 JSON）以及 **`structuredContent`**（JSON）。
- 注解（annotation）：读取类工具带 `readOnlyHint: true`、`idempotentHint: true`；写入类工具带 `readOnlyHint: false`；
  可能删除或覆盖数据的工具带 `destructiveHint: true`。没有任何工具会访问 toantranmei.com 以外的地方
  （`openWorldHint: false`）。

### 错误

| 形式 | 何时出现 | 模型看到什么 |
| --- | --- | --- |
| 协议错误 `invalid_params`（-32602） | 未知工具名；参数类型错误、缺失或未知；本地校验失败（例如 `give 1 to 20 pages`） | JSON-RPC 错误消息 |
| 结果带 `isError: true` | 服务器拒绝或出错 | 文本 `<消息> (<code>)`，以及 `structuredContent: {"error": {"code", "message", "status", "rule", "field", "params"}}` |

`code` 是服务器的错误码（`unauthorized`、`invalid_token`、`insufficient_scope`、`not_found`、`too_many_blocks`…，见
[错误码](ma-loi.md)），或工具自身的错误码：

| 错误码 | 含义 |
| --- | --- |
| `no_run` | 该工具需要一次 agent 运行（未设置 `TTM_RUN_ID`） |
| `no_task` | 未传 `task`，也没有当前任务（`TTM_TASK`） |
| `no_project` | 未传 `project`，也没有 `TTM_PROJECT` |
| `unsupported` | 该服务器不支持此工具 |
| `network`、`decode` | 无法调用 API / 响应无法识别 |

服务器错误在服务器提供时会保留 `status`、`rule`（未通过的检查）、`field`（输入字段）和 `params`（相关的值）；文本中也会
写出 `rule` 和 `field`。agent 应按 `rule` 分支，而不是解析文字：`split-limit`（拆得更少或更浅，`params` 中有上限），
`rev-conflict`（重新读取任务后重试），`approval-required`（用 `factory-request-approval` 申请批准）；`insufficient_scope`
的 `params.required` 是缺少的 scope。

401 错误会附带提示 ``Sign in again with `ttm login`.``

## Resource

| URI | 名称 | 类型 | 内容 |
| --- | --- | --- | --- |
| `notee://spec/markdown` | `notee-markdown` | `text/markdown` | Notee-flavored Markdown 规范（启用 `notee` 组时提供） |

---

## workspace 组

始终启用。Scope：`workspace:read`。

### workspace-get-self

当前登录的用户（id、名称、邮箱、套餐）和当前工作区。建议第一个调用。

参数：无。

```json
{
  "user": { "id": "0190f3a1-…", "email": "an@example.com", "name": "An Nguyễn", "avatar_url": null, "locale": "vi", "plan": "free" },
  "workspace": { "id": "0190f3a2-…", "name": "Team Aurora", "kind": "team", "role": "member", "slug": "aurora" }
}
```

当前工作区 = profile 的工作区（`ttm workspace switch`），或 token 被限定的工作区（远程 MCP），否则为个人工作区。

### workspace-list

用户所属的工作区，包括类型（`personal`/`team`）和角色（`owner`、`admin`、`member`、`guest`）。

参数：无。结果：`{"workspaces": [{id, name, kind, role, slug?}]}`。

### workspace-get-users

某个工作区的成员（人）及其角色，用于查找要提及或分配工作的人。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `workspace_id` | string | | 默认为当前工作区 |
| `query` | string | | 按名称或邮箱过滤，不区分大小写 |

```json
{ "query": "bình" }
```

```json
{ "users": [{ "user": { "id": "0190…", "name": "Bình", "email": "binh@example.com", "avatar_url": null }, "role": "member" }] }
```

错误：`forbidden`（访客无法查看成员列表）。

---

## notee 组

### notee-search

按标题和正文搜索你可查看的页面（不区分声调符号）。Scope `notee:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `query` | string | 是 | 要搜索的词，所有词都必须出现；≤ 8 个词，≤ 200 个字符 |
| `limit` | integer | | 1–20，默认 10 |
| `workspace_id` | string | | 默认为当前工作区 |

```json
{ "query": "ke hoach quy 4", "limit": 5 }
```

文本：`- Kế hoạch quý 4 (id: 0190f3b0-…): …đoạn trích…`。结构化：
`{"results": [{id, workspace_id, title, icon, snippet}]}`（仅标题匹配时 `snippet` 为空）。

错误：`query` 为空时返回 `invalid_params`；查询过长时返回 `bad_request`。

### notee-fetch

以 [Notee-flavored Markdown](notee-markdown.md) 读取页面，或读取数据库的 property 和视图。Scope `notee:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | string | 是 | 页面、数据库行或数据库的 id 或 URL |
| `block` | string | | 被截断页面的 `unknown_block_ids` 中的某个 id，用于读取该部分 |

页面的文本：

```text
<page id="0190f3b0-…" title="Kế hoạch quý 4" url="https://toantranmei.com/notes/0190f3b0-…">
# Mục tiêu
- Phát hành ttm 0.2
</page>
```

被截断的页面会多一行 ``The page was truncated. Read the missing parts with notee-fetch and `block` set to one of: …``。
数据库行会多出 `<properties>{…}</properties>`。结构化：`{"kind": "page", id, title, icon, url, parent,
properties?, markdown, truncated, unknown_block_ids}` 或 `{"kind": "database", id, title, url, properties[], views[]}`。

数据库的文本是一张 `| Property | Type |` 表格。

错误：`not_found`（不存在或无权查看）。

### notee-query-database

用只读 SQL 或 JSON 过滤器查询数据库的行。Scope `notee:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `database` | string | 是 | id 或 URL |
| `sql` | string | 二选一 | `SELECT … FROM rows …`，列名即 property 名 |
| `filter` | object | 二选一 | `{"and": [{"property", "op", "value"}]}` |
| `sorts` | array | | 与 `filter` 搭配：`[{"property": "Due", "direction": "asc"}]` |
| `page_size` | integer | | 1–100（默认 100） |
| `cursor` | string | | 上一页的 `next_cursor` |

```json
{ "database": "0190f3c0-…", "sql": "SELECT \"Name\", \"Status\" FROM rows WHERE \"Status\" = 'Done' ORDER BY \"Due\" LIMIT 20" }
```

```json
{ "columns": ["Name", "Status"], "rows": [{ "id": "", "values": { "Name": "Viết tài liệu", "Status": "Done" } }], "next_cursor": null }
```

`filter` 的运算符：`eq`、`neq`、`contains`、`not_contains`、`starts_with`、`ends_with`、`gt`、`gte`、`lt`、`lte`、
`in`、`not_in`、`contains_all`、`is_empty`、`is_not_empty`、`between`、`is_relative`。SQL 和过滤器规则：[Notee](notee.md)。

错误：两者都缺或两者都有时返回 `invalid_params`；`sql_syntax`、`sql_not_select`、`sql_table`、`sql_function`、
`sql_forbidden`、`sql_error`、`bad_request`（`unknown_prop`、`filter_shape`、`invalid_cursor`）、`query_timeout`、
`result_too_large`、`query_busy`、`sql_sandbox_unavailable`。

### notee-get-comments

页面的评论 thread，以及每个 thread 锚定的摘录。Scope `notee:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `page` | string | 是 | id 或 URL |
| `include_resolved` | boolean | | 包含已关闭的 thread（默认 `false`） |

```json
{ "threads": [{ "id": "0190f3e0-…", "anchor": { "kind": "text", "block_id": "b7", "quote": "ra mắt tháng 12" },
  "quote": { "exact": "ra mắt tháng 12", "prefix": null, "suffix": null }, "resolved": false,
  "comments": [{ "id": "…", "author": "An Nguyễn", "text": "Chốt ngày cụ thể chưa?", "created_at": "2026-11-04T05:00:00Z" }] }] }
```

### notee-create-pages

在某个页面下、作为数据库的行、或在工作区顶层创建 1–20 个页面。Scope `notee:write`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `parent` | object | 是 | `page_id`、`database_id`、`workspace_id` 中 **恰好一个** |
| `pages` | array | 是 | 1–20 个页面，合计 ≤ 1,000 个 block |
| `pages[].title` | string | 是 | 标题 |
| `pages[].icon` | string | | Emoji |
| `pages[].content` | string | | Markdown 内容 |
| `pages[].properties` | object | | 父级为数据库时，按 property 名给出的列值 |

```json
{ "parent": { "page_id": "0190f3b0-…" }, "pages": [{ "title": "Biên bản 04/11", "icon": "📝", "content": "## Quyết định\n- Phát hành thứ Sáu" }] }
```

```json
{ "pages": [{ "id": "0190f3c4-…", "title": "Biên bản 04/11", "url": "https://toantranmei.com/notes/0190f3c4-…" }] }
```

错误：`invalid_params`（`give 1 to 20 pages`、`parent needs exactly one of page_id, database_id, workspace_id`）、
`too_many_blocks`、`markdown_too_large`、`rate_limited`、`forbidden`。

### notee-update-page

编辑页面：查找替换（`update_content`）或替换全部内容（`replace_content`）；也可修改标题以及数据库行的列值。
`mode: "suggest"` 会创建修改建议而不是直接写入。Scope `notee:comment`（建议）或 `notee:write`（写入）。请先用
`notee-fetch` 读取页面。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `page` | string | 是 | id 或 URL |
| `update_content` | array | | `[{old_str, new_str, replace_all?}]`；`old_str` 精确匹配、区分大小写，除非 `replace_all`，否则必须恰好匹配一处 |
| `replace_content` | string | | 全部新内容；保留读到的所有 `<unknown/>` 标签 |
| `allow_deleting_content` | boolean | | 允许去掉子页面、子数据库、`<truncated/>` 部分（默认 `false`） |
| `title` | string | | 新标题 |
| `properties` | object | | 要修改的列值（数据库行） |
| `mode` | `"apply"` \| `"suggest"` | | 默认取工作区设置（`agent_write_mode`）；token 缺少 `notee:write` 时始终为 `suggest` |

至少需要一项修改；`update_content` 不能与 `replace_content` 同时使用。修改建议只能修改内容。

```json
{ "page": "0190f3b0-…", "update_content": [{ "old_str": "ra mắt tháng 11", "new_str": "ra mắt tháng 12" }], "mode": "suggest" }
```

文本：`Suggestion <id> created; people with edit access can accept or reject it.`、`Page updated.` 或
`No change applied.`，并附带 `Warning: update_content[0]: old_str not found; skipped`。结构化：
`{applied, warnings, seq, snapshot_seq, suggestion_id?}`。

错误：`invalid_params`（`nothing to change`，两种方式同时使用）、`would_delete_children`、`would_delete_truncated`、
`too_many_blocks`、`markdown_too_large`、`insufficient_scope`（缺少 `notee:write` 时使用 `mode: apply`）、
`bad_request`（`suggest_content_only`、`one_of`）、`rate_limited`、`forbidden`。

### notee-move-pages

把页面移到另一个页面下、移入数据库或移到工作区顶层。Scope `notee:write`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `pages` | array of string | 是 | 各页面的 id 或 URL |
| `parent` | object | 是 | `page_id`、`database_id`、`workspace_id` 中恰好一个 |

结果：`{"pages": [{id, title, url}]}`。页面依次移动；哪个页面出错就停在那里。

### notee-duplicate-page

复制一个页面及其内容。Scope `notee:write`。

| 参数 | 类型 | 必填 |
| --- | --- | --- |
| `page` | string | 是 |

结果：副本的 `{id, title, url}`。

### notee-create-database

创建数据库。Scope `notee:write`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `parent` | object | 是 | `page_id`、`database_id`、`workspace_id` 中恰好一个 |
| `title` | string | 是 | |
| `properties` | array | | `[{"name": "Status", "type": "select", "options": [{"name": "Todo"}, {"name": "Done"}]}]`；类型：`title`、`text`、`number`、`select`、`multi_select`、`status`、`date`、`person`、`checkbox`、`url`、`email`、`phone`、`relation` |

结果：`{id, title, url}`。

### notee-update-database

添加、修改、删除 property 和视图，重命名数据库。Scope `notee:write`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `database` | string | 是 | id 或 URL |
| `changes` | object | 是 | `{"add_properties": [...], "update_properties": [...], "remove_properties": ["Name"], "views": [...], "title": "…"}` |

结果：服务器的响应（JSON）。

### notee-upsert-rows

创建或更新 1–100 行。Scope `notee:write`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `database` | string | 是 | id 或 URL |
| `rows` | array | 是 | 1–100 个 `{id?, values}` 元素 |
| `rows[].id` | string | | 有：更新该行；无：创建新行 |
| `rows[].values` | object | 是 | 按 property 名给出的值；选项按名称指定 |

```json
{ "database": "0190f3c0-…", "rows": [{ "values": { "Name": "Viết changelog", "Status": "Todo" } }] }
```

结果：`{"ids": ["0190f3c7-…"]}`。错误：`invalid_params`（`give 1 to 100 rows`）、`row_deleted`、`bad_request`
（`row_elsewhere`）。

### notee-delete-rows

删除行（移入回收站）。Scope `notee:write`。`destructiveHint: true`。

| 参数 | 类型 | 必填 |
| --- | --- | --- |
| `database` | string | 是 |
| `ids` | array of string | 是（不能为空） |

### notee-create-comment

锚定在页面某段摘录上的评论、针对整个页面的评论，或回复某个 thread。Scope `notee:comment`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `page` | string | 是 | id 或 URL |
| `text` | string | 是 | 内容（不能为空） |
| `quote` | object | | `{exact, prefix?, suffix?}`；省略 = 针对整个页面的评论 |
| `quote.exact` | string | 是（在 `quote` 中） | 与页面中完全一致的文字，1–1,000 个字符 |
| `quote.prefix`、`quote.suffix` | string | | 紧挨在前 / 后的几个字，用于 `exact` 出现多次时 |
| `thread_id` | string | | 回复这个 thread，而不是新开 thread |

```json
{ "page": "0190f3b0-…", "text": "Nguồn số liệu?", "quote": { "exact": "tăng 30%", "suffix": " so với quý trước" } }
```

错误：`anchor_not_found`、`ambiguous_anchor`（请加上 `prefix`/`suffix`）、`bad_request`（`quote_length`）。

### notee-get-suggestions

页面上的修改建议：作者、agent、每处修改及其锚点和修改前/后的 Markdown。Scope `notee:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `page` | string | 是 | id 或 URL |
| `include_resolved` | boolean | | 包含已接受、已拒绝、已过时的建议（默认只返回 `pending`） |

每条建议：`id`、`status`（`pending`、`accepted`、`rejected`、`stale`）、`author`、`via_agent`、`created_at`、`changes[]`
（`id`、`kind` = `insert`/`delete`/`replace`、`anchor`、`before_nfm`、`after_nfm`）。

### notee-resolve-suggestion

接受（全部或部分修改）或拒绝一条修改建议。需要页面编辑权限。Scope `notee:write`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `suggestion` | string | 是 | 建议的 id |
| `decision` | `"accept"` \| `"reject"` | 是 | |
| `change_ids` | array of string | | 只接受这些修改（拒绝时忽略） |

结果：`{id, status, stale_change_ids}`；锚点已丢失的修改为 `stale`，不会被应用。

---

## factory 组

供 runner **一次运行中** 的 agent 使用（变量 `TTM_RUN_ID`、`TTM_TASK`、`TTM_PROJECT`，见
[连接 MCP](mcp.md)）。作业（job）的 token 带有 `factory:run`（包含 `factory:read`）。

### factory-get-task

当前进行中的任务（或按 key 指定的其他任务）：标题、状态、角色、负责人、父任务。Scope `factory:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `task` | string | | key（`AUR-12`）或 id；默认为本次 run 的任务 |

结果：任务对象（camelCase）：`id`（key）、`key`、`uuid`、`title`、`state`、`category`、`role`、`assignee`、
`assigneeMember{kind, id, name}`、`parentId`、`estimate`、`description`、`rev`… 错误：`no_task`、`not_found`。

### factory-get-context

服务器为任务组装好的 context：描述、验收标准、相关的 Notee 页面、兄弟任务、建议文件。建议在开始前阅读。
Scope `factory:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `task` | string | | 默认为本次 run 的任务 |

在 run 中不传 `task`：返回 run 的 context；否则：返回任务的 context。服务器返回 `markdown` 时，模型收到的就是该
Markdown。

### factory-report-progress

向 Execution Map 报告进度（实时显示给整个团队）。Scope `factory:run`。需要 run。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `message` | string | 是 | 一两句话：已完成 / 正在做什么 |
| `percent` | integer | | 0–100 |

结果：`{"ok": true}`，文本 `Progress reported.` 错误：`invalid_params`（`percent must be 0-100`）、`no_run`、
`conflict`（`run-finished`）。

### factory-request-approval

在继续之前请求某个人（或工作流规定的角色）审批。Scope `factory:run`。需要 run。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `reason` | string | 是 | 需要审批什么以及原因 |

文本：`Approval requested. Wait for the decision before doing what needs approval.` 审批人会收到通知。

### factory-create-subtasks

把任务拆分为真实的子任务。Scope `factory:run` 或 `factory:write`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `parent` | string | | 父任务 key；默认为本次 run 的任务 |
| `tasks` | array | 是 | 不能为空；每次 run 最多 10 个 |
| `tasks[].title` | string | 是 | |
| `tasks[].role` | string | | `PM`、`BA`、`UI/UX`、`DEV`、`GUARDIAN`、`QC` 或工作区的角色 |
| `tasks[].description` | string | | |
| `tasks[].estimate` | integer | | 点数或小时，按项目的用法 |
| `tasks[].type` | string | | `feature`、`bug`、`chore`… |
| `tasks[].priority` | string | | |
| `tasks[].labels` | array of string | | |
| `tasks[].depends_on` | array of string | | 已有任务的 key，或 `#<n>` = 本次调用中的第 n 个子任务（从 0 开始） |
| `tasks[].acceptance_criteria` | array of string | | |

```json
{ "tasks": [{ "title": "API callback", "role": "DEV", "estimate": 2 }, { "title": "Test đăng nhập", "role": "QC", "depends_on": ["#0"] }] }
```

结果：`{tasks: [...], warnings: [...]}`。限制：每次 run 10 个，单个根任务下 30 个，深度 2 层，且角色需允许拆分。
超出时：返回 `conflict` 错误（规则 `split-limit`），带有消息和 `params`（`max_depth`、`per_run`、`per_root`、`depth`、
`run_count`、`root_count`），以便 agent 重新拆分。

### factory-attach-artifact

把产出附加到 run：pull request、测试结果、截图、链接、备注。Scope `factory:run`。需要 run。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `kind` | string | 是 | `pr`（记为该 run 的 PR）、`test_result`、`ci`、`screenshot`、`link`、`note` |
| `url` | string | | |
| `text` | string | | 内容（测试日志、备注） |
| `title` | string | | |
| `passed` | boolean | | 用于 `test_result` 和 `ci` |

`url`、`text`、`passed` 至少需要一个。

```json
{ "kind": "test_result", "title": "cargo test", "passed": true, "text": "test result: ok. 42 passed" }
```

### factory-get-rules

项目为 agent 制定的规则（分支、commit、完成前的测试）。Scope `factory:read`。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `project` | string | | key 或 id；默认为本次 run 的项目（`TTM_PROJECT`） |

文本：受管理的 Markdown 块（与 `ttm rules apply` 写入 `AGENTS.md` 的内容相同）。结构化：`rev`、
`managed_markdown`、`hooks`、`checks[{id, description, command, enabled}]`。

### factory-renew-lease

持有或续期对某块代码区域（glob）的修改权，避免其他 agent 同时修改。Scope `factory:run`。需要 run。

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `paths` | array of string | 是 | glob，例如 `src/auth/**` |

结果：`{granted, expires_at, held_by, paths}`。区域正被其他 run 持有时：`granted: false`，`held_by` 指明持有者；
请先做别的事，或等待后再次调用。
