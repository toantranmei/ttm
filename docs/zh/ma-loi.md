---
title: 错误码
description: API、ttm 和 ttm-mcp 返回的稳定错误码、含义及处理方法。
order: 14
section: reference
---

## 错误的格式

服务器以 `application/problem+json` 格式返回错误：

```json
{
  "code": "insufficient_scope",
  "title": "this token lacks the scope notee:write",
  "status": 403,
  "params": { "required": "notee:write" }
}
```

| 字段 | 含义 |
| --- | --- |
| `code` | 用于分支判断的稳定错误码（见下表） |
| `title` | 面向读者的英文说明 |
| `status` | HTTP 状态码 |
| `field` | 出错的输入字段（用于 `bad_request`、`conflict`），例如 `anchor.quote.exact` |
| `rule` | 具体违反的规则（用于 `bad_request`、`conflict`、`precondition_failed`），例如 `split-limit` |
| `params` | 相关的值，例如 `{"required": "notee:write"}`、`{"max_depth": 2, "per_run": 10}` |

`ttm` 向 stderr 输出 `error: <title> (<code>, rule <rule>, field <field>, HTTP <status>) <params>`（`rule`、`field`、
`params` 部分仅在服务器发送时出现），例如：

```text
error: At most 10 subtasks per run (10 already). (conflict, rule `split-limit`, HTTP 409) {"depth":1,"max_depth":2,"per_root":30,"per_run":10,"root_count":12,"run_count":10}
```

使用 `--json` 时，stdout 还会输出 `{"error": {"code", "message", "status", "rule", "field", "params"}}`（后三个字段仅在
服务器发送时出现），方便脚本按 `rule` 分支。MCP 工具返回 `isError: true`、文本 `<消息> (<code>)`，以及带有相同字段的
`structuredContent.error`：agent 可按 `rule`（`split-limit`、`rev-conflict`、`approval-required`）分支，或读取
`insufficient_scope` 的 `params.required`。

> **注意：** 对于 `bad_request`、`conflict`、`precondition_failed` 错误，`code` 比较笼统；具体原因在
> `rule`（下表中的 **Rule** 列）和提示信息中。

## 认证与权限

| Code | HTTP | 含义 | 处理方法 |
| --- | --- | --- | --- |
| `invalid_token` | 401 | Token 错误、已过期或已撤销 | 重新 `ttm login`；检查是否有旧的 `TTM_TOKEN` |
| `unauthorized` | 401 | 没有 token | `ttm login` 或设置 `TTM_TOKEN` |
| `insufficient_scope` | 403 | Token 缺少 scope；`params.required` 指明所需的 scope | 使用该 scope 登录（`ttm login --scope …`）或创建新的 PAT |
| `token_not_allowed` | 403 | 此路由不对 token 开放（管理 token、共享、付款、删除页面、创建工作区…） | 在网页上操作 |
| `forbidden` | 403 | 你对该资源没有权限 | 向页面/工作区的所有者申请权限 |
| `upgrade_required` | 426 | `ttm` 版本低于服务器的 `min_cli` | `ttm self-update` |

## 登录（OAuth）

| Code | 含义 | 处理方法 |
| --- | --- | --- |
| `access_denied` | 你点击了 **拒绝** | 重新运行 `ttm login` |
| `authorization_pending`、`slow_down` | （device flow）正在等待你授权 / 轮询过快 | `ttm` 会自动处理，无需操作 |
| `expired_token` | 设备码超过 10 分钟 | 重新 `ttm login --device` |
| `invalid_grant` | 授权码已使用或已过期；PKCE 错误；refresh token 已过期、不存在或**被重复使用**（整个 token 族都会被撤销） | 重新 `ttm login` |
| `invalid_client` | Client 不存在 | 检查 `--host` |
| `invalid_scope` | `--scope` 中有未知的 scope | 使用正确的 scope 名称（见 [登录与 token](dang-nhap.md)） |
| `invalid_request` | 缺少参数 | 如果在使用 `ttm` 时遇到，请报告问题 |
| `invalid_target` | （远程 MCP）`resource` 不是 `…/api/v1/mcp` | 使用正确的 MCP URL |

## 通用

| Code | HTTP | 含义 |
| --- | --- | --- |
| `bad_request` | 400 | 输入错误；查看 `rule` 和 `field` |
| `not_found` | 404 | 不存在，或你无权查看（服务器不区分这两种情况） |
| `conflict` | 409 | 状态冲突；查看 `rule` |
| `precondition_failed` | 412 | 数据在你读取之后已被修改（`rule: rev-conflict`）；重新读取后再试 |
| `rate_limited` | 429 | 超出限制（例如每个 token 每小时 60 次 Notee 写入）；稍候再试 |
| `internal` | 500 | 服务器错误；稍后重试，若反复出现请报告问题 |
| `http_<mã>` | 任意 | 响应不是 problem+json（代理、502 网关…） |

