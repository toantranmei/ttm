---
title: 命令参考
description: ttm 的全部命令与子命令，包括语法、选项、默认值、示例、JSON 输出和退出码。
order: 10
section: reference
---

```text
ttm [OPTIONS] <COMMAND>
```

`ttm --help` 和 `ttm <命令> --help` 会打印原始帮助信息（英文）。本页对应版本 **0.1.6**。

## 全局选项

适用于所有命令，放在子命令之前或之后均可（`ttm whoami --json` = `ttm --json whoami`）。

| 选项 | 环境变量 | 作用 |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | 服务器，例如 `http://localhost:3000` |
| `--profile <NAME>` | `TTM_PROFILE` | `config.toml` 中的 profile |
| `--json` | | 在 stdout 上以 JSON 输出结果 |
| `-v`, `--verbose` | `TTM_LOG` | 向 stderr 输出更多日志（`-v` 为 info，`-vv` 为 debug） |
| `-h`, `--help` | | 帮助 |
| `-V`, `--version` | | 版本（`ttm 0.1.6`；仅在顶层可用） |

## 退出码

| 代码 | 何时 |
| --- | --- |
| `0` | 成功 |
| `1` | 出错（网络、认证、API、文件……）；`ttm doctor` 有 `FAIL` 项；`ttm sync --check` 发现不一致；`ttm sync` 遇到冲突；`ttm agent run` 以 `succeeded` 以外的状态结束 |
| `2` | 命令行语法错误（未知选项、缺少参数）。对于 `ttm hook …`：操作被规则阻止 |

出错时总会向 stderr 打印 `error: <消息>`；使用 `--json` 时，还会向 stdout 输出 `{"error": {"code", "message"}}`（参见
[配置](cau-hinh.md) 和 [错误码](ma-loi.md)）。

## 概要

