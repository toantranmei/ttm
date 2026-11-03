---
title: 命令
description: ttm 命令、全局参数与 JSON 输出。
order: 3
---

# 命令

所有命令通用的参数:`--host <URL>`、`--profile <名称>`、`--json`(供脚本和智能体使用的 JSON 输出)、`-v` / `-vv`
(详细日志输出到 stderr)。

| 命令 | 作用 |
| --- | --- |
| `ttm login [--device] [--token <PAT>]` | 登录 |
| `ttm logout` | 吊销令牌并从本机删除 |
| `ttm whoami` | 当前登录用户 |
| `ttm status` | 正在使用的服务器、配置档、令牌和工作区 |
| `ttm doctor` | 检查服务器、discovery、令牌、版本、钥匙串、git、claude |
| `ttm workspace list` | 你的工作区(`*` 表示当前工作区) |
| `ttm workspace switch <id|slug|名称>` | 为配置档选择工作区 |
| `ttm workspace current` | 当前工作区 |
| `ttm token list` / `create` / `revoke [id]` | 打开 `/me/ket-noi` 页面(见下文) |
| `ttm notes search|get|query|comments` | 搜索、读取页面(Markdown)、查询数据库(SQL 或 JSON 过滤)、读取评论 |
| `ttm notes create|edit|comment|rows` | 从文件创建页面、编辑(`--old/--new`、`--replace`、`--suggest`)、引文评论、数据库行 |
| `ttm notes suggestions|resolve` | 查看并接受/拒绝修改建议 |
| `ttm project init|link|open|status` | 将仓库关联到 Factory 项目 |
| `ttm sync [--check] [--resolve]` | 同步 `.ttm/plan` 中的计划,提交变更请求 |
| `ttm task list|show|claim|done|approve|split` | 项目任务 |
| `ttm rules apply` | 将规则写入 AGENTS.md/CLAUDE.md 和智能体钩子 |
| `ttm runner up|down|status|config|install-service` | 在本机运行服务器分派的智能体任务 |
| `ttm agent run|logs|stop` | 立即运行任务、查看日志、停止 |
| `ttm mcp install --client <客户端>` | 配置 MCP 客户端使用 `ttm-mcp` |
| `ttm self-update [--check]` | 更新 ttm |
| `ttm completion <bash|zsh|fish|powershell|elvish>` | Shell 自动补全脚本 |

## 令牌

令牌管理 API 只接受网页登录会话,不接受令牌(令牌不能创建其他令牌)。因此 `ttm token create|list|revoke` 会在浏览器中
打开 `<服务器>/me/ket-noi`;新令牌只在那里显示一次。

```sh
ttm token create --scope notee:read,workspace:read --expires 30d --name "CI"
```

## 自动补全

```sh
ttm completion zsh > "${fpath[1]}/_ttm"
ttm completion bash > ~/.local/share/bash-completion/completions/ttm
ttm completion fish > ~/.config/fish/completions/ttm.fish
```

## JSON

使用 `--json` 时结果以 JSON 输出到 stdout;错误格式为 `{"error": {"code": "...", "message": "..."}}`,退出码非 0。
