---
title: 连接 MCP
description: 在 Claude Code、Claude Desktop、Cursor、VS Code、Codex、Windsurf 中使用 ttm-mcp，或为 Claude.ai 和 ChatGPT 使用远程 MCP。
order: 6
section: guides
---

让 AI 应用使用你的 Notee 和 Factory 有两种方式：

| | 本地 MCP（`ttm-mcp`） | 远程 MCP |
| --- | --- | --- |
| 运行位置 | 在你的机器上，应用通过 stdio 启动 `ttm-mcp` | 在服务器上，应用调用 `https://toantranmei.com/api/v1/mcp` |
| 安装 | `ttm`、`ttm-mcp` | 无需安装 |
| 登录 | `ttm login` 获得的 token（或 `TTM_TOKEN`） | 在应用内走 OAuth，在网页上点击 **允许** |
| 适用于 | Claude Code、Claude Desktop、Cursor、VS Code、Codex、Windsurf、runner | Claude.ai、ChatGPT、支持通过 URL 接入 MCP 的应用 |
| 工具 | 相同（同一份源码） | 相同 |

AI 始终 **代表你** 行事，权限范围是 token 的 scope 与你对每个页面的权限。

## 本地 MCP

### 准备

```sh
ttm login          # ttm-mcp 复用这个 token
ttm doctor         # ttm-mcp 这一行必须是 [ok  ]
```

`ttm-mcp` 读取 token 的方式与 `ttm` 相同：先 `TTM_TOKEN`，再 keyring，再 `credentials.toml`，按当前选中的 profile 和
host。token 过期且有 refresh token 时会自动续期。未登录时服务器仍会启动，但所有工具都返回 `unauthorized` 错误
（日志：``not signed in to <host>; run `ttm login` (tools will answer unauthorized)``）。

### 自动安装

```sh
ttm mcp install --client claude-code
ttm mcp install --client claude-desktop
ttm mcp install --client cursor
ttm mcp install --client codex
```

| Client | `ttm mcp install` 做什么 |
| --- | --- |
| `claude-code` | 运行 `claude mcp add --scope user <tên> -- <ttm-mcp> [args]`。如果该名称已存在，先移除（`claude mcp remove --scope user <tên>`）再重新添加。需要 `PATH` 中有 `claude`。 |
| `claude-desktop` | 把 `mcpServers.<tên>` 写入 `claude_desktop_config.json` |
| `cursor` | 把 `mcpServers.<tên>` 写入 `~/.cursor/mcp.json` |
| `codex` | 把 `[mcp_servers.<tên>]` 写入 `$CODEX_HOME/config.toml`（默认 `~/.codex/config.toml`） |

对于配置文件：

- **合并，不覆盖**：其他 MCP 服务器、其他键、键的顺序、`ttm` 条目已有的 `env` 以及（TOML 的）注释都保持不变；只会重设
  `ttm` 条目的 `command` 和 `args`。
- **备份**：修改前会把旧文件复制为 `<file>.bak-<giây unix>`。
- **文件损坏**（JSON/TOML 语法错误）时不写入任何内容并报错；修好文件后重新运行。
- 没有任何变化时重新运行，会打印 ``<client> already uses <命令> for `<名称>`.``，且不创建备份。

| 标志 | 默认值 | 作用 |
| --- | --- | --- |
| `--client <tên>` | （必填） | `claude-code`、`claude-desktop`、`cursor`、`codex` |
| `--name <tên>` | `ttm` | client 配置中的服务器名称 |
| `--command <đường dẫn>` | 与 `ttm` 同目录的 `ttm-mcp`，其次 `PATH` 中的，最后是字面量 `ttm-mcp` | client 要运行的命令。使用绝对路径，因为图形界面应用看不到 shell 的 `PATH`。 |
| `--toolsets <danh sách>` | （不传） | 在 `args` 中加入 `--toolsets <danh sách>` |
| `--dry-run` | | 打印将要写入的内容（或将要运行的 `claude` 命令），不写入任何内容 |
| `--host`、`--profile`（全局标志） | | 只有显式传入（或设置了 `TTM_HOST`）时才会固定写入 `args` |

```sh
ttm mcp install --client cursor --dry-run
```

```text
--- /Users/an/.cursor/mcp.json (dry run)
{
  "mcpServers": {
    "ttm": {
      "command": "/Users/an/.cargo/bin/ttm-mcp",
      "args": []
    }
  }
}

Dry run: nothing written.
```