| 命令 | 作用 |
| --- | --- |
| [`login`](#login) | 登录（浏览器、`--device`、`--token`） |
| [`logout`](#logout) | 撤销 token 并从本机删除 |
| [`whoami`](#whoami) | 当前登录的用户 |
| [`status`](#status) | 正在使用的 host、profile、token、工作区 |
| [`doctor`](#doctor) | 检查环境 |
| [`workspace`](#workspace) | `list`、`switch`、`current` |
| [`token`](#token) | `list`、`create`、`revoke`（打开网页） |
| [`notes`](#notes) | `search`、`get`、`query`、`comments`、`create`、`edit`、`comment`、`rows`、`suggestions`、`resolve` |
| [`project`](#project) | `init`、`link`、`open`、`status` |
| [`sync`](#sync) | 同步计划 |
| [`task`](#task) | `list`、`show`、`claim`、`done`、`approve`、`split` |
| [`rules`](#rules) | `apply` |
| [`hook`](#hook) | `pre-bash`、`stop`（由 agent 调用） |
| [`runner`](#runner) | `up`、`down`、`status`、`config`、`install-service`、`uninstall-service` |
| [`agent`](#agent) | `run`、`logs`、`stop` |
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
| `--scope <SCOPE>` | `workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run offline_access` | 申请的 scope，以空格分隔（不适用于 `--token`） |
| `--no-browser` | | 不打开浏览器，只打印 URL |

```sh
ttm login
ttm login --device
printf %s "$PAT" | ttm login --token -
ttm --profile dev login --host http://localhost:3000 --scope "workspace:read notee:read"
```

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

JSON：`host`、`profile`、`method`（`browser` | `device` | `token`）、`stored_in`（`keyring` | `file` | `env`）、`user`
（token 缺少 `workspace:read` 时为 `null`）。

## logout

```text
ttm logout
```

在服务器上撤销访问 token 和 refresh token，然后从 keyring 和 `credentials.toml` 中删除。即使服务器
未确认，也会删除本地副本。无法清除 `TTM_TOKEN`（只给出警告）。

```text
Signed out of https://toantranmei.com (profile `default`).
```

JSON：`host`、`profile`、`revoked`（服务器已确认）、`removed`（有 token 从本机删除）。

## whoami

```text
ttm whoami
```

调用 `/me`（需要 `workspace:read`）。

```text
An Nguyễn <an@example.com>
host: https://toantranmei.com  profile: default
workspace: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

JSON：`host`、`profile`、`user`（`id`、`email`、`name`、`avatar_url`、`locale`、`plan`）、`workspace`（`id`、`name`、
`kind`、`role`、`slug` 或 `null`）。

## status

```text
ttm status
```

显示本机将要使用的设置；无需登录或联网（始终以 0 退出）。

```text
ttm 0.1.6
host:      https://toantranmei.com
profile:   default (/Users/an/.config/ttm/config.toml)
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
workspace: not chosen (ttm workspace switch)
server:    API v1 (min ttm 0.1.0), features: notee, factory
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
[ok  ] host       https://toantranmei.com (profile default)
[ok  ] discovery  api_base https://toantranmei.com/api/v1
[ok  ] version    ttm 0.1.6, server API v1
[ok  ] token      ttm_oat_Ab12… (keyring) as an@example.com
[ok  ] keyring    OS keyring available
[ok  ] git        git version 2.50.1
[warn] claude     Claude Code not found on PATH (needed by ttm runner)
[ok  ] ttm-mcp    /Users/an/.cargo/bin/ttm-mcp
```

JSON：`{name, level, detail}` 数组。

## workspace

### workspace list

```text
ttm workspace list
```

列出你所属的工作区；`*` 标记 profile 的当前工作区。

```text
* Cá nhân  personal · owner · 0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
  Team Aurora [aurora]  team · member · 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

JSON：`{id, name, kind, role, slug?, current}` 数组。

### workspace switch

```text
ttm workspace switch <WORKSPACE>
```

为 profile 设置当前工作区。`<WORKSPACE>`：id、slug 或名称（精确匹配，不区分大小写），或者
名称的前缀（仅当只匹配一个工作区时）。匹配到多个时：`matches several workspaces (…); use the id`。

```text
Current workspace for profile `default`: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

### workspace current

```text
ttm workspace current
```

打印名称和 id（仅当服务器能响应时才有名称）。JSON：`profile`、`workspace_id`、`workspace`。

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
Created Ghi chú họp (https://toantranmei.com/notes/0190f3c4-…)
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
ttm project init --key <KEY> [--name <NAME>] [--repo-url <URL>] [--workspace <ID>]
```

在服务器上为当前仓库创建项目，然后进行关联（同 `link`）。需要 `factory:write`。

| 选项 | 默认值 |
| --- | --- |
| `--key <KEY>` | （必填）任务 key 的前缀，会转为大写 |
| `--name <NAME>` | 仓库根目录的名称 |
| `--repo-url <URL>` | `git remote get-url origin` |
| `--workspace <ID>` | 当前工作区 |

JSON：`project.toml` 的内容。

### project link

```text
ttm project link <PROJECT>
```

将仓库与已有项目（key 或 id）关联，写入 `.ttm/project.toml`、`.ttm/plan`、`.ttm/base`、`.ttm/.gitignore`。

```text
Linked /Users/an/code/aurora to AUR (Aurora), plan rev 42.
Commit .ttm/project.toml and .ttm/plan; run `ttm rules apply` to install the project rules.
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
AUR-12     doing          DEV       Dev bot          Đăng nhập bằng Google
```

```text
AUR-12  Đăng nhập bằng Google
state: doing  role: DEV  assignee: Dev bot

Cho phép đăng nhập bằng tài khoản Google.
```

其他输出：`AUR-12 is now yours.`、`AUR-12 is now done.`、`AUR-12 approved; state done.`、`AUR-12 rejected; state doing.`（`--reject`）、
`2 subtask(s) created under AUR-12.` JSON：任务对象（camelCase，`id` 为 key），`split` 则为 `{tasks, warnings}`。

## rules

### rules apply

```text
ttm rules apply
```

将 `<!-- ttm:begin -->…<!-- ttm:end -->` 块写入 `AGENTS.md` 和 `CLAUDE.md`，把 hook 合并到 `.claude/settings.json`，并保存
`.ttm/cache/rules.json`。打印已更改的文件，或 `Rules already up to date.` JSON：`changed`、`checks`。

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

`status` 的 JSON：`runner_id`、`name`、`running`、`pid`、`capacity`、`accept_team_jobs`、`harnesses`、`repos`、
`worktree_root`。`config` 的：`runner.toml` 的内容。`down` 的：`stopped`、`pid`。`install-service` 的：`installed`、`path`（服务的 `PATH`；Windows 上为 `null`）。
`uninstall-service` 的：`removed`（未安装时为 `null`）。

## agent

| 命令 | 选项 | 作用 |
| --- | --- | --- |
| `ttm agent run <KEY>` | | 立即在本机运行任务并打印事件；仅当 `succeeded` 时以 0 退出 |
| `ttm agent logs <RUN>` | `-f`, `--follow` | 该 run 的事件；`-f` 每 2 秒轮询一次 |
| `ttm agent stop <RUN>` | | Kill switch |

`run` 的 JSON：`run_id`、`status`、`branch`、`worktree`、`pr_url`、`paused_until`。`logs --json` 每个事件输出一行
JSON（`id`、`at`、`kind`、`message`、`data`）。

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
