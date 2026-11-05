---
title: Runner
description: 在你的机器上运行 Factory 的 agent：仓库配置、并发容量、后台服务、ttm agent、Claude Code 的要求、token 与安全模型。
order: 9
section: guides
---

**Runner** 就是 `ttm runner up`：开发机上的一个进程，领取 Factory 服务器分派的作业（job），在每个任务独立的 git worktree 中用你的
Claude 账号运行 coding agent（**Claude Code**），把活动实时发送到 Execution Map，然后推送分支并创建 pull request。分派什么任务、用哪个模型、prompt
怎么写都由服务器决定；runner 只负责执行。

## 要求

| 需要 | 原因 |
| --- | --- |
| `ttm` 和 `ttm-mcp` 位于同一目录（或 `ttm-mcp` 在 `PATH` 中） | agent 使用 `factory-*` 和 `notee-*` MCP 工具 |
| 以 `factory:run` scope 登录（`ttm login` 的默认值） | 注册 runner、领取作业 |
| `git` | worktree、提交、推送 |
| [Claude Code](https://docs.anthropic.com/claude-code)（`claude` 在 `PATH` 中），已登录 Claude 订阅或设置了 `ANTHROPIC_API_KEY` | `claude-code` harness |
| 已执行 `gh auth login` 的 `gh`（GitHub CLI）（可选） | 推送分支后创建 pull request |
| 项目仓库的一个 clone，带有 `origin` remote | runner 只在你声明过的 clone 中工作 |

`ttm doctor` 会检查 `git`、`claude` 和 `ttm-mcp`。

## 开始

```sh
ttm runner config --repo AUR=~/code/aurora --max-parallel 2
ttm runner up
```

```text
Runner 0190f500-… up: 2 agent(s) at once. Ctrl-C to stop.
```

在网页上把任务分配给 agent（或执行 `ttm agent run AUR-12`），runner 会在几秒内领取作业。可以在 Execution Map 上或用
`ttm agent logs <run> -f` 跟踪进度。

## 配置：`ttm runner config`

```sh
ttm runner config --name "macbook-an" --max-parallel 2 --keep-days 7 \
  --worktree-root ~/orca/workspaces --repo AUR=~/code/aurora --repo WEB=~/code/web
ttm runner config --accept-team-jobs true --auto-merge false
```

```text
Saved /Users/an/.config/ttm/runner.toml
```

| 标志 | 默认值 | 作用 |
| --- | --- | --- |
| `--name <NAME>` | 机器名（`HOSTNAME`/`COMPUTERNAME`/`hostname`） | 网页上显示的名称；以相同名称重新注册会保留 runner id |
| `--max-parallel <N>` | 根据机器计算 | 同时运行的 agent 数量（≥ 1） |
| `--accept-team-jobs true\|false` | `false` | 也领取队友任务的作业，而不只是你自己的任务 |
| `--worktree-root <DIR>` | `config.toml` 的 `worktree_root`，其次是 `~/.ttm/worktrees` | 创建 worktree 的位置 |
| `--keep-days <N>` | `3` | 已完成作业的 worktree 保留天数 |
| `--repo KEY=PATH` | | 项目 `KEY` 的本地 clone（可重复；key 转为大写，路径会规范化，`~/` 会展开） |
| `--auto-merge true\|false` | `false` | 自动合并合并队列中的第一项并推送 |

只有你传入的标志才会改变；其他值保持不变。`runner.toml` 文件与 `config.toml` 位于同一目录：

```toml
name = "macbook-an"
max_parallel = 2
accept_team_jobs = false
worktree_root = "~/orca/workspaces"
keep_days = 7
runner_id = "0190f500-…"     # 由首次注册写入
auto_merge = false

[repos]
AUR = "/Users/an/code/aurora"
WEB = "/Users/an/code/web"

[mcp.local.github-local]                         # 本地 MCP（0.2.0），见下文
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }
allow = ["dev", "reviewer"]
team_jobs = false
```

重新执行 `ttm runner up`（或重启服务）以应用新配置。`[mcp.local.*]` 在每个作业开始时都会重新读取，无需重启。

### 为什么必须声明仓库

Runner **永远不会 clone 服务器发来的 URL**。每个作业都会注明它的仓库；runner 只在以下条件都满足时运行：

1. 作业所属项目在 `[repos]` 中有 clone（按 key，不区分大小写，或按 id）；
2. 该 clone 的 `origin` 与作业的仓库一致（规范化后比较：去掉协议、用户、端口、`.git`、末尾的 `/`；例如
   `git@github.com:a/b.git` = `https://github.com/a/b`）；对于 `local` 项目，作业的路径必须与声明的路径一致；
3. 作业的分支名符合 git 的规则；
4. worktree 位于 `worktree_root` 之下：作业的 `worktree_hint` 只能是相对路径；绝对路径（`/…`、`C:\…`）、含 `..`、`\` 或
   `:` 的路径，或经由符号链接指向 `worktree_root` 之外的路径都会被拒绝。

不满足时，作业会被拒绝并上报服务器（`repo_not_configured`、`invalid_branch` 或 `invalid_worktree`），终端打印：

```text
ttm runner: refused AUR-12: no local clone is configured for project AUR; add it with `ttm runner config --repo AUR=<path to your clone>`
```

这样，即使账号被盗或出现伪造的作业，也无法让你的机器从陌生的地方下载并运行代码。

### 并发容量

同时运行的 agent 数 = 设置了 `--max-parallel` 时取其值；否则为 `clamp(CPU 核数 / 4, 1, 3)`，并会下调以保证每个 agent 至少有
**4 GB 空闲内存**。服务器可能按套餐设置更低的上限（每个 runner：Free 1、Pro 3、Team 6）。当 agent 报告 Claude 的用量额度已用完时，runner
会**暂停领取作业**直到额度重置（该 run 以 `paused` 状态结束并附带 `paused_until`），服务器会重新排队，而不是不停重试。

## 本地 MCP：`ttm runner config mcp`

本地 MCP 是运行在**你自己机器上**的 MCP 服务器（例如带你自己 token 的 GitHub 服务器）。默认**任何 agent 都不能**使用：你需要按
handle 逐个允许 agent。

```sh
ttm runner config mcp add github-local --command npx --arg -y --arg @modelcontextprotocol/server-github \
  --env GITHUB_TOKEN=env:GITHUB_TOKEN
ttm runner config mcp allow dev --mcp github-local     # 不带 --mcp：所有本地 MCP
ttm runner config mcp deny dev --mcp github-local
ttm runner config mcp team-jobs github-local on
ttm runner config mcp remove github-local
ttm runner config mcp list
```

| 键 | 含义 |
| --- | --- |
| 名称（`[mcp.local.<名称>]`） | `^[a-z0-9][a-z0-9-]{0,31}$`，不能是 `ttm`（runner 自己的 MCP 服务器） |
| `command`、`args` | 启动服务器的命令 |
| `env` | `K = "env:VAR"`：**运行时**读取 runner 的环境变量 `VAR`（真实值不会写进文件）；`K = "值"` 仅用于非机密值。`--env` 会拒绝看起来像 token 的字面值 |
| `allow` | 允许使用的 agent handle |
| `team_jobs` | `true`：别人委派的作业也能使用（默认 `false`） |

作业获得本地 MCP 的条件：名称在 `job.mcp.local` 中、作业的 agent 在 `allow` 中，且作业不是同事的作业或 `team_jobs = true`。缺少
`env:VAR` 时该 MCP 被跳过（附警告事件）；名称为 `ttm` 或与远程 MCP 重名也会被跳过。正如 `ttm runner config mcp list` 所示：**只有
名称、允许的 agent 和 `team_jobs` 会上报给服务器**；命令、参数和环境变量从不离开本机。这些命令用 `toml_edit` 修改 `runner.toml`，
保留你的注释。

### 同事作业的 Claude token：`ttm runner config claude-token`

```sh
claude setup-token | ttm runner config claude-token     # 从 stdin 读取 token，存入 keyring
ttm runner config claude-token --remove
```

参见[同事作业的隔离](#同事作业的隔离)。

## 作业如何运行

1. **检查**仓库和分支（见上一节）。
2. **Worktree**：`git fetch origin <主分支>`，然后 `git worktree add -B <分支> <路径> origin/<主分支>`。路径为
   `<worktree_root>/<worktree_hint>`（服务器建议 `<小写 key>/<去掉 ttm/ 的分支>`，例如 `aur/aur-12-dang-nhap-google`）；
   没有建议的作业使用 `<worktree_root>/<KEY>/<分支>`（特殊字符替换为 `-`）。
3. **规则**：在 worktree 中运行 `ttm rules apply`（`AGENTS.md`/`CLAUDE.md` 区块、Claude Code hook）。
4. **MCP**：写入 `<cache>/mcp-<run>.json`（权限 0600），指向 `ttm-mcp --toolsets <作业的工具组，默认 factory,notee>`，其中
   `TTM_TOKEN` 是作业的 token，另有 `TTM_RUN_ID`、`TTM_TASK`、`TTM_PROJECT`。
5. **Agent**：在 worktree 中运行 `claude -p <prompt> --output-format stream-json --verbose --mcp-config <file> [--model <m>]
   [--max-turns <n>] [--allowedTools <列表>]`；模型、轮数和允许的工具由作业决定。最长时间由作业决定，默认 **120 分钟**。
6. **事件**：每一行 stream-json 都会变成一个事件，按批发送（每秒一次或每 25 个事件一次）：

   | Claude Code | 事件 |
   | --- | --- |
   | 启动 | `log`（`agent started <model>`） |
   | agent 的文本 | `thought` |
   | 调用工具 | `action`（`TodoWrite` → `plan`） |
   | 最终结果 | `response`，附带输入/输出 token 数 |
   | 测试命令（`cargo test`、`pnpm test`、`pytest`、`go test`、`vitest`、`playwright test` 等） | `log` `tests passed`/`tests failed`；结果作为 `test_result` 附加到 run 上 |

7. **结束**：如果成功，提交 agent 留下的改动（`chore(<key>): changes from agent run <run>`），推送分支，装有 `gh` 时用
   `gh pr create` 创建 PR（或沿用该分支已打开、已被推送更新的 PR），然后报告状态（`succeeded`、`failed`、`cancelled`、`paused`）、提交、PR 和已使用的 token。
   PR 标题为 `<KEY>: <任务标题>`（最多 72 个字符），agent 的总结作为 PR 正文。临时 MCP 文件会被删除。

从 0.2.0 起，在规则与 agent 之间，runner 还会安装作业的 skill、准备本次 run 专用的 git hook，并把远程和本地 MCP 写入 MCP 文件；
Claude Code 以 `--strict-mcp-config`（agent 只看到这些服务器）和 `--settings <cache>/settings-<run>.json` 运行（见下文）。

已完成作业的 worktree 会保留 `keep_days` 天供你查看，然后在下一次 `ttm runner up` 启动时删除。

每个 run 在开始和结束时各输出一行（stderr）：

```text
● AUR-12 running · sonnet · worktree /Users/an/orca/workspaces/aur/aur-12-dang-nhap-google
✓ AUR-12 succeeded → https://github.com/an/aurora/pull/7
✕ AUR-13 refused: repo_not_configured add it with `ttm runner config --repo AUR=<path to your clone>`
```

### Git 身份与 `commit-msg` hook

- 带 `git` 的作业（agent v2）以 **agent** 的名字和邮箱提交（`GIT_AUTHOR_*`、`GIT_COMMITTER_*`），例如
  `DEV · Code <dev+aurora@agents.toantranmei.com>`。不带 `git` 的作业（人的任务）保持本机的 git 身份。
- **所有作业**，无论 `commit_identity` 模式（`agent`、`delegator`、`delegator_with_trailer`），git 都以 `core.hooksPath` 指向本次
  run 专用的 hook 目录 `<cache>/hooks/<run>/` 运行（通过 `GIT_CONFIG_COUNT/KEY/VALUE`，不修改任何配置文件）。其中的 `commit-msg`：
  1. 删除所有匹配（不区分大小写）`^co-authored-by:.*(claude|anthropic)`、`generated (with|by) .*claude`、`🤖 generated with`、
     `noreply@anthropic.com` 的行；
  2. 添加作业的 trailer（仅 `delegator_with_trailer` 模式：`Co-authored-by: <agent 名> <agent 邮箱>`，只加一次）；
  3. 运行仓库原有的 `commit-msg`（来自 `core.hooksPath` 或 `.git/hooks`）。其他原有 hook 通过 shim 串接。
- Claude Code 始终带 `--settings` 运行，内容为 `{"includeCoAuthoredBy": false, "attribution": {"commit": "", "pr": ""}}`。
- Claude Code 还会收到带 runner 说明的 `--append-system-prompt`：由 runner 推送分支并创建 PR，agent 不运行 `git push`
  或写入 PR/issue 的 `gh` 命令（只读 `gh` 可以），总结中不提 PR；评审 run 通过 factory 工具给出结论。
- runner 自己提交剩余改动和推送时使用相同的环境。run 结束后删除 hook 目录。

规则：runner 生成的任何提交都不包含 Claude/Anthropic 行。

### Skill

作业可以携带**已审核**的 skill（`job.skills`：slug、版本、10 分钟有效的签名下载 URL、SHA-256、大小）。runner 会：

1. 有缓存 `<cache>/skills/<sha256>.zip` 时直接使用，否则下载（60 秒，超出声明大小或 5 MB 即停止）；
2. 计算 SHA-256；不一致则以 `skill_hash_mismatch` **拒绝作业**；
3. 安全解压（不允许 `..`、不允许符号链接、≤ 200 个文件、≤ 5 MB）到 `<worktree>/.claude/skills/ttm-<slug>/`；若 git 已跟踪该目录则
   跳过该 skill（警告）；
4. 向仓库的 `info/exclude` 添加一次 `/.claude/skills/ttm-*/`，使 skill 永远不会被提交；
5. run 结束后删除已安装的目录，无论成功、失败、取消还是超时。

### 经网关的 MCP 与本地 MCP

- 工作区的**远程 MCP**（`ttm mcp-server …`）：作业获得指向服务器网关（`/api/v1/mcp/gw/<server>`）的 `{name, url}`，用作业 token
  调用。MCP 服务器的密钥（OAuth token、密钥 header）留在服务器上，永远不会到达你的机器。
- **本地 MCP**：见[本地 MCP](#本地-mcpttm-runner-config-mcp)。

### 同事作业的隔离

同事的作业（`team_job`：委派者不是 runner 的所有者）不能看到你个人的 Claude 配置：

- 始终带 `--setting-sources project,local` 运行（不读取你的 `~/.claude/settings.json`）；
- 用 `ttm runner config claude-token` 保存 token 后，还会为每次 run 使用空的 `CLAUDE_CONFIG_DIR`，并把 `CLAUDE_CODE_OAUTH_TOKEN`
  设为该 token，因此看不到你的 hook、插件或个人 skill（`~/.claude/skills`）；
- 只有保存了 token，runner 才上报 `team_isolation`。没有 token 时 runner **拒绝**所有同事作业（`unsupported_feature`，上报服务器，
  绝不使用本机的 Claude 配置运行），`ttm runner status` 会提示需要运行的命令。

Spike 结果（Claude Code 2.1.318，仅阅读 `claude --help`）：存在 `--setting-sources`，但帮助没有说明是否排除插件和个人 skill，因此
仅靠该标志**不够**；所以另外使用空的 `CLAUDE_CONFIG_DIR`。将在 M4 模拟阶段用真实作业确认。

### 功能（`features`）

注册时 runner 上报自己具备的功能：`git_identity`、`skills`、`mcp_gateway`、`mcp_local`，保存了同事作业的 Claude token 时再加
`team_isolation`；以及本地 MCP 列表（`name`、`allow`、`team_jobs`）。`runner.toml` 变化时 heartbeat 会重新发送本地 MCP 列表。
`requires` 中含有 runner 不具备的功能的作业会以 `unsupported_feature` 被拒绝。

### 与服务器的连接

- Runner 只建立**出站**连接（可以在 NAT 之后运行，无需开放端口）：每 20 秒发送一次 heartbeat，有空闲容量时 long-poll 作业
  （`?wait=25`，服务器最多保持 25 秒）。等待中的 long-poll 不会因 heartbeat 而被取消，因此服务器已分派的作业不会丢失。
- heartbeat 携带正在运行的 run 列表；服务器返回需要停止的 run（来自网页的 **kill switch** 或 `ttm agent stop`），或关闭 runner
  的指令（“turned off from the web”）。
- runner token 有效期 12 小时，在过期前 30 分钟自动续期（重新注册，旧 token 被撤销）。连续超过 3 次被 `401` 拒绝时，runner 会停止。

### 自动合并（`auto_merge`）

开启后，runner 每分钟取出 `[repos]` 中各项目合并队列的第一项，在临时 worktree `ttm/merge-<id>` 中把 `origin/<run 的分支>` 合并进主分支（`git merge --no-ff`，提交
`merge: <KEY> (<分支>)`），推送 `HEAD:<主分支>`，并报告 `merged` 或 `conflict`（服务器会为冲突创建 “resolve” 任务）。报告结果使用你的用户
token（需要 `factory:write`）。

## 监控与停止

```sh
ttm runner status
```

```text
● macbook-an running
     runner  0190f500-…
        pid  4242
   capacity  2
  team jobs  no
  harnesses  claude-code
  worktrees  /Users/an/orca/workspaces
   features  git_identity, skills, mcp_gateway, mcp_local

PROJECT  CLONE
───────────────────────────────
AUR      /Users/an/code/aurora
WEB      /Users/an/code/web
```

有本地 MCP 时会附加本地 MCP 表格（名称、命令、允许的 agent、team jobs）；同事作业尚未隔离时会给出警告。

```sh
ttm runner down     # 向 runner.pid 中的进程发送 SIGTERM（Windows：taskkill）
```

`Ctrl-C` 或 `ttm runner down` 会停止 runner；正在运行的 agent 会被取消。如果 runner 以服务方式运行，服务会重新启动它；要彻底关闭，请运行 `ttm runner uninstall-service`。

## 后台服务：`ttm runner install-service`

```sh
ttm runner install-service
ttm --profile work runner install-service     # 服务使用 work profile
```

| 操作系统 | 安装内容 | 日志 |
| --- | --- | --- |
| macOS | LaunchAgent `~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist`（`RunAtLoad`、`KeepAlive`），用 `launchctl load -w` 加载 | `<cache>/runner.log`（`~/.cache/ttm/runner.log`） |
| Linux | `~/.config/systemd/user/ttm-runner.service`（`Restart=on-failure`），`systemctl --user enable --now` | `journalctl --user -u ttm-runner` |
| Windows | 任务计划程序 `ttm-runner`，登录时运行（`schtasks /Create /SC ONLOGON`） | （没有日志文件） |

服务运行 `<ttm 路径> [--profile <名称>] runner up`；只有你显式传入 `--profile` 时才会写入它。

服务会带上 `PATH`：launchd 和 systemd 以最小的 `PATH`（`/usr/bin:/bin:…`）启动服务，其中没有 `claude`、`gh`，有时连 `git`
也没有。因此 plist（`EnvironmentVariables`）和 unit（`Environment="PATH=…"`）会写入运行 `install-service` 的 shell 的
`PATH`（去掉空项或 `.` 这类相对路径），并补上缺少的 `~/.local/bin`、`~/.cargo/bin`、`/opt/homebrew/bin`、`/opt/homebrew/sbin`、
`/usr/local/bin`、`/usr/bin`、`/bin`、`/usr/sbin`、`/sbin`。命令会打印写入的 `PATH`：

```text
Runner service installed: /Users/an/Library/LaunchAgents/com.toantranmei.ttm-runner.plist
PATH of the service: /Users/an/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/Users/an/.cargo/bin:/opt/homebrew/sbin:/usr/sbin:/sbin
```

之后若把工具装到别的目录，请在已有新 `PATH` 的 shell 中再次运行 `ttm runner install-service`（会覆盖并重新加载服务）。
在 Windows 上，任务以你账户的环境（Windows 中设置的 `PATH`）运行，无需额外操作。如果希望在你未登录时 runner 也能运行，
Linux 还需要 `loginctl enable-linger $USER`。

### 卸载服务：`ttm runner uninstall-service`

```sh
ttm runner uninstall-service
```

先停止服务再移除：macOS 执行 `launchctl unload -w` 并删除 plist；Linux 执行 `systemctl --user disable --now`、删除 unit、
`daemon-reload`；Windows 执行 `schtasks /End` 再 `schtasks /Delete /F /TN ttm-runner`。输出
`Runner service stopped and removed: <file>`，未安装时为 `No runner service installed.`。

## 立即运行一个任务：`ttm agent`

```sh
ttm agent run AUR-12
```

```text
Run 0190f610-… for AUR-12 on branch aur-12-dang-nhap-google
[log] worktree /Users/an/.ttm/worktrees/aur/aur-12-dang-nhap-google on aur-12-dang-nhap-google from main
[log] agent started claude-sonnet-4-5
[thought] Mình sẽ đọc context của task trước.
[action] mcp__ttm__factory-get-context
[action] Bash: cargo test -p auth
[log] tests passed: cargo test -p auth
[response] Đã thêm đăng nhập Google, test đều qua.
Run 0190f610-…: succeeded
worktree: /Users/an/.ttm/worktrees/aur/aur-12-dang-nhap-google
pull request: https://github.com/an/aurora/pull/57
```

`agent run` 会在需要时注册本机，向服务器为该任务申请一个作业（`factory:run`），使用 `runner.toml` 的配置立即在前台运行（仍然需要声明
`--repo`），打印事件，并且**只有 run 为 `succeeded` 时才以 0 退出**。`Ctrl-C` 会取消 run。任务已有其他 run 时：`409 conflict`（`rule: run-active`）。

```sh
ttm agent logs 0190f610-…            # 到目前为止的事件
ttm agent logs 0190f610-… -f         # 持续跟踪，每 2 秒轮询一次（Ctrl-C 退出）
ttm agent logs 0190f610-… --json     # 每个事件一行 JSON
ttm agent stop 0190f610-…            # kill switch
```

```text
2026-11-04T05:31:02Z [log] agent started claude-sonnet-4-5
2026-11-04T05:31:09Z [action] Bash: cargo test -p auth
```

`agent stop` 通知服务器停止 run；正在运行它的 runner 会在下一次 heartbeat（≤ 20 秒）时收到指令，结束 agent 进程并报告 `cancelled`。

## Runner 使用的 token

| Token | 用于 | 存储位置 |
| --- | --- | --- |
| 用户 token（`ttm login`） | 注册 runner、`ttm agent run`、报告合并结果（`auto_merge`） | keyring `<profile>@<host>`（或 `TTM_TOKEN`） |
| runner token（`ttm_job_`，12 小时） | heartbeat、领取作业、发送事件、结束 run | keyring `<profile>#runner@<host>`，永远不来自 `TTM_TOKEN` |
| 作业 token（`ttm_job_`） | agent 的 `ttm-mcp`，仅限该 run；MCP 网关 | 临时 MCP 文件（0600），run 结束后删除 |
| 同事作业的 Claude token（`claude setup-token`） | 同事作业的 `CLAUDE_CODE_OAUTH_TOKEN` | keyring `<profile>#claude-team` |

撤销 runner：在 `/me/ket-noi` 的 **Runner** 或 **Token** 标签页操作。

## 安全模型

- **只有出站连接**；不在你的机器上开放任何端口。
- **只使用你声明的仓库**，每个作业都检查 `origin`；不 clone 陌生 URL；分支名在使用前经过检查。
- **每个任务一个 worktree、一个分支**；你自己的 worktree 不会被改动。`pre-bash` hook 会阻止直接推送受保护分支、`--no-verify`
  以及不符合约定的提交。worktree 始终位于 `worktree_root` 之下。
- **作业 token** 只能在它所属的 run 中使用，随作业过期；它保存在 0600 文件中，run 结束后删除。
- **Agent 以你的操作系统用户权限运行**，受作业下发的允许工具列表约束。这不是沙箱：只为你信任的项目和工作区运行 runner，并在合并前审查 PR。
- **提交和 PR 中没有 Claude/Anthropic 行**（run 的 `commit-msg` hook、`--settings`）。
- **只用已审核的 skill**，按 SHA-256 校验，从不提交，run 结束后删除。
- **本地 MCP 默认关闭**，只对你允许的 agent 开放；服务器只知道名称。同事的作业看不到你个人的 Claude 配置。
- **运行作业的 agent 永远没有管理工具**（`factory:admin`）：作业的 MCP 去掉 `admin` 工具组。
- 每一步的最终 prompt 都会经过你的机器（用于运行 Claude Code）；服务器永远不会接收你的 Claude key。
