---
title: 配置
description: config.toml 与 profile、host 的选择顺序、各操作系统上的文件位置、JSON 输出、日志与网络。
order: 5
section: guides
---

`ttm` 无需任何配置即可运行：没有配置文件时，使用 profile `default` 和 host `https://toantranmei.com`。下面这些
文件会在你登录、切换工作区或配置 runner 时创建。

## 文件位置

| 目录 | macOS、Linux | Windows | 覆盖方式 |
| --- | --- | --- | --- |
| 配置 | `$XDG_CONFIG_HOME/ttm`，默认 `~/.config/ttm` | `%APPDATA%\ttm` | `TTM_CONFIG_DIR` |
| 缓存 | `$XDG_CACHE_HOME/ttm`，默认 `~/.cache/ttm` | `%LOCALAPPDATA%\ttm\cache` | `TTM_CACHE_DIR` |
| Worktree | `~/.ttm/worktrees` | `%USERPROFILE%\.ttm\worktrees` | `config.toml` 或 `runner.toml` 中的 `worktree_root` |

> **注意：** 在 macOS 上，ttm 同样使用 `~/.config/ttm` 和 `~/.cache/ttm`（不使用 `~/Library`）。

| 文件 | 位置 | 内容 |
| --- | --- | --- |
| `config.toml` | 配置目录 | Profile、host、当前工作区、`worktree_root` |
| `credentials.toml` | 配置目录 | 无法使用 keyring 时存放的 token（权限 0600） |
| `runner.toml` | 配置目录 | Runner 配置（见 [Runner](runner.md)） |
| `discovery-<host>.json` | 缓存 | Host 的 discovery，1 小时内复用；host 无响应时仍使用旧副本 |
| `deprecations` | 缓存 | 当天已显示过的“endpoint 即将移除”警告 |
| `runner.pid`、`runner.log`、`runner-worktrees.json` | 缓存 | Runner 的进程、日志（macOS 服务）和 worktree 登记簿 |
| `.ttm/project.toml` | 仓库内 | 将仓库关联到 Factory 项目（见 [Factory](factory.md)） |

`ttm status` 会打印当前使用的 `config.toml` 路径。

## config.toml

```toml
# 未指定 --profile 或 TTM_PROFILE 时使用的 profile。
default_profile = "work"

# `ttm sync --resolve` 和 runner 使用的 worktree 目录（默认 ~/.ttm/worktrees）。`~/` 会被展开。
worktree_root = "~/orca/workspaces"

[profiles.default]
host = "https://toantranmei.com"

[profiles.work]
host = "https://toantranmei.com"
workspace = "0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71"

[profiles.dev]
host = "http://localhost:3000"
```

| 键 | 类型 | 写入者 | 含义 |
| --- | --- | --- | --- |
| `default_profile` | 字符串 | 你 | 默认 profile；未设置时为 `default` |
| `worktree_root` | 字符串 | 你 | Worktree 根目录 |
| `profiles.<tên>.host` | URL | `ttm login` | 该 profile 的服务器 |
| `profiles.<tên>.workspace` | id | `ttm workspace switch` | 该 profile 的当前工作区 |

`ttm` 以整体方式写入此文件（先写临时文件再重命名），因此你添加的注释会在下次写入时丢失。文件语法错误会导致所有命令
（`ttm completion` 除外）以 `invalid TOML in <đường dẫn>: …` 退出。

## Profile

一个 profile 是一对有名称的（host，工作区）。Token 按（profile，host）对存储。

**Profile** 的选择顺序：

1. `--profile <tên>`
2. `TTM_PROFILE`
3. `config.toml` 中的 `default_profile`
4. `default`

**Host** 的选择顺序：

1. `--host <URL>`
2. `TTM_HOST`
3. Profile 的 `host`
4. `https://toantranmei.com`（构建时的值，可在构建时通过 `TTM_DEFAULT_HOST` 修改）

Host 会被规范化：去掉末尾的 `/`，域名转为小写，缺少协议时补上 `https://`（`localhost` 和 `127.0.0.1` 例外，补上
`http://`）。只接受 `http` 和 `https`；其他值报 `invalid host`。

```sh
ttm --host toantranmei.com status        # → https://toantranmei.com
ttm --host localhost:3000 status         # → http://localhost:3000
```

