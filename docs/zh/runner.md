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
```

重新执行 `ttm runner up`（或重启服务）以应用新配置。

### 为什么必须声明仓库

Runner **永远不会 clone 服务器发来的 URL**。每个作业都会注明它的仓库；runner 只在以下条件都满足时运行：

1. 作业所属项目在 `[repos]` 中有 clone（按 key，不区分大小写，或按 id）；
2. 该 clone 的 `origin` 与作业的仓库一致（规范化后比较：去掉协议、用户、端口、`.git`、末尾的 `/`；例如
   `git@github.com:a/b.git` = `https://github.com/a/b`）；对于 `local` 项目，作业的路径必须与声明的路径一致；
3. 作业的分支名符合 git 的规则。

不满足时，作业会被拒绝并上报服务器（`repo_not_configured` 或 `invalid_branch`），终端打印：

```text
ttm runner: refused AUR-12: no local clone is configured for project AUR; add it with `ttm runner config --repo AUR=<path to your clone>`
```

这样，即使账号被盗或出现伪造的作业，也无法让你的机器从陌生的地方下载并运行代码。

### 并发容量

同时运行的 agent 数 = 设置了 `--max-parallel` 时取其值；否则为 `clamp(CPU 核数 / 4, 1, 3)`，并会下调以保证每个 agent 至少有
**4 GB 空闲内存**。服务器可能按套餐设置更低的上限（每个 runner：Free 1、Pro 3、Team 6）。当 agent 报告 Claude 的用量额度已用完时，runner
会**暂停领取作业**直到额度重置（该 run 以 `paused` 状态结束并附带 `paused_until`），服务器会重新排队，而不是不停重试。

## 作业如何运行

1. **检查**仓库和分支（见上一节）。
2. **Worktree**：`git fetch origin <主分支>`，然后 `git worktree add -B <分支> <路径> origin/<主分支>`。默认路径为
   `<worktree_root>/<KEY>/<分支>`（特殊字符替换为 `-`）。
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
   `gh pr create` 创建 PR，然后报告状态（`succeeded`、`failed`、`cancelled`、`paused`）、提交、PR 和已使用的 token。临时 MCP 文件会被删除。

已完成作业的 worktree 会保留 `keep_days` 天供你查看，然后在下一次 `ttm runner up` 启动时删除。

### 与服务器的连接

- Runner 只建立**出站**连接（可以在 NAT 之后运行，无需开放端口）：每 20 秒发送一次 heartbeat，有空闲容量时每 15 秒 long-poll 一次作业。
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
runner:    macbook-an (0190f500-…)
running:   yes
capacity:  2
harnesses: claude-code
repo:      AUR = /Users/an/code/aurora
repo:      WEB = /Users/an/code/web
```

```sh
ttm runner down     # 向 runner.pid 中的进程发送 SIGTERM（Windows：taskkill）
```

`Ctrl-C` 或 `ttm runner down` 会停止 runner；正在运行的 agent 会被取消。如果 runner 以服务方式运行，服务会重新启动它；要彻底关闭，请卸载服务。

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

> **注意：** 服务不会继承 shell 的 `PATH`。如果日志报告 `no agent harness found; install Claude Code (claude)
> first`，而 `claude` 在终端里可以运行，请为服务添加 `PATH`。macOS：加入 plist 后重新加载
>
> ```xml
> <key>EnvironmentVariables</key>
> <dict>
>   <key>PATH</key>
>   <string>/Users/an/.local/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin</string>
> </dict>
> ```
>
> Linux：执行 `systemctl --user edit ttm-runner`，在 `[Service]` 中加入 `Environment=PATH=…`。如果希望在你未登录时 runner
> 也能运行，Linux 还需要 `loginctl enable-linger $USER`。

### 卸载服务

```sh
# macOS
launchctl unload -w ~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist
rm ~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist

# Linux
systemctl --user disable --now ttm-runner.service
rm ~/.config/systemd/user/ttm-runner.service && systemctl --user daemon-reload
```

```powershell
# Windows
schtasks /Delete /TN ttm-runner /F
```

## 立即运行一个任务：`ttm agent`

```sh
ttm agent run AUR-12
```

```text
Run 0190f610-… for AUR-12 on branch aur-12-dang-nhap-google
[log] worktree /Users/an/.ttm/worktrees/AUR/aur-12-dang-nhap-google on aur-12-dang-nhap-google from main
[log] agent started claude-sonnet-4-5
[thought] Mình sẽ đọc context của task trước.
[action] mcp__ttm__factory-get-context
[action] Bash: cargo test -p auth
[log] tests passed: cargo test -p auth
[response] Đã thêm đăng nhập Google, test đều qua.
Run 0190f610-…: succeeded
worktree: /Users/an/.ttm/worktrees/AUR/aur-12-dang-nhap-google
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
| 作业 token（`ttm_job_`） | agent 的 `ttm-mcp`，仅限该 run | 临时 MCP 文件（0600），run 结束后删除 |

撤销 runner：在 `/me/ket-noi` 的 **Runner** 或 **Token** 标签页操作。

## 安全模型

- **只有出站连接**；不在你的机器上开放任何端口。
- **只使用你声明的仓库**，每个作业都检查 `origin`；不 clone 陌生 URL；分支名在使用前经过检查。
- **每个任务一个 worktree、一个分支**；你自己的 worktree 不会被改动。`pre-bash` hook 会阻止直接推送 `main`/`master`、`--no-verify`
  以及不符合约定的提交。
- **作业 token** 只能在它所属的 run 中使用，随作业过期；它保存在 0600 文件中，run 结束后删除。
- **Agent 以你的操作系统用户权限运行**，受作业下发的允许工具列表约束。这不是沙箱：只为你信任的项目和工作区运行 runner，并在合并前审查 PR。
- 每一步的最终 prompt 都会经过你的机器（用于运行 Claude Code）；服务器永远不会接收你的 Claude key。
