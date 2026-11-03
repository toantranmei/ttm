---
title: MCP 连接
description: 在 Claude Code、Claude Desktop、Cursor 和 Codex 中使用 ttm-mcp,或使用无需安装的远程 MCP。
order: 4
---

# MCP 连接

`ttm-mcp` 是在你的电脑上运行的 MCP 服务器(stdio),使用 `ttm login` 保存的令牌(或 `TTM_TOKEN`)。AI 在令牌权限范围内
代表你操作。

## 快速配置

```sh
ttm login
ttm mcp install --client claude-code      # 执行:claude mcp add --scope user ttm -- ttm-mcp
ttm mcp install --client claude-desktop   # 修改 claude_desktop_config.json
ttm mcp install --client cursor           # 修改 ~/.cursor/mcp.json
ttm mcp install --client codex            # 修改 ~/.codex/config.toml
```

配置文件在修改前会先 **备份**(`<文件>.bak-<时间戳>`),并采用 **合并** 方式:其他 MCP 服务器和设置保持不变。文件损坏时
ttm 不会写入。可用 `--dry-run` 预览。

## 手动配置

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": [] }
  }
}
```

Codex(`~/.codex/config.toml`):

```toml
[mcp_servers.ttm]
command = "ttm-mcp"
args = []
```

## ttm-mcp 参数

| 参数 | 作用 |
| --- | --- |
| `--host <URL>` | 服务器(默认值与 `ttm` 相同) |
| `--profile <名称>` | `config.toml` 中的配置档 |
| `--toolsets notee,factory` | 额外启用的工具组(`workspace` 始终启用) |
| `-v`、`-vv` | 日志输出到 stderr |

## 工具

| 工具 | 作用 |
| --- | --- |
| `workspace-get-self` | 当前用户和当前工作区 |
| `workspace-list` | 工作区列表 |
| `workspace-get-users` | 工作区成员(可按姓名/邮箱过滤) |

`notee` 组:`notee-search`、`notee-fetch`、`notee-query-database`、`notee-get-comments`、`notee-create-pages`、
`notee-update-page`(`mode: suggest` 提交修改建议)、`notee-move-pages`、`notee-duplicate-page`、`notee-create-database`、
`notee-update-database`、`notee-upsert-rows`、`notee-delete-rows`、`notee-create-comment`、`notee-get-suggestions`、
`notee-resolve-suggestion`;资源 `notee://spec/markdown`。

`factory` 组(运行中的智能体使用):`factory-get-task`、`factory-get-context`、`factory-report-progress`、
`factory-request-approval`、`factory-create-subtasks`、`factory-attach-artifact`、`factory-get-rules`、`factory-renew-lease`。

## 无需终端的 Claude Desktop

在 Releases 页面下载对应系统的 `.mcpb` 文件,用 Claude Desktop 打开,粘贴在 `/me/ket-noi` 创建的令牌。

## 远程 MCP

无需安装:将远程 MCP 地址(在网页版本页面中显示)添加到 Claude.ai 或 ChatGPT,登录并允许即可。
