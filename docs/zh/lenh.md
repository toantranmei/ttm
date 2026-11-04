---
title: 命令参考
description: ttm 的全部命令与子命令，包括语法、选项、默认值、示例、JSON 输出和退出码。
order: 10
section: reference
---

```text
ttm [OPTIONS] <COMMAND>
```

`ttm --help` 和 `ttm <命令> --help` 会打印原始帮助信息（英文）。本页对应版本 **0.2.0**。输出约定（结果行、`Next:`、符号、徽章、颜色）：[命令行界面](giao-dien.md)。

## 全局选项

适用于所有命令，放在子命令之前或之后均可（`ttm whoami --json` = `ttm --json whoami`）。

| 选项 | 环境变量 | 作用 |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | 服务器，例如 `http://localhost:3000` |
| `--profile <NAME>` | `TTM_PROFILE` | `config.toml` 中的 profile |
| `--json` | | 在 stdout 上以 JSON 输出结果（保持 0.1.6 的结构，只新增键） |
| `--plain` | `TTM_PLAIN=1` | 适合日志的纯文本：无颜色、边框、符号、emoji |
| `-y`, `--yes` | | 对所有确认回答“是”并采用默认值，不再提问（脚本、CI） |
| `-v`, `--verbose` | `TTM_LOG` | 向 stderr 输出更多日志（`-v` 为 info，`-vv` 为 debug） |
| `-h`, `--help` | | 帮助 |
| `-V`, `--version` | | 版本（`ttm 0.2.0`；仅在顶层可用） |

## 退出码

| 代码 | 何时 |
| --- | --- |
| `0` | 成功 |
| `1` | 出错（网络、认证、API、文件……）；`ttm doctor` 有 `FAIL` 项；`ttm sync --check` 发现不一致；`ttm sync` 遇到冲突；`ttm agent run` 以 `succeeded` 以外的状态结束 |
| `2` | 命令行语法错误（未知选项、缺少参数）；缺少标志且无法提问（`missing_flag`）；需要确认但没有终端也没有 `--yes`（`confirmation_required`）；服务器没有该功能（`unsupported_feature`）；标志值无效（`invalid_flag`）。对于 `ttm hook …`：操作被规则阻止 |

错误以同一模式输出到 stderr：

```text
✕ <摘要>
  rule: <rule>  field: <field>
  Fix: <修复方法>
  Help: https://toantranmei.com/help/ttm/ma-loi#<rule>
```

使用 `--plain` 时第一行为 `ERROR: <摘要>`。使用 `--json` 时，还会向 stdout 输出 `{"error": {"code", "message", "status?",
"rule?", "field?", "params?"}}`（参见 [配置](cau-hinh.md) 和 [错误码](ma-loi.md)）。

## 服务器功能

新命令只有在服务器的 `discovery.features` 含有对应字符串时才调用 API；否则打印 `✕ This server does not support <x> yet`
（rule `unsupported_feature`，退出码 2），不调用任何 API。

| 功能 | 命令 |
| --- | --- |
| `workflow_v2` | `project init`（模板、预设 agent）、`workflow`、`automation` |
| `agent_v2` | `agent list/show/create/edit/pause/resume/archive/skills/mcp`、`skill`、`mcp-server` |
| `ascode_v2` | `workflow validate --file`、`cr`（配置变更请求）、`sync`（`.ttm/` 中的配置目录） |
| `templates` | `template` |
| `membership` | `whoami`/`login` 的 Mei-er 行、徽章 |
| `confirmations` | 通过 MCP 的两步确认（参见 [MCP 工具](mcp-cong-cu.md)） |

## 概要

