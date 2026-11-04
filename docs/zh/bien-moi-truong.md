---
title: 环境变量
description: ttm、ttm-mcp 和 runner 读取的所有环境变量，及其默认值和优先级。
order: 13
section: reference
---

值为空的变量视为未设置（另有说明的除外）。

## 连接与认证

| 变量 | 读取方 | 默认值 | 含义 |
| --- | --- | --- | --- |
| `TTM_HOST` | `ttm`、`ttm-mcp` | profile 的 host，其次是 `https://toantranmei.com` | 服务器。优先级低于 `--host`，高于 profile。设置后，`ttm login` 会把该 host 写入 profile，`ttm mcp install` 会把 `--host` 固定写入客户端配置。 |
| `TTM_PROFILE` | `ttm`、`ttm-mcp` | `default_profile`，其次是 `default` | `config.toml` 中的 profile。优先级低于 `--profile`。 |
| `TTM_TOKEN` | `ttm`、`ttm-mcp` | （无） | 所有请求使用的 token，优先于 keyring 和 `credentials.toml`。不会自动续期。runner 不会把它用作 runner 自己的 token。 |
| `TTM_NO_KEYRING` | `ttm`、`ttm-mcp`、runner | （启用 keyring） | 值非空且不为 `0`：不读取也不写入 keyring；token 存入 `credentials.toml`。 |
| `TTM_NO_BROWSER` | `ttm` | （打开浏览器） | 有值时：不打开浏览器（`ttm login`、`ttm token …`、`ttm project open`），只打印 URL。 |

## 目录

| 变量 | 默认值 | 含义 |
| --- | --- | --- |
| `TTM_CONFIG_DIR` | `~/.config/ttm`（`%APPDATA%\ttm`） | 存放 `config.toml`、`credentials.toml`、`runner.toml` 的目录 |
| `TTM_CACHE_DIR` | `~/.cache/ttm`（`%LOCALAPPDATA%\ttm\cache`） | discovery 缓存、runner 的 pid 和日志、worktree 登记簿 |
| `TTM_HOME` | 操作系统的用户目录 | 替代用户主目录：用于展开 `~/`、`~/.ttm/worktrees`、`ttm mcp install` 查找客户端配置的位置、runner 的服务文件。适用于便携安装和测试（在 Windows 上 `HOME` 会被忽略）。 |
| `XDG_CONFIG_HOME` | `~/.config` | （macOS、Linux）配置目录的根；也是 Linux 上 Claude Desktop 配置和 runner 的 systemd 文件的根 |
| `XDG_CACHE_HOME` | `~/.cache` | （macOS、Linux）缓存目录的根 |
| `APPDATA`、`LOCALAPPDATA` | 由 Windows 设置 | （Windows）配置和缓存目录；`APPDATA` 也是查找 Claude Desktop 配置的位置 |
| `CODEX_HOME` | `~/.codex` | `ttm mcp install --client codex` 修改的 Codex 配置目录 |

## 日志

| 变量 | 含义 |
| --- | --- |
| `TTM_LOG` | 日志过滤器（`tracing_subscriber::EnvFilter`），覆盖 `-v`/`-vv`。例如 `TTM_LOG=debug`、`TTM_LOG=ttm_api=debug,warn`。 |

## 输出

| 变量 | 默认值 | 含义 |
| --- | --- | --- |
| `TTM_PLAIN` | （关闭） | 非空且不是 `0`/`false`/`no`/`off`：同 `--plain`（无颜色、边框、符号、emoji） |
| `TTM_COLUMNS` | （无） | 输出宽度，优先于 `COLUMNS` 和终端大小；限制在 40–200 |
| `COLUMNS` | （无） | 未设置 `TTM_COLUMNS` 时的宽度 |
| `NO_COLOR` | （无） | 非空：关闭颜色（优先于 `CLICOLOR_FORCE`） |
| `CLICOLOR_FORCE` | （无） | `1`：即使在管道中也输出颜色 |
| `COLORTERM` | （无） | `truecolor` 或 `24bit`：使用精确的品牌色 |
| `WT_SESSION` | 由 Windows Terminal 设置 | 同 `COLORTERM=truecolor` |
| `TERM` | （无） | `dumb`：无颜色；包含 `256color`：256 色 |

## ttm-mcp

| 变量 | 默认值 | 含义 |
| --- | --- | --- |
| `TTM_TOOLSETS` | 所有组 | 同 `--toolsets` 选项：额外启用的工具组，以逗号分隔（`notee`、`factory`、`admin`；`workspace` 始终启用）。选项优先于变量。设置了 `TTM_RUN_ID` 时总会去掉 `admin`（并给出警告）。 |
| `TTM_RUN_ID` | （无） | agent 本次运行的 id。`factory-report-progress`、`factory-request-approval`、`factory-attach-artifact`、`factory-renew-lease` 必需；`factory-get-context` 使用该 run 的 context。 |
| `TTM_TASK` | （无） | `factory-get-task`、`factory-get-context`、`factory-create-subtasks` 的默认任务 key（`AUR-12`）。 |
| `TTM_PROJECT` | （无） | `factory-get-rules` 的默认项目（id 或 key）。 |

runner 会为它替 agent 启动的 `ttm-mcp` 自动设置 `TTM_TOKEN`（作业的 token）、`TTM_HOST`、`TTM_NO_KEYRING=1`、`TTM_RUN_ID`、`TTM_TASK`、`TTM_PROJECT`。
`ttm hook` 也会读取 `TTM_TASK` 和 `TTM_RUN_ID`，以便把规则检查结果关联到正确的 run。

## Runner

| 变量 | 含义 |
| --- | --- |
| `HOSTNAME`、`COMPUTERNAME` | runner 的默认机器名（`runner.toml` 中没有 `name` 时）；都没有则运行 `hostname` 命令 |
| `PATH`（Windows 上还有 `PATHEXT`） | 查找 `claude`、`git`、`gh`、`ttm-mcp` 的位置 |
| `[mcp.local.*]` 中 `env:VAR` 引用的变量 | 运行时传给本地 MCP 的值；缺失时跳过该 MCP |

runner 为每个作业的 agent 和 git 设置：`GIT_AUTHOR_NAME/EMAIL`、`GIT_COMMITTER_NAME/EMAIL`（作业带 git 身份时）、`GIT_CONFIG_COUNT`、
`GIT_CONFIG_KEY_0=core.hooksPath`、`GIT_CONFIG_VALUE_0=<该 run 的 hook 目录>`（所有作业）；保存了 Claude token 时的同事作业：
`CLAUDE_CONFIG_DIR`（该 run 的空目录）和 `CLAUDE_CODE_OAUTH_TOKEN`。见 [Runner](runner.md)。

## 构建时

| 变量 | 含义 |
| --- | --- |
| `TTM_DEFAULT_HOST` | 嵌入二进制文件的默认 host（`cargo build`）。发布版本使用 `https://toantranmei.com`。 |

## 网络

HTTP 客户端照常遵循 `HTTPS_PROXY`、`HTTP_PROXY`、`ALL_PROXY`、`NO_PROXY`。