```sh
ttm --profile dev --host http://localhost:3000 mcp install --client claude-code --name ttm-dev
```

```text
Claude Code now starts `/Users/an/.cargo/bin/ttm-mcp` as MCP server `ttm-dev`.
Restart the client to load it. Sign in first with `ttm login` if you have not.
```

安装后请重启应用。

### Claude Code

手动安装（用户范围，所有项目）：

```sh
claude mcp add --scope user ttm -- ttm-mcp
claude mcp add --scope user ttm-notee -- ttm-mcp --toolsets notee
```

仅用于某个项目时，在仓库根目录放一个 `.mcp.json` 文件：

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": ["--toolsets", "notee,factory"] }
  }
}
```

检查：在 Claude Code 中输入 `/mcp`，或运行 `claude mcp list`。

### Claude Desktop

配置文件：

| 操作系统 | 路径 |
| --- | --- |
| macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| Windows | `%APPDATA%\Claude\claude_desktop_config.json` |
| Linux | `$XDG_CONFIG_HOME/Claude/claude_desktop_config.json`（默认 `~/.config/Claude/…`） |

```json
{
  "mcpServers": {
    "ttm": {
      "command": "/Users/an/.cargo/bin/ttm-mcp",
      "args": []
    }
  }
}
```

在 Windows 上路径要使用 `\\`：`"C:\\Users\\an\\.cargo\\bin\\ttm-mcp.exe"`。完全退出 Claude Desktop 后再重新打开。

#### `.mcpb` 包（一键安装）

从 0.1.6 起，[GitHub Releases](https://github.com/toantranmei/ttm/releases) 上的每个版本都为各操作系统提供 Claude Desktop
包，并附带 `.sha256` 文件：

| 包 | 机器 |
| --- | --- |
| `ttm-mcp-darwin.mcpb` | macOS（Apple Silicon 和 Intel，universal 二进制） |
| `ttm-mcp-win32-x64.mcpb` | Windows x64（Windows on ARM 通过模拟运行） |
| `ttm-mcp-linux-x64.mcpb`、`ttm-mcp-linux-arm64.mcpb` | Linux（静态 musl 二进制） |

用 Claude Desktop 打开该文件（双击，或 **Settings → Extensions → Install Extension…**）。该包运行
`ttm-mcp --toolsets notee`（`workspace` 和 `notee` 工具组，不含 `factory`），并询问两个值：**Access token**（在
`https://toantranmei.com/me/ket-noi` 创建的 `ttm_pat_…` PAT；留空则使用 `ttm login` 保存的 token）和 **Server**（默认
`https://toantranmei.com`）。无需另外安装 `ttm-mcp`；安装新版本的包即可更新。

### Cursor