`ttm` 的本地错误码：`network`（无法连接）、`decode`（响应格式异常）、`invalid_url`、`error`（其他错误：未登录、
文件无法读取…）。

## Notee：写入页面

| Code / Rule | HTTP | 含义 | 处理方法 |
| --- | --- | --- | --- |
| `too_many_blocks` | 413 | 单次写入新增、修改或删除超过 1,000 个 block；或 `<unknown>`/`<truncated>` 重复过多 | 拆分（多个 `update_content`，或先创建再逐步追加） |
| `markdown_too_large` | 413 | Markdown 超过 2 MB | 拆分 |
| `would_delete_children` | 409 | 新内容会丢掉子页面或子数据库 | 保留 `<page>`/`<database>` 标签，若确实要删除则加上 `allow_deleting_content` / `--allow-deleting-content` |
| `would_delete_truncated` | 409 | 去掉了长页面的 `<truncated/>` 行 | 将该行保留在末尾，或使用 `allow_deleting_content` |
| rule `one_of` | 400 | 没有要修改的内容，或同时使用了 `update_content` 和 `replace_content` | 二选一 |
| rule `suggest_content_only` | 400 | 修改建议中带有 `title`/`properties` | 修改标题、列请使用 `mode: apply` |
| rule `mode` | 400 | `mode` 不是 `apply`/`suggest` | |
| rule `is_database` | 400 | 把数据库当作页面读取 | 使用 `notee-fetch`（返回 schema）或 `GET /databases/{id}/schema` |
| rule `batch_size` | 400 | 一次创建 0 个或超过 20 个页面 | 1–20 |
| rule `parent_shape` | 400 | `parent` 同时包含 `page_id`、`database_id`、`workspace_id` 中的多个 | 只能有一个 |
| `rate_limited` | 429 | Agent 每小时写入超过 60 次（按 token 计） | 将修改合并到更少的写入中 |

警告（不是错误，位于 `warnings` 中）：`update_content[i]: old_str not found; skipped`、
`… old_str matches N places; give more context or replace_all; skipped`、`old_str is empty`，以及被忽略的标签
（`<unknown url=…> is not a block of this page; left out`、`<page> without the url of an existing page; left out`…）。

## Notee：评论与修改建议

| Code / Rule | HTTP | 含义 | 处理方法 |
| --- | --- | --- | --- |
| `anchor_not_found` | 422 | 页面中没有该引用片段（或资源不含文字） | 从 `notee-fetch`/`ttm notes get` 中原样复制文字 |
| `ambiguous_anchor` | 422 | 引用片段出现在多处 | 添加 `prefix` 和/或 `suffix` |
| rule `quote_length` | 400 | `exact` 为空或超过 1,000 个字符 | |
| `stale`（位于 `stale_change_ids` 中） | | 修改建议中某项变更的锚点片段已不存在 | 基于新内容重新提出建议 |

## Notee：数据库

| Code / Rule | HTTP | 含义 | 处理方法 |
| --- | --- | --- | --- |
| `sql_syntax` | 400 | SQL 语法错误 | 查看提示信息中的位置 |
| `sql_not_select` | 400 | 不是恰好一条 `SELECT` 语句（含 DML、多条语句） | 只写一条 `SELECT` |
| `sql_table` | 400 | 读取了 `rows` 以外的表 | 只能 `FROM rows` 和你自己的 CTE |
| `sql_function` | 400 | 使用了不允许的函数 | 函数列表见 [Notee](notee.md) |
| `sql_forbidden` | 400 | 使用了被禁止的子句（`SELECT INTO`、`FOR UPDATE`、参数 `$1`、名为 `rows` 的 CTE、嵌套 `WITH`、`LATERAL`、`reg*` 类型转换） | 去掉该子句 |
| `sql_error` | 400 | Postgres 执行时报错 | 按提示信息修改 |
| `query_timeout` | 408 | 查询超过 3 秒 | 缩小过滤范围，加上 `LIMIT` |
| `result_too_large` | 413 | 单页结果超过 4 MB | 少选几列，减小 `page_size` |
| `query_busy` | 429 | 你同时运行的查询超过 2 个，或服务器繁忙 | 稍等片刻再试 |
| `sql_sandbox_unavailable` | 503 | 服务器尚未启用安全 SQL | 使用 JSON 过滤器 |
| rule `sql_or_filter` | 400 | 同时提供了 `sql` 和 `filter` | 二选一 |
| rule `unknown_prop` | 400 | `filter`/`sorts` 中的 property 不存在 | 先读取 schema |
| rule `filter_shape` | 400 | 过滤器格式错误 | `{"property", "op", "value"}`、`{"and": []}`、`{"or": []}` |
| rule `invalid_cursor` | 400 | Cursor 已损坏 | 使用原样的 `next_cursor` |
| `row_deleted` | 409 | 该行在回收站中 | 在网页上恢复，或创建新行 |
| rule `row_elsewhere` | 400 | 该 id 是另一个数据库的行 | |