| 命令 | 作用 |
| --- | --- |
| [`login`](#login) | 登录（浏览器、`--device`、`--token`） |
| [`logout`](#logout) | 撤销 token 并从本机删除 |
| [`whoami`](#whoami) | 当前登录的用户 |
| [`status`](#status) | 正在使用的 host、profile、token、工作区 |
| [`doctor`](#doctor) | 检查环境 |
| [`workspace`](#workspace) | `list`、`switch`、`current`、`members` |
| [`token`](#token) | `list`、`create`、`revoke`（打开网页） |
| [`notes`](#notes) | `search`、`get`、`query`、`comments`、`create`、`edit`、`comment`、`rows`、`suggestions`、`resolve` |
| [`project`](#project) | `init`、`link`、`open`、`status` |
| [`sync`](#sync) | 同步计划 |
| [`task`](#task) | `list`、`show`、`claim`、`done`、`approve`、`split` |
| [`rules`](#rules) | `apply` |
| [`hook`](#hook) | `pre-bash`、`stop`（由 agent 调用） |
| [`workflow`](#workflow) | `show`、`validate`、`simulate`、`diff`、`publish`、`upgrade`、`copy` |
| [`automation`](#automation) | `list`、`show`、`test`、`enable`、`disable`、`runs` |
| [`runner`](#runner) | `up`、`down`、`status`、`config`（+ `config mcp`、`config claude-token`）、`install-service`、`uninstall-service` |
| [`agent`](#agent) | `run`、`logs`、`stop`、`list`、`show`、`create`、`edit`、`pause`、`resume`、`archive`、`skills`、`mcp` |
| [`skill`](#skill) | `list`、`push`、`pull`、`show`、`review`、`import` |
| [`mcp-server`](#mcp-server) | `list`、`add`、`connect`、`remove`（工作区的远程 MCP 服务器） |
| [`cr`](#cr) | `list`、`show`、`approve`、`reject` |
| [`inbox`](#inbox) | 通知、`--watch`、`read` |
| [`template`](#template) | `list`、`publish`、`install` |
| [`mcp`](#mcp) | `install` |
| [`self-update`](#self-update) | 更新 ttm |
| [`completion`](#completion) | 命令补全脚本 |

---

## login

```text
ttm login [--device | --token <PAT>] [--scope <SCOPE>] [--no-browser]
```

登录并为当前（profile，host）保存 token。默认打开浏览器（loopback `127.0.0.1` + PKCE，等待 5 分钟）。
指定了 `--host`/`TTM_HOST` 或 profile 尚未设置 host 时，会把 host 写入 profile。

| 选项 | 默认值 | 作用 |
| --- | --- | --- |
| `--device` | | Device flow：打印一个代码，在 `<host>/activate` 输入。不能与 `--token` 同时使用。 |
| `--token <PAT>` | | 保存一个已有的 token；`-` 表示从 stdin 读取 |
| `--scope <SCOPE>` | `workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run factory:admin offline_access` | 申请的 scope，以空格分隔（不适用于 `--token`） |
| `--no-browser` | | 不打开浏览器，只打印 URL |

```sh
ttm login
ttm login --device
printf %s "$PAT" | ttm login --token -
ttm --profile dev login --host http://localhost:3000 --scope "workspace:read notee:read"
```

```text
✓ Signed in as An Nguyễn <an@example.com>
  host https://toantranmei.com · profile default · token in keyring
✦ Mei-er since 2025-11  Thank you for being a Mei-er. You keep this going.
Next: ttm project init
```

Mei-er 会员会多一行金色文字（等级、加入月份、按等级的感谢；周年当天有祝福）。目录尚未关联项目时 `Next:` 为
`ttm project init`，否则为 `ttm whoami`。缺少 `factory:admin` 的旧 token 仍可使用；管理命令报 `insufficient_scope` 并建议
`ttm login`。

JSON：`host`、`profile`、`method`（`browser` | `device` | `token`）、`stored_in`（`keyring` | `file` | `env`）、`user`
（token 缺少 `workspace:read` 时为 `null`）、`meta.membership`（`{tier, status, since, until, badge}` 或 `null`）。

## logout

```text
ttm logout
```

在服务器上撤销访问 token 和 refresh token，然后从 keyring 和 `credentials.toml` 中删除。即使服务器
未确认，也会删除本地副本。无法清除 `TTM_TOKEN`（只给出警告）。

```text
✓ Signed out of https://toantranmei.com
  profile default · revoked on the server: yes
Next: ttm login
```

JSON：`host`、`profile`、`revoked`（服务器已确认）、`removed`（有 token 从本机删除）。

## whoami

```text
ttm whoami
```

调用 `/me`（需要 `workspace:read`）。

```text
An Nguyễn <an@example.com>
     user  An Nguyễn
    email  an@example.com
     host  https://toantranmei.com
  profile  default
workspace  Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
✦ Mei-er since 2025-11  Thank you for being a Mei-er. You keep this going.
```

非会员会看到一行淡色文字 `Support toantranmei → https://toantranmei.com/mei-er`（仅在 `whoami` 中，其他命令都不会提）。
使用 `--plain`：`user: An Nguyễn`…，徽章为 `[Mei-er]`。

JSON：`host`、`profile`、`user`（`id`、`email`、`name`、`avatar_url`、`locale`、`plan`、`membership?`）、`workspace`（`id`、
`name`、`kind`、`role`、`slug` 或 `null`）、`meta.membership`。

## status

```text
ttm status
```

显示本机将要使用的设置；无需登录或联网（始终以 0 退出）。

```text
✓ Signed in to https://toantranmei.com
  version  ttm 0.2.0
     host  https://toantranmei.com
  profile  default (/Users/an/.config/ttm/config.toml)
    token  ✓ ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
  keyring  ✓ OS keyring
workspace  not chosen
   server  ✓ API v1 (min ttm 0.1.0), features: notee, factory, workflow_v2, agent_v2
Next: ttm workspace switch <name>
```

JSON：`version`、`host`、`profile`、`config_file`、`signed_in`、`token`（`source`、`masked`、`expires_at`、`expired`、
`scope`）、`workspace`、`server`（`api_version`、`min_cli`、`features`）或 `null`。

## doctor

```text
ttm doctor
```

每行一项检查：`host`、`discovery`（重新获取，不使用缓存）、`version`（与 `min_cli` 比较）、`token`（调用
`/me`）、`keyring`、`git`、`claude`、`ttm-mcp`。级别为 `ok`、`warn`、`FAIL`。**存在 `FAIL` 时以 1 退出**。

```text
! 1 warning(s), no failure
CHECK      STATUS  DETAIL
────────────────────────────────────────────────────────────────────────
host       ✓       https://toantranmei.com (profile default)
discovery  ✓       api_base https://toantranmei.com/api/v1
version    ✓       ttm 0.2.0, server API v1
token      ✓       ttm_oat_Ab12… (keyring) as an@example.com
keyring    ✓       OS keyring available
git        ✓       git version 2.50.1
claude     !       Claude Code not found on PATH (needed by ttm runner)
ttm-mcp    ✓       /Users/an/.cargo/bin/ttm-mcp
Next: install Claude Code, then ttm doctor
```

`--plain`：用 `ok`、`warn`、`FAIL` 代替 `✓`、`!`、`✕`，标题下没有分隔线。

JSON：`{name, level, detail}` 数组。

## workspace

### workspace list

```text
ttm workspace list
```

列出你所属的工作区；`●`（`--plain`：`*`）标记 profile 的当前工作区。

```text
2 workspace(s), current: Team Aurora
   NAME         SLUG    ROLE    KIND      ID
────────────────────────────────────────────────────────────────────────
   Cá nhân      -       owner   personal  0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
●  Team Aurora  aurora  member  team      0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

JSON：`{id, name, kind, role, slug?, current}` 数组。

### workspace switch

```text
ttm workspace switch <WORKSPACE>
```

为 profile 设置当前工作区。`<WORKSPACE>`：id、slug 或名称（精确匹配，不区分大小写），或者
名称的前缀（仅当只匹配一个工作区时）。匹配到多个时：`matches several workspaces (…); use the id`。

```text
✓ Current workspace: Team Aurora
  id 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71 · profile default
Next: ttm project init
```

### workspace current

```text
ttm workspace current
```

打印名称和 id（仅当服务器能响应时才有名称）。JSON：`profile`、`workspace_id`、`workspace`。

### workspace members

```text
ttm workspace members [--workspace <ID>]
```

工作区成员（默认当前工作区）：名称及 Mei-er 徽章、邮箱、角色。需要 `workspace:read`。

```text
NAME                    EMAIL              ROLE
An Nguyễn [Mei-er]      an@example.com     owner
Lan                     lan@example.com    member
```

JSON：`{user{id, name, email, avatar_url, membership?}, role, membership?}` 数组。

## token

token 在网页上管理；这些命令会打开 `<host>/me/ket-noi`（并打印 URL）。JSON：`{"url": "…"}`。

| 命令 | 选项 | 作用 |
| --- | --- | --- |
| `ttm token list` | | 打开 token 列表 |
| `ttm token create` | `--name <NAME>`、`--scope <SCOPE>`（逗号或空格分隔）、`--expires <DAYS>`（`1`–`365`，例如 `30d`） | 打开创建 token 的表单，并附带预填值 |
| `ttm token revoke [ID]` | | 打开撤销页面 |

```sh
ttm token create --name CI --scope notee:read,workspace:read --expires 90d
```

## notes

页面/数据库参数接受 id 或 URL。参见 [Notee](notee.md) 指南。

### notes search

```text
ttm notes search <QUERY> [--limit <N>] [--workspace <ID>]
```

| 选项 | 默认值 | 作用 |
| --- | --- | --- |
| `--limit <N>` | `10` | 最大结果数（服务器最多返回 20 条） |
| `--workspace <ID>` | 当前工作区 | 限定在某个工作区内 |

```text
0190f3b0-1111-7aaa-8bbb-0123456789ab  Kế hoạch quý 4
0190f3b0-2222-7aaa-8bbb-0123456789ab  Họp tuần
    …chốt kế hoạch phát hành bản 0.2 trước ngày 15…
```

JSON：`{id, workspace_id, title, icon, snippet}` 数组。

### notes get

```text
ttm notes get <ID> [--block <BLOCK>] [-o <FILE>]
```

以 Notee-flavored Markdown 打印页面，或打印数据库的标题和属性。

| 选项 | 作用 |
| --- | --- |
| `--block <BLOCK>` | 读取被截断页面的某一部分（`unknown_block_ids` 中的 id） |
| `-o`, `--output <FILE>` | 将 Markdown 写入文件而不是 stdout |

JSON（页面）：`id`、`title`、`icon`、`url`、`parent`、`properties`（数据库行）、`markdown`、`truncated`、
`unknown_block_ids`。JSON（数据库）：`id`、`title`、`url`、`properties[]`、`views[]`。

### notes query

```text
ttm notes query <DATABASE> (--sql <SQL> | --filter <FILE>) [--page-size <N>] [--cursor <CURSOR>]
```

| 选项 | 作用 |
| --- | --- |
| `--sql <SQL>` | 在 `rows` 表上执行的只读 SQL |
| `--filter <FILE>` | JSON 文件 `{"filter": …, "sorts": …}` 或仅包含过滤条件；`-` 表示读取 stdin |
| `--page-size <N>` | 1–100 行（服务器默认 100）；超出范围时 `ttm` 在调用服务器之前就会停止：`invalid value '500' for '--page-size <N>': 500 is out of range: --page-size takes 1 to 100 rows per page` |
| `--cursor <CURSOR>` | 上一页的 `next_cursor` |

`--sql` 和 `--filter` 必须且只能指定一个。每行打印 `<id>  <values JSON>`；还有下一页时，stderr 打印
`more rows: --cursor <CURSOR>`。JSON：`columns`、`rows[{id, values}]`、`next_cursor`。

### notes comments

```text
ttm notes comments <PAGE> [--all]
```

列出未关闭的评论 thread；`--all` 包括已关闭的 thread。JSON：thread 数组（`id`、`anchor`、`quote`、`resolved_at`、
`comments[]`）。

### notes create

```text
ttm notes create (--parent <PAGE> | --database <DATABASE> | --workspace <WORKSPACE>) -f <FILE> [--title <TITLE>] [--icon <ICON>]
```

| 选项 | 作用 |
| --- | --- |
| `--parent <PAGE>` | 在此页面下创建 |
| `--database <DATABASE>` | 作为数据库的一行创建 |
| `--workspace <WORKSPACE>` | 在工作区顶层创建（id） |
| `-f`, `--file <FILE>` | Markdown 文件（必填）；`-` 表示读取 stdin |
| `--title <TITLE>` | 标题（默认：第一个 `# ` 行，否则为文件名） |
| `--icon <ICON>` | Emoji |

```text
✓ Created Ghi chú họp
  id 0190f3c4-…
  https://toantranmei.com/notes/0190f3c4-…
Next: ttm notes get 0190f3c4-…
```

JSON：`{id, title, url}`。

### notes edit

```text
ttm notes edit <PAGE> (--old <TEXT> --new <TEXT>)... [--replace-all] [--suggest] [--allow-deleting-content]
ttm notes edit <PAGE> --replace -f <FILE> [--suggest] [--allow-deleting-content]
```

| 选项 | 作用 |
| --- | --- |
| `--old <TEXT>` | 要查找的文本（可重复），区分大小写，必须恰好匹配一处 |
| `--new <TEXT>` | 替换同一位置 `--old` 的文本 |
| `--replace-all` | 替换所有匹配处 |
| `--replace` | 用 `-f` 指定的文件替换全部内容（不能与 `--old`/`--new` 同时使用） |
| `-f`, `--file <FILE>` | 供 `--replace` 使用的文件；`-` 表示读取 stdin |
| `--suggest` | 创建修改建议，而不是直接写入 |
| `--allow-deleting-content` | 允许删除子页面、子数据库和 `<truncated/>` 部分 |

```text
Page updated.
warning: update_content[0]: old_str not found; skipped
```

或者 `Suggestion <id> created.`，或者 `No change applied.` JSON：`applied`、`warnings`、`seq`、`snapshot_seq`、
`suggestion_id`。

### notes comment

```text
ttm notes comment <PAGE> -m <MESSAGE> [--quote <QUOTE> [--prefix <PREFIX>] [--suffix <SUFFIX>] | --thread <THREAD>]
```

| 选项 | 作用 |
| --- | --- |
| `-m`, `--message <MESSAGE>` | 内容（必填） |
| `--quote <QUOTE>` | 用于锚定的引用文本；留空 = 针对整个页面的评论 |
| `--prefix <PREFIX>`, `--suffix <SUFFIX>` | 引用文本紧前/紧后的几个字，用于引用文本出现多次的情况（需要 `--quote`） |
| `--thread <THREAD>` | 在此 thread 中回复（不能与 `--quote` 同时使用） |

```text
Comment thread 0190f3e0-… created.
```

### notes rows upsert

```text
ttm notes rows upsert <DATABASE> -f <FILE>
```

JSON 文件 `[{"id"?: "…", "values": {"<property>": …}}]`，1–100 行；`-` 表示读取 stdin。

```text
2 row(s) saved: 0190f3c7-…, 0190f3c5-…
```

JSON：`{"ids": [...]}`，顺序与输入一致。

### notes rows delete

```text
ttm notes rows delete <DATABASE> <IDS>...
```

将这些行移入回收站。`2 row(s) deleted.`

### notes suggestions

```text
ttm notes suggestions <PAGE> [--all]
```

列出待处理的修改建议（`--all`：还包括 `accepted`、`rejected`、`stale`），附带每处修改的 id 以及修改前/后的 Markdown。

### notes resolve

```text
ttm notes resolve <SUGGESTION> (--accept [--change <ID>]... | --reject)
```

| 选项 | 作用 |
| --- | --- |
| `--accept` | 接受 |
| `--reject` | 拒绝 |
| `--change <ID>` | 只接受这些修改（可重复，需要 `--accept`） |

```text
Suggestion 0190f3d1-…: accepted
Stale (anchor text gone): c3
```

JSON：`id`、`status`、`stale_change_ids`。

## project

### project init

```text
ttm project init [--name <NAME>] [--key <KEY>] [--workspace <ID>] [--repo <URL>] [--workflow <W>]
                 [--agents create|reuse|none] [--runner this|none] [--yes]
```

在服务器上为当前仓库创建项目，然后进行关联（同 `link`）。需要 `factory:write`。在终端中是**向导**：名称 → key（按名称建议，
`^[A-Z][A-Z0-9]{1,9}$`）→ 工作区 → 仓库 → workflow → 预设 agent（从 `GET /factory/agent-presets` 预览；已有相同角色的 agent
时询问复用还是新建）→ runner → 确认。每个问题都有标志；`--yes` 采用默认值。向导结束后 stderr 显示
`Equivalent: ttm project init … --yes`。兼容 0.1.6：不是终端、给出 `--key` 且没有 `--workflow`/`--agents`/`--runner` 时，
像以前一样直接创建，不提问。workflow 与预设 agent 需要 `workflow_v2` 功能。

| 选项 | 默认值 | 说明 |
| --- | --- | --- |
| `--name <NAME>` | 仓库根目录的名称 | 项目名称 |
| `--key <KEY>` | 按名称建议 | 任务 key 前缀（2–10 个大写字母或数字，以字母开头） |
| `--workspace <ID>` | 当前工作区 | |
| `--repo <URL>`（旧：`--repo-url`） | `git remote get-url origin` | |
| `--workflow <W>` | `balanced` | `professional`（9 个状态、6 个 agent）、`balanced`（5 个状态、3 个 agent，约少 40% token）、`kanban`、`scrum`、`autopilot` 或模板 slug |
| `--agents <A>` | `create` | `create` 创建模板的 agent，`reuse` 复用相同角色的 agent，`none` 不要 agent |
| `--runner <R>` | `this` | `this`：在 `runner.toml` 写入 `[repos] KEY = <当前目录>`；`none` |

```text
OK: Created project WEB (Web todo)
  linked /home/an/web-todo · plan rev 1 · workflow balanced
AGENT              CODE   STATUS
[agent] DEV · Code  dev    created
[agent] QA · Test   qa     created
[agent] Lead        lead   reused
Next: ttm runner up
```

JSON：`project.toml` 的内容（有 agent 时 + `agents[{code, agent_id, created}]`）。

### project link

```text
ttm project link <PROJECT>
```

将仓库与已有项目（key 或 id）关联，写入 `.ttm/project.toml`、`.ttm/plan`、`.ttm/base`、`.ttm/.gitignore`。

```text
✓ Linked /Users/an/code/aurora to AUR (Aurora)
  plan rev 42
  Commit .ttm/project.toml and .ttm/plan.
Next: ttm rules apply
```

### project open

```text
ttm project open
```

打开 `<host>/factory?project=<id>`。JSON：`{"url": "…"}`。

### project status

```text
ttm project status
```

```text
project: AUR (0190f400-…)
host:    https://toantranmei.com
rev:     42
pending change request: 0190f4a1-…
```

## sync

```text
ttm sync [--check | --resolve]
```

将计划拉取到 `.ttm/plan`，并把本地修改作为变更请求（change request）提交。参见 [Factory](factory.md)。

| 选项 | 作用 |
| --- | --- |
| `--check` | 只报告本地与服务器是否不一致；**不一致时以 1 退出**。不写入任何内容。 |
| `--resolve` | 发生冲突时，让本地 agent（`claude`）在单独的 worktree 中合并，并提交新的变更请求 |

JSON：`local_rev`、`server_rev`、`local_changes`、`server_changes`、`change_request`、`skipped`、`nothing_to_send`、
`conflicts[{task, field, base, local, server}]`、`in_sync`。

## task

在已关联的仓库中运行（`show`、`claim`、`done`、`approve`、`split` 除外，它们接受任意 key）。

| 命令 | 选项 | 作用 | Scope |
| --- | --- | --- | --- |
| `ttm task list` | `--state <STATE>`（状态 id 或类型）、`--mine` | 项目的任务 | `factory:read` |
| `ttm task show <KEY>` | | 标题、状态、角色、负责人、描述 | `factory:read` |
| `ttm task claim <KEY>` | | 把任务分配给自己 | `factory:write` |
| `ttm task done <KEY>` | | 转为 `done`（guard 和审批关卡仍然生效） | `factory:write` |
| `ttm task approve <KEY>` | `--reject` | 批准/拒绝正在等待的步骤 | `factory:write` |
| `ttm task split <KEY> -f <FILE>` | `-f`, `--file`（必填；`-` = stdin） | 从 JSON 创建子任务（键可用 camelCase 或 snake_case，见 [Factory](factory.md)） | `factory:write` |

```text
AUR · 2 task(s)
   KEY     TITLE                          ROLE  ASSIGNEE            EST
────────────────────────────────────────────────────────────────────────
●  AUR-12  Đăng nhập bằng Google          DEV   🤖 Dev bot ✦ Mei-er   3
◐  AUR-13  Trang hồ sơ                    QA    👤 Lan                2
Next: ttm task show AUR-12
```

```text
╭─ AUR-12 · Đăng nhập bằng Google ─╮
│ ● doing                          │
╰──────────────────────────────────╯
   status  doing (in-progress)
     role  DEV
 assignee  🤖 Dev bot
delegator  👤 An ✦ Mei-er

Acceptance criteria
  ✓ Google button shows
  ○ Email is verified

Cho phép đăng nhập bằng tài khoản Google.

Next: ttm agent run AUR-12
```

其他输出：`✓ AUR-12 is now yours`、`✓ AUR-12 is now done`、`✓ AUR-12 approved`、`✓ AUR-12 rejected`（`--reject`）、
`✓ 2 subtask(s) created under AUR-12` JSON：任务对象（camelCase，`id` 为 key），`split` 则为 `{tasks, warnings}`。

## rules

### rules apply

```text
ttm rules apply
```

将 `<!-- ttm:begin -->…<!-- ttm:end -->` 块写入 `AGENTS.md` 和 `CLAUDE.md`，把 hook 合并到 `.claude/settings.json`，并保存
`.ttm/cache/rules.json`。打印已更改的文件，或 `✓ Rules already up to date` JSON：`changed`、`checks`。

## hook

由 Claude Code 通过 hook 调用（由 `rules apply` 安装）；从 stdin 读取 hook 的 JSON。阻止时**以 2 退出**，并在
stderr 上给出原因；允许或不在已关联的仓库中时以 0 退出。

| 命令 | Hook | 检查 |
| --- | --- | --- |
| `ttm hook pre-bash` | `PreToolUse`（matcher `Bash`） | 对 `tool_input.command` 检查 `no_push_main`（受保护分支来自缓存的规则，见 [Factory](factory.md)）、`no_verify`、`conventional_commits` |
| `ttm hook stop` | `Stop` | `tests_before_done`：`transcript_path` 中包含测试命令（`stop_hook_active` 时跳过） |

## runner

| 命令 | 作用 |
| --- | --- |
| `ttm runner up` | 在前台运行 runner（按 `Ctrl-C` 停止）；必要时注册本机，并清理旧的 worktree |
| `ttm runner down` | 停止 `runner.pid` 中的进程 |
| `ttm runner status` | Id、名称、是否运行中、容量、harness、仓库 |
| `ttm runner config [选项]` | 修改 `runner.toml` |
| `ttm runner install-service` | 安装登录时运行的服务（launchd、systemd `--user`、Task Scheduler），并带上运行该命令的 shell 的 `PATH` |
| `ttm runner uninstall-service` | 停止并移除服务（plist、unit 或 Task Scheduler 任务） |

`runner config` 的选项：

| 选项 | 默认值 | 作用 |
| --- | --- | --- |
| `--max-parallel <N>` | 根据 CPU 和内存 | 同时运行的 agent 数 |
| `--accept-team-jobs <true\|false>` | `false` | 接收队友任务的作业（job） |
| `--worktree-root <DIR>` | `~/.ttm/worktrees` | 创建 worktree 的位置 |
| `--keep-days <N>` | `3` | 已完成的 worktree 保留天数 |
| `--repo <KEY=PATH>` | | 项目的本地克隆（可重复） |
| `--name <NAME>` | 主机名 | 在网页上显示的名称 |
| `--auto-merge <true\|false>` | `false` | 自动合并合并队列的队首并 push |

### runner config mcp

本机的本地 MCP 服务器（`runner.toml` 中的 `[mcp.local.<name>]` 表，参见 [Runner](runner.md)）。修改文件时保留注释。服务器只会
得知名称、`allow` 和 `team_jobs`，永远不会得知命令或环境变量。

| 命令 | 选项 | 说明 |
| --- | --- | --- |
| `ttm runner config mcp add <NAME> --command <CMD>` | `--arg <ARG>`（可重复）、`--env K=env:VAR` 或 `K=<非机密值>`（可重复） | 添加或替换本地 MCP。名称 `^[a-z0-9][a-z0-9-]{0,31}$`，不能是 `ttm`。看起来像 token 的值会被拒绝：请用 `env:VAR` |
| `ttm runner config mcp allow <AGENT>` | `--mcp <NAME>`（可重复；默认所有本地 MCP） | 允许 agent（handle）使用 |
| `ttm runner config mcp deny <AGENT>` | `--mcp <NAME>`（可重复） | 取消权限 |
| `ttm runner config mcp team-jobs <NAME> on\|off` | | 是否允许他人委派的任务使用（默认 `off`） |
| `ttm runner config mcp remove <NAME>` | | 删除 |
| `ttm runner config mcp list` | | 名称、命令、`allow`、team jobs 的表格 + 一行 "Announced to the server: names and allowed agents only" |

```sh
ttm runner config mcp add github-local --command npx --arg -y --arg @modelcontextprotocol/server-github --env GITHUB_TOKEN=env:GITHUB_TOKEN
ttm runner config mcp allow dev --mcp github-local
```

### runner config claude-token

```text
printf %s "$TOKEN" | ttm runner config claude-token
ttm runner config claude-token --remove
```

将 `claude setup-token` 得到的 Claude Code OAuth token（从 stdin 读取）保存到 keyring，用于同事的任务：每次运行使用单独的
Claude 配置目录。有 token 时 runner 会声明 `team_isolation` 功能。参见 [Runner](runner.md)。

`ttm runner status` 还会显示 `features` 和本地 MCP 表（名称、allow、team jobs）。

`status` 的 JSON：`runner_id`、`name`、`running`、`pid`、`capacity`、`accept_team_jobs`、`harnesses`、`repos`、
`worktree_root`、`features`、`local_mcp`。`config` 的：`runner.toml` 的内容。`down` 的：`stopped`、`pid`。`install-service` 的：`installed`、`path`（服务的 `PATH`；Windows 上为 `null`）。
`uninstall-service` 的：`removed`（未安装时为 `null`）。

## agent

| 命令 | 选项 | 作用 |
| --- | --- | --- |
| `ttm agent run <KEY>` | | 立即在本机运行任务并打印事件；仅当 `succeeded` 时以 0 退出 |
| `ttm agent logs <RUN>` | `-f`, `--follow` | 该 run 的事件；`-f` 每 2 秒轮询一次 |
| `ttm agent stop <RUN>` | | Kill switch |

`run` 的 JSON：`run_id`、`status`、`branch`、`worktree`、`pr_url`、`paused_until`。`logs --json` 每个事件输出一行
JSON（`id`、`at`、`kind`、`message`、`data`）。面向人的 `logs`：`10:00:02 › edited src/login.rs`（按 `kind` 显示符号：`·`
thought、`›` action、`◆` mcp_call、`✕` error、`NN%` progress）。

### Agent 管理

需要 `agent_v2` 功能；所有命令都接受 `--workspace <ID>`（默认当前工作区）。`<AGENT>` 为 handle（`dev`、`@dev`）、名称或 id。
读取需要 `factory:read`；修改需要 `factory:admin`（工作区的 owner/admin）。

| 命令 | 说明 |
| --- | --- |
| `ttm agent list` | 表格：🤖 名称 + 所有者徽章、`@handle`、角色、模型、skill（`◐` = 有版本待审核）、MCP、状态 |
| `ttm agent show <AGENT>` | 标题 + git 身份、简介、skill、MCP、项目 |
| `ttm agent create [--preset <CODE>] [字段]` | 终端中为向导（预设或逐项提问）；脚本使用 `--yes` + 标志 |
| `ttm agent edit <AGENT> [字段] [--git-email <E>] [--git-name <N>] [--bio <TEXT>]` | 仅通过标志；`""` = 恢复默认 |
| `ttm agent pause\|resume\|archive <AGENT>` | 需要确认（`--yes`） |
| `ttm agent skills add <AGENT> <SLUG>[@<V>]` / `skills rm <AGENT> <SLUG>` | 添加（用 `@V` 固定版本）/ 移除 skill |
| `ttm agent mcp add <AGENT> <SERVER\|local:NAME> [--tools a,b\|*] [--required]` / `mcp rm <AGENT> <SERVER>` | 授予 / 移除 MCP；`--required` = 只在能提供该本地 MCP 的 runner 上运行 |

`create`/`edit` 的字段：`--name`、`--handle`、`--role`、`--model`（`opus`、`sonnet`、`haiku` 或 id）、`--budget`（每次运行的
token）、`--daily-budget`、`--turns`、`--permission`（`read`、`edit`、`full`、`custom`）、`--skills a,b`、
`--instructions-file <FILE>`（`-` = stdin）。

```text
3 agent(s)
   NAME              HANDLE  ROLE  MODEL   SKILLS        MCP  STATUS
🤖 DEV · Code ✦ Mei-er @dev  DEV   sonnet  ttm-tdd ◐     1    active
```

## workflow

需要 `workflow_v2`（`validate --file` 需要 `ascode_v2`）。`--project <KEY>` 默认为已关联的项目。修改需要 `factory:admin`。

| 命令 | 说明 |
| --- | --- |
| `ttm workflow show [--version <N\|draft>]` | 标题（名称 · vN · draft?）、状态表（按类别的符号、名称、角色、自动分配 🤖/👤、WIP）、流转表（从 → 到、guard、⚡ 规则数） |
| `ttm workflow validate [--file .ttm/workflow.yaml]` | 有 `--file`：在服务器上检查 as-code 文件；没有：检查草稿。每个问题一行 `✕ <rule\|code> <field>: <message>`；**有问题时退出码 1** |
| `ttm workflow simulate <TASK> --to <STATUS> [--version <V>] [--roles a,b]` | 每一步 ✓/✕ + 原因；被阻止时退出码 1 |
| `ttm workflow diff [<FROM> <TO>]` | 默认为项目所用版本与草稿；状态、流转、规则的 `+`/`-`/`~` |
| `ttm workflow publish [--note <TEXT>]` | 显示差异 → 确认 → 把草稿发布为新版本（`note` ≤ 280） |
| `ttm workflow upgrade [--to <N>]` | 显示预览（会移动的任务）→ 确认 → 升级项目 |
| `ttm workflow copy [--name <NAME>]` | 复制为工作区中可编辑的 workflow |

```text
OK: AUR-12 can move to qc
  ok  doing -> qc  (transition t3, 1 rule)
```

## automation

项目的自动化规则（需要 `workflow_v2`；`--project <KEY>`）。`enable`/`disable` 需要 `factory:admin`。

| 命令 | 说明 |
| --- | --- |
| `ttm automation list` | 表格：`●`/`⏸` 开/关、名称、触发器摘要、7 天内运行次数 |
| `ttm automation show <RULE>` | 以树形显示 When / If / Then |
| `ttm automation test <RULE> --task <KEY>` | 试运行（不做任何修改）：每个动作 ✓ 会执行 / ✕ 跳过 + 原因 |
| `ttm automation enable\|disable <RULE>` | 开 / 关 |
| `ttm automation runs [<RULE>] [--limit 20]` | 最近的运行 |

`<RULE>`：id、slug 或名称。

## skill

工作区的 skill（需要 `agent_v2`；`--workspace <ID>`）。runner 如何安装 skill 见 [Runner](runner.md)。

| 命令 | 说明 |
| --- | --- |
| `ttm skill list [--tag <T>] [--builtin]` | skill 表格（`--builtin` 包含内置 skill） |
| `ttm skill push <DIR> [--slug <S>] [--note <TEXT>]` | 本地检查（SKILL.md + front matter、≤ 5 MB、≤ 200 个文件、安全路径），显示清单 + sha256，没有时创建 skill（`--slug` 默认为目录名），上传确定性 zip → `v<N> pending review` 或 `v<N> ready`。无变化 → `No changes` |
| `ttm skill pull <SLUG> [--version <V>] [--dir <DIR>]` | 下载、校验 sha、解压（已关联仓库中默认 `.ttm/skills/<slug>`，否则 `./<slug>`） |
| `ttm skill show <SLUG> [--version <V>]` | 版本与审核状态 |
| `ttm skill review list` | 待审核队列 |
| `ttm skill review approve <SLUG>@<V> [--note <TEXT>]` | 显示与当前版本的差异 → 确认 → 批准 |
| `ttm skill review reject <SLUG>@<V> --note <TEXT>` | 拒绝（需要备注） |
| `ttm skill import --git <URL> --path <P> [--ref main] [--slug <S>]` | 从 GitHub/GitLab 导入 |

```text
OK: ttm-tdd v3 pending review
  sha256 3c9e7159… · 3 file(s) · 1.2 KB
Next: ttm skill review approve ttm-tdd@3
```

## mcp-server

工作区的远程 MCP 服务器，agent 通过服务器的网关使用（需要 `agent_v2`；`--workspace <ID>`）。

| 命令 | 说明 |
| --- | --- |
| `ttm mcp-server list` | 表格：名称、URL（截断）、认证、状态、工具数、已连接（workspace/me） |
| `ttm mcp-server add [--name] [--url] [--transport http\|sse] [--auth none\|workspace\|delegator] [--secret-header <H>] [--secret-env <VAR>]` | 向导；密钥只能通过 `--secret-env VAR` 或终端中隐藏输入，永远不作为标志值 |
| `ttm mcp-server connect <NAME> [--for workspace\|me] [--no-browser]` | 打开 OAuth 页面（或只打印 URL），等待连接完成（每 2 秒查询一次，最多 5 分钟） |
| `ttm mcp-server remove <NAME>` | 确认，并提醒正在使用的 agent |

## cr

变更请求：计划（来自 `ttm sync`）和配置（workflow、规则、agent、skill；需要 `ascode_v2`）。

| 命令 | 选项 | 说明 |
| --- | --- | --- |
| `ttm cr list` | `--project <KEY>`、`--kind plan\|workflow\|rule\|agent\|skill`、`--status pending\|applied\|rejected\|all`（默认 `pending`） | 表格：短 id、类型、状态、作者 + 徽章、文件数、时长 |
| `ttm cr show <ID>` | | 标题 + 每个文件的差异 |
| `ttm cr approve <ID>` | | 确认后应用（服务器期间已改变时返回 409 `stale`） |
| `ttm cr reject <ID>` | `--note <TEXT>` | 拒绝 |

## inbox

```text
ttm inbox [--unread] [--kind factory,…] [--watch] [--interval 5]
ttm inbox read [<ID>… | --all]
```

你的通知：`●` 未读、类型/原因、内容（截断）、时长、链接的表格。`--watch` 先打印列表，然后每 `--interval` 秒（至少 2 秒）查询一次，
每条新通知一行；`Ctrl-C` 平稳退出；加 `--json` 时输出 NDJSON。

```text
● 14:02 skill_review  Skill ttm-tdd v3 waiting for review → https://toantranmei.com/factory/skills/…
```

## template

公开模板库（需要 `templates`）。

| 命令 | 说明 |
| --- | --- |
| `ttm template list [--kind workflow\|agent\|skill\|rule-pack\|bundle] [--query <Q>]` | 搜索模板 |
| `ttm template publish --from workflow\|agent\|skill <SOURCE> [--slug] [--name] [--price <VND>] [--workspace]` | 导出预览（显示被排除的内容）→ 确认 → 发布。价格在网页上设置：命令会打印 `/me/items/<id>` 链接 |
| `ttm template install <SLUG> [--workspace <ID>]` | 预览（将创建：状态、规则、agent、skill + “runs code” 标记）→ 确认 → 安装；有待审核的 skill 时 `Next: ttm skill review list` |

## mcp

### mcp install

```text
ttm mcp install --client <CLIENT> [--name <NAME>] [--command <PATH>] [--toolsets <TOOLSETS>] [--dry-run]
```

| 选项 | 默认值 | 作用 |
| --- | --- | --- |
| `--client <CLIENT>` | （必填） | `claude-code`、`claude-desktop`、`cursor`、`codex` |
| `--name <NAME>` | `ttm` | 客户端配置中的服务器名称 |
| `--command <PATH>` | `ttm` 旁边的 `ttm-mcp`，其次是 `PATH` 上的 | 客户端运行的命令 |
| `--toolsets <TOOLSETS>` | | 向 `ttm-mcp` 传递 `--toolsets` |
| `--dry-run` | | 只打印，不写入 |

显式传入的 `--host`/`--profile`（或 `TTM_HOST`）会被固定写入 `args`。配置文件会被合并并备份
（`<file>.bak-<unix 秒数>`）。JSON：`client`、`file`、`backup`、`changed`、`dry_run`、`entry{name, command, args}`。详情：
[连接 MCP](mcp.md)。

## self-update

```text
ttm self-update [--check] [--version <VERSION>]
```

| 选项 | 作用 |
| --- | --- |
| `--check` | 只报告是否有新版本 |
| `--version <VERSION>` | 安装指定版本（例如 `0.2.0`），即使比当前版本旧 |

从 GitHub Releases 下载，校验 SHA-256 以及（有 `gh` 时）attestation，替换同一目录中的 `ttm` 和 `ttm-mcp`。JSON：`current`、
`latest`、`update_available`、`updated`、`attestation`。

## completion

```text
ttm completion <SHELL>
```

`<SHELL>`：`bash`、`elvish`、`fish`、`powershell`、`zsh`。将脚本打印到 stdout；不读取配置。参见
[安装](cai-dat.md)。

## ttm-mcp

```text
ttm-mcp [--host <URL>] [--profile <NAME>] [--toolsets <LIST>] [-v...]
```

通过 stdio 运行的 MCP 服务器。参见 [连接 MCP](mcp.md) 和 [MCP 工具](mcp-cong-cu.md)。