`ttm mcp install --client cursor` 会写入 `~/.cursor/mcp.json`（所有项目）。仅用于某个项目时：在仓库中使用
`.cursor/mcp.json`，格式相同：

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": [] }
  }
}
```

### VS Code（GitHub Copilot，agent 模式）

目前还没有 `--client vscode`；请手动添加到工作区的 `.vscode/mcp.json`（或使用命令 **MCP: Open User Configuration**）：

```json
{
  "servers": {
    "ttm": {
      "type": "stdio",
      "command": "ttm-mcp",
      "args": ["--toolsets", "notee"]
    }
  }
}
```

### Codex

`~/.codex/config.toml`（或 `$CODEX_HOME/config.toml`）：

```toml
[mcp_servers.ttm]
command = "/Users/an/.cargo/bin/ttm-mcp"
args = ["--toolsets", "notee"]
```

### Windsurf 及其他应用

Windsurf：`~/.codeium/windsurf/mcp_config.json`。大多数应用使用相同的 `mcpServers` 块：

```json
{
  "mcpServers": {
    "ttm": {
      "command": "ttm-mcp",
      "args": [],
      "env": { "TTM_PROFILE": "work" }
    }
  }
}
```

需要时通过 `env` 传入配置：`TTM_PROFILE`、`TTM_HOST`、`TTM_TOOLSETS`，或 `TTM_TOKEN`（没有 keyring 的机器）。
通过 npm 运行，无需安装：`"command": "npx", "args": ["-y", "@toantranmei/mcp"]`。

### ttm-mcp 的选项

```text
Usage: ttm-mcp [OPTIONS]
```

| 标志 | 变量 | 作用 |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | 服务器 |
| `--profile <NAME>` | `TTM_PROFILE` | `config.toml` 中的 profile |
| `--toolsets <LIST>` | `TTM_TOOLSETS` | 启用的工具组，以逗号分隔：`notee`、`factory`、`admin`。`workspace` 始终启用。不设置 = 全部。在运行中（`TTM_RUN_ID`）总会去掉 `admin`。 |
| `-v`、`-vv` | `TTM_LOG` | 日志输出到 stderr |
| `-V`、`--version` | | 打印版本 |

`ttm-mcp` 没有 `--json` 也没有子命令：它只在 stdin/stdout 上讲 MCP 协议。无法访问 discovery 时，它使用默认端点
`<host>/api/v1`。

## 工具组（toolset）

| 组 | 工具 | 所需 scope |
| --- | --- | --- |
| `workspace`（始终启用） | `workspace-get-self`、`workspace-list`、`workspace-get-users` | `workspace:read` |
| `notee`（别名 `notes`） | `notee-search`、`notee-fetch`、`notee-query-database`、`notee-get-comments`、`notee-create-pages`、`notee-update-page`、`notee-move-pages`、`notee-duplicate-page`、`notee-create-database`、`notee-update-database`、`notee-upsert-rows`、`notee-delete-rows`、`notee-create-comment`、`notee-get-suggestions`、`notee-resolve-suggestion`；resource `notee://spec/markdown` | `notee:read` / `notee:comment` / `notee:write` |
| `factory` | 8 个运行工具（`factory-get-task`、`factory-get-context`、`factory-report-progress`、`factory-request-approval`、`factory-create-subtasks`、`factory-attach-artifact`、`factory-get-rules`、`factory-renew-lease`）+ 0.2.0 只读工具（`factory-workflow-*`、`factory-automation-list/get/test`、`factory-agent-list/get`、`factory-skill-list/get`、`factory-template-search`、`factory-template-install-preview`、`factory-inbox`） | `factory:read` / `factory:run` |
| `admin` | 两步确认的配置工具（`factory-workflow-draft-update`、`factory-workflow-publish`、`factory-workflow-upgrade`、`factory-automation-draft`、`factory-automation-set-enabled`、`factory-agent-draft`、`factory-agent-set-status`、`factory-skill-review`、`factory-template-install`、`factory-task-approve`、`factory-confirm`、`factory-confirm-cancel`）。agent 运行中永远不提供 | `factory:admin`（`factory-task-approve`：`factory:write`） |

```sh
ttm-mcp --toolsets notee            # workspace + notee
ttm-mcp --toolsets ""               # 仅 workspace
```

各工具的参数、结果和错误：[MCP 工具](mcp-cong-cu.md)。工具越少，模型越容易选对工具，占用的 context 也越少；`factory`
主要为在 runner 中运行的 agent 启用。

`ttm mcp install --toolsets <LIST>` 会把同样的列表写入 client 配置（例如
`ttm mcp install --client claude-desktop --toolsets notee,factory,admin`）。不想通过 MCP 做管理就去掉 `admin`。旧 token
（0.2.0 之前登录）缺少 `factory:admin` scope：admin 工具会返回 `insufficient_scope`，请重新 `ttm login`。参见
[MCP 工具](mcp-cong-cu.md)。

服务器向 client 介绍自己为 `ttm`，并附带说明：*Start with workspace-get-self to learn who the user is and which
workspace is current.* 当工具修改页面时，client 应用的名称（例如 `claude-code`）会记录在页面历史中：“An (via
agent claude-code)”。

## agent 运行时的变量

runner 运行一个任务时，会用以下变量启动 `ttm-mcp`：

| 变量 | 值 |
| --- | --- |
| `TTM_TOKEN` | 作业（job）的 token（`ttm_job_…`），只能用于该次 run |
| `TTM_HOST` | runner 的 host |
| `TTM_NO_KEYRING` | `1` |
| `TTM_RUN_ID` | run 的 id |
| `TTM_TASK` | 任务 key，例如 `AUR-12` |
| `TTM_PROJECT` | 项目的 id（或 key） |

因此 `factory-*` 工具知道“这次 run 的任务”是哪个，agent 无需传入 key。在 run 之外，需要 run 的工具
（`factory-report-progress`、`factory-request-approval`、`factory-attach-artifact`、`factory-renew-lease`）会返回
`no_run` 错误。