## Factory

大多数是带有以下 `rule` 的 `409 conflict`：

| Rule | 含义 | 处理方法 |
| --- | --- | --- |
| `split-limit` | 超出子任务限制（深度、每个 run、每个根任务），或该角色不允许拆分；`params` 中有当前的限制值 | 拆得更少或更浅 |
| `approval-required` | 状态转换需要审批；或变更请求（change request）必须由其他角色审批 | 由具备该角色的人执行 `ttm task approve` |
| `tests-required`、`ci-required` | 最后一次运行尚无通过的测试 / CI | 运行测试，附上 `test_result` |
| `blocked-by-dependency` | 依赖的任务或子任务尚未完成 | |
| `wip-limit` | 目标列的 WIP 已满 | |
| `sprint-required` | 需要先将任务加入 sprint | |
| `budget-exceeded` | 任务的 token 预算已用尽 | |
| `transition-not-allowed` | 工作流不允许该转换（或状态未知） | 在网页上查看工作流 |
| `rev-conflict` (412) | 任务在读取之后已被修改 | 重新运行命令 |
| `run-active` | 任务已有正在执行的 run | 等待，或 `ttm agent stop` |
| `run-finished` | Run 已结束 | |
| `not-offered` | Run 已不再等待领取 | |
| `no-approval-pending` | 任务没有在等待审批 | |
| `not-pending`、`not-author` | 变更请求已有决定 / 创建者不能自行审批 | |
| `merge-finished` | 合并队列中的该项已完成 | |
| `key_taken` | 项目 key 已被使用 | 换一个 `--key` |
| `ambiguous-key` | 多个项目使用了相同的 key | 使用任务 id |
| `dependency-cycle`、`not-found`（`field: dependencies`） | 循环依赖或依赖不存在 | |

`ttm sync` 收到带有 `conflicts` 列表（任务、字段、base、local、server）的 `409` 时会将其打印出来；见 [Factory](factory.md)。

## Runner

| Code | 含义 | 处理方法 |
| --- | --- | --- |
| `repo_not_configured` | 作业（job）所属项目在 `runner.toml` 中没有 clone，或 clone 的 `origin` 与作业的仓库不一致 | `ttm runner config --repo KEY=<đường dẫn clone>` |
| `invalid_branch` | 作业的分支名不符合 git 规则 | 在网页上修改任务/分支 |
| `invalid_worktree` | 作业的 `worktree_hint` 是绝对路径、含 `..`，或指向 `worktree_root` 之外 | 作为服务器问题反馈；runner 绝不会在 `worktree_root` 之外创建 worktree |

这两个错误码会在 run 结束时作为 `error` 上报给服务器。

## MCP 工具

| Code | 含义 |
| --- | --- |
| `no_run` | 该工具需要一次 agent 运行（`TTM_RUN_ID`） |
| `no_task` | 缺少 `task` 且没有 `TTM_TASK` |
| `no_project` | 缺少 `project` 且没有 `TTM_PROJECT` |
| `unsupported` | 服务器不支持该工具 |

## ttm 的常见提示信息

| 提示信息 | 含义 |
| --- | --- |
| `` not signed in to <host> (profile <p>); run `ttm login` `` | 该（profile，host）没有 token |
| `the saved token has expired; run ttm login` | Token 已过期，且没有 refresh token |
| `the session expired and could not be renewed; run ttm login` | 续期失败 |
| `the server rejected this token (…)` | `ttm login --token` 使用了无效的 token |
| `cannot reach <host>` | 网络、DNS 问题，或 `--host` 错误 |
| `this directory is not linked to a project; run ttm project link <KEY>` | 在已关联的仓库之外运行了 Factory 命令 |
| ``no agent harness found; install Claude Code (`claude`) first`` | Runner 的 `PATH` 中没有 `claude` |
| `timed out after 5 minutes waiting for the browser` | 没有及时点击 **允许** |
| `invalid TOML in <file>: …` | `config.toml`/`credentials.toml`/`runner.toml` 语法错误 |