需要工作区的命令（`notes search`、`notes create --workspace`、`project init`、MCP 工具）按以下顺序选择**工作区**：
先取命令参数，再取 profile 的工作区。`ttm-mcp` 在尚未选择工作区时使用个人工作区。

查看本机当前使用的全部设置：

```sh
ttm status
```

```text
ttm 0.1.4
host:      https://toantranmei.com
profile:   work (/Users/an/.config/ttm/config.toml)
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
workspace: 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
server:    API v1 (min ttm 0.1.0), features: notee, factory
```

未登录或断网时 `ttm status` 不会报错（显示 `server: unreachable`）。

## 环境变量

最常用的变量：

| 变量 | 作用 |
| --- | --- |
| `TTM_HOST` | 服务器（优先级低于 `--host`） |
| `TTM_PROFILE` | Profile（优先级低于 `--profile`） |
| `TTM_TOKEN` | Token，优先于 keyring 和文件 |
| `TTM_NO_KEYRING=1` | 不使用 keyring，只使用 `credentials.toml` |
| `TTM_LOG` | 日志级别（`tracing` 语法），例如 `debug` |

完整列表（包括 `ttm-mcp` 和 runner 的变量）：[环境变量](bien-moi-truong.md)。

## JSON 输出

所有命令都接受 `--json`（放在命令行任意位置均可）：结果以格式化的 JSON 输出到 **stdout**，辅助信息仍输出到
stderr。

```sh
ttm whoami --json
```

```json
{
  "host": "https://toantranmei.com",
  "profile": "default",
  "user": {
    "id": "0190f3a1-0000-7000-8000-000000000001",
    "email": "an@example.com",
    "name": "An Nguyễn",
    "avatar_url": null,
    "locale": "vi",
    "plan": "free"
  },
  "workspace": null
}
```

出错时，`--json` 会额外向 stdout 输出一个错误对象（并向 stderr 输出 `error: …` 行），退出码非 0：

```json
{
  "error": {
    "code": "insufficient_scope",
    "message": "this token lacks the scope notee:write (insufficient_scope, HTTP 403) [requires scope notee:write]",
    "status": 403,
    "params": { "required": "notee:write" }
  }
}
```

来自服务器的错误还带有 `status`，以及服务器发送时的 `rule`（例如 `split-limit`、`rev-conflict`）、`field` 和 `params`。
`code` 是服务器的稳定错误码（见 [错误码](ma-loi.md)）、OAuth 错误码（`access_denied`、`invalid_grant`…），或本地错误码：
`network`（无法连接）、`decode`（响应格式异常）、`upgrade_required`（ttm 版本过旧）、`invalid_url`、`error`（其他错误，
如未登录、文件无法读取）。

在脚本中使用的示例：

```sh
ttm notes search "OKR" --json | jq -r '.[].id'
ttm notes query "$DB" --sql 'SELECT "Name" FROM rows' --json | jq '.rows[].values'
```

## 日志

日志输出到 **stderr**（`ttm-mcp` 的 stdout 专用于 MCP 协议）。

| 参数 | 级别 |
| --- | --- |
| （无） | `warn`：警告，例如 endpoint 即将移除、服务器要求新版本 |
| `-v` | `info`（ttm 自身的模块为 `debug`） |
| `-vv` | `debug` |
| `TTM_LOG=<filter>` | 覆盖级别，语法为 `tracing_subscriber::EnvFilter`，例如 `TTM_LOG=ttm_api=debug,warn` |

Token 永远不会出现在日志中：所有显示 token 的地方只保留前缀和最多 4 个字符。

## 网络

- 每个请求的超时为 **30 秒**，并发送 `User-Agent: ttm/<phiên bản> (<os>; <arch>)`。
- 读取类请求（GET、PUT、DELETE）遇到连接错误、超时和 5xx 错误时最多重试 **3 次**（分别等待 0.5 秒、1 秒、2 秒）。
- `429` 对所有类型的请求最多重试 3 次，按 `Retry-After` 等待（每次最多 60 秒）。
- 带有 `Deprecation` 头的 endpoint 每天产生一条警告，如有移除日期（`Sunset`）会一并显示。
- 代理：遵循标准变量 `HTTPS_PROXY`、`HTTP_PROXY`、`NO_PROXY`。