## 远程 MCP

URL：**`https://toantranmei.com/api/v1/mcp`**（MCP Streamable HTTP，无会话状态）。

### Claude.ai

1. **Settings → Connectors → Add custom connector**。
2. 名称任意（例如 *toantranmei*），URL 填 `https://toantranmei.com/api/v1/mcp`，点击 **Add**，然后点击 **Connect**。
3. 浏览器会打开 toantranmei.com 的授权页面；按需登录后点击 **允许**。

### ChatGPT

在 **Settings → Connectors** 中（自定义 connector 需要开启 developer 模式），用上面的 URL 新建一个 connector，选择 OAuth
认证并完成授权步骤。

### 授权页面显示什么

- 标题 **“<应用名称> 想要访问你的账号”**，以及一行 *外部应用。仅在你信任此应用时才允许。*
- 如果应用名称与 toantranmei 的应用名称 **相似** 但并非官方应用，会显示红色警告。
- 授权后浏览器返回的位置（*授权后，浏览器将返回 <host>。*）。
- 所申请的权限列表。应用未申请具体 scope 时，会获得除 `offline_access` 以外的所有 scope。
- 当你属于多个工作区时显示 **限定在工作区内** 一栏。
- **允许** / **拒绝** 按钮。

远程应用的 token 有效期为 **1 小时**；只有申请了 `offline_access` 的应用才会获得 refresh token（90 天，每次使用都会轮换）。
重复使用旧的 refresh token 会撤销整个 token 家族。在 `<host>/me/ket-noi` 的 **已连接的应用** 标签页中断开连接。

### 面向 client 开发者的细节

| 项 | 值 |
| --- | --- |
| 没有 token | `401`，带 `WWW-Authenticate: Bearer resource_metadata="https://toantranmei.com/.well-known/oauth-protected-resource/api/v1/mcp"` |
| Protected Resource Metadata（RFC 9728） | `/.well-known/oauth-protected-resource`（以及后缀 `/api/v1/mcp`） |
| Authorization Server Metadata（RFC 8414） | `/.well-known/oauth-authorization-server` |
| 授权页面 | `/ket-noi/cli` |
| token、撤销、设备 | `/api/v1/oauth/token`、`/api/v1/oauth/revoke`、`/api/v1/oauth/device_authorization` |
| client 注册 | Client ID Metadata Document（`client_id` 是一个 URL），或 Dynamic Client Registration `/api/v1/oauth/register` |
| PKCE | 必须，`S256` |
| `resource` | 必须是 `https://toantranmei.com/api/v1/mcp`，否则返回 `invalid_target` |

远程 MCP 上的每次工具调用都会经过与 API 请求相同的 token、scope 和权限检查。远程 MCP 启用所有工具组；需要 run 的
Factory 工具在这里返回 `no_run`。

### 通过远程 MCP 修改配置

远程 MCP 有相同的 `admin` 组（带 `factory:admin` 的用户 token）。远程服务器不保存会话，因此无法反向询问（elicitation）：admin
工具总是返回 `confirm_id`，由模型询问用户后再调用 `factory-confirm`。任务 token 只有 `factory,notee` 组。

## 限制

| 限制 | 值 |
| --- | --- |
| agent 写入 Notee 的次数 | 每个 token 每小时 60 次（创建页面、以页面形式创建行、编辑页面）；超出返回 `429 rate_limited` |
| 单次写入 | 1,000 个 block（含嵌套 block），2 MB Markdown |
| `notee-create-pages` | 每次 1–20 个页面 |
| `notee-upsert-rows` | 每次 1–100 行 |
| `notee-search` | 最多 20 条结果；查询词 ≤ 8 个词，≤ 200 个字符 |
| `notee-query-database` | 每页 1–100 行（默认 100），3 秒，每页 4 MB，每人同时最多 2 个查询 |
| 长页面 | 超过 20,000 个节点后截断（`truncated`） |
| 评论的摘录 | 1–1,000 个字符 |
| `factory-create-subtasks` | 每次 run 10 个子任务，单个根任务下 30 个，最多 2 层 |

## 用 MCP Inspector 测试服务器

```sh
npx @modelcontextprotocol/inspector ttm-mcp --toolsets notee
```

Inspector 会打开一个网页界面，可列出工具、读取 resource `notee://spec/markdown` 并逐个试调用工具。
