---
title: 概览
description: ttm 与 ttm-mcp 是什么、主要概念，以及各组件如何相互连接。
order: 1
section: start
---

**ttm** 是 [toantranmei.com](https://toantranmei.com) 的开源（MIT）工具集，包含两个以相同版本
发布的程序：

| 程序 | 是什么 | 何时使用 |
| --- | --- | --- |
| `ttm` | 命令行（CLI） | 登录、切换工作区、读取和修改 Notee 页面、将仓库与 Factory 项目关联、同步计划、对 agent 强制执行规则、在你的机器上运行 agent、把 MCP 安装到各种 AI 应用 |
| `ttm-mcp` | 在你的机器上通过 stdio 运行的 [MCP](https://modelcontextprotocol.io) 服务器 | 让 Claude Code、Claude Desktop、Cursor、Codex、VS Code…… 代表你读写 Notee、处理 Factory 任务 |

源代码：<https://github.com/toantranmei/ttm>。当前版本：**0.1.6**。

如果你不使用终端，则无需安装任何东西：把**远程 MCP URL** `https://toantranmei.com/api/v1/mcp` 粘贴到
Claude.ai 或 ChatGPT 即可（参见 [连接 MCP](mcp.md)）。

## 概念

| 概念 | 含义 |
| --- | --- |
| **Host** | `ttm` 与之通信的服务器。默认为 `https://toantranmei.com`；可通过 `--host`、`TTM_HOST` 或 profile 修改（例如开发时用 `http://localhost:3000`）。 |
| **Discovery** | 文档 `GET <host>/.well-known/ttm` 给出 API、OAuth、MCP 的地址，API 版本、`min_cli` 以及功能（`notee`、`factory`）。除 host 外，ttm 不硬编码任何 URL。缓存有效期为 1 小时。 |
| **Profile** | `config.toml` 中带名称的一组（host，当前工作区），例如 `default`、`work`。每个 profile 有自己的 token。 |
| **Workspace** | 工作区（`personal` 或 `team`），包含 Notee 页面和 Factory 项目。你的角色为 `owner`、`admin`、`member` 或 `guest`。当前工作区按 profile 保存。 |
| **Token** | 以 `ttm_` 开头的密钥字符串：`ttm_pat_`（在网页上创建的个人 token）、`ttm_oat_`（OAuth 访问 token，由 `ttm login` 或远程 MCP 应用获取）、`ttm_ort_`（refresh token）、`ttm_job_`（runner 及每次 agent 运行的 token）。 |
| **Scope** | token 的权限：`workspace:read`、`notee:read`、`notee:write`、`notee:comment`、`factory:read`、`factory:write`、`factory:run`、`offline_access`。实际权限 = scope 与你对每个资源的权限的交集。参见 [登录与 token](dang-nhap.md)。 |
| **Notee** | 笔记应用：页面、数据库、评论、修改建议。agent 以 [Notee-flavored Markdown](notee-markdown.md) 形式读写页面。 |
| **Factory** | 面向人和 agent 的项目管理应用：项目、任务、工作流、审批关卡、变更请求（change request）、合并队列。网页是唯一可信来源；仓库在 `.ttm/plan` 中保存计划的副本。 |
| **Runner** | `ttm runner up`：开发机上的进程，从服务器接收*作业*（job），创建 git worktree，用你的 Claude 账号运行 Claude Code，将事件发送到 Execution Map，推送分支并创建 pull request。 |
| **Harness** | 用于运行某个 coding agent 的适配器。目前有 `claude-code`。 |
| **本地 MCP 与远程 MCP** | 本地 MCP 是你机器上的 `ttm-mcp`（stdio，使用 `ttm login` 的 token）。远程 MCP 是 endpoint `https://toantranmei.com/api/v1/mcp`（Streamable HTTP，OAuth 2.1），使用同一套工具。 |

## 架构

```text
                      ┌──────────────── toantranmei.com（服务器）──────────────────┐
 Claude.ai, ChatGPT ─▶│ /api/v1/mcp        远程 MCP（Streamable HTTP，OAuth 2.1）  │
                      │ /.well-known/ttm   discovery                               │
 ttm, ttm-mcp ───────▶│ /api/v1            API（Bearer token、scope、ACL）         │
   (Bearer)           │ /ket-noi/cli       登录授权页面                            │
                      │ /activate          输入设备码                              │
                      │ /me/ket-noi        管理 token、应用、runner                │
                      └────────────────────────────────────────────────────────────┘
                              ▲ long-poll + heartbeat（仅出站连接）
                              │
 开发机：  ttm runner ── 每个任务一个 git worktree ── claude -p（你的账号）
                                                      │ stdio
                                                      ▼
                                                 ttm-mcp（本次运行的 token）
```

- **“大脑”在服务器上**：拆分任务、选择模型、拼装 context 和 prompt。`ttm` 只负责认证、创建 worktree、按作业运行
  harness、发送事件、在本地强制执行规则。
- **agent 只在你的机器上运行**，使用你的 Claude 套餐或 API key。服务器不保存 Claude key。
- 本地 MCP 和远程 MCP 使用**同一套工具**：服务器复用了 crate `ttm-mcp-tools`，因此工具名称、参数和
  结果都相同。

## 仓库内容

| Crate | 作用 |
| --- | --- |
| `ttm-api` | `/api/v1` 的 HTTP 客户端：重试、User-Agent、`Deprecation` 警告、426 错误、problem+json |
| `ttm-core` | 配置、profile、host 选择、discovery、token 存储、项目文件、worktree |
| `ttm-mcp-tools` | 基于 `Backend` trait 的 MCP 工具（与远程 MCP 共用） |
| `ttm-runner` | Runner：接收作业、worktree、Claude Code harness、事件流、合并队列 |
| `ttm-cli` | `ttm` 二进制 |
| `ttm-mcp` | `ttm-mcp` 二进制 |

## 0.2 新功能

- **统一输出**：品牌色、表格、状态符号、`Next:` 行；`--plain` 适合日志，`--json` 适合脚本（[命令行界面](giao-dien.md)）。
- **带标志的向导**：`ttm project init`、`ttm agent create`、`ttm mcp-server add` 等在终端中提问，CI 中用 `--yes`。
- **CLI 中的 Factory v2**：工作流、自动化、agent、skill、MCP 服务器、变更请求、收件箱、模板（[Factory](factory.md)）；`.ttm/` 中的
  文件化配置（[配置即代码](as-code.md)）。
- **MCP v2**：`outputSchema`、统一的 Markdown 格式、带两步确认的管理工具（[MCP 工具](mcp-cong-cu.md)）。
- **Runner**：提交使用 agent 的身份（绝不出现 Claude/Anthropic 行）、经审核的 skill、经网关的 MCP 和本地 MCP（[Runner](runner.md)）。

## 继续阅读

- 先看 [安装](cai-dat.md)，再看 [快速开始](bat-dau-nhanh.md)（5 分钟）。
- [连接 MCP](mcp.md)，用于 Claude、Cursor、Codex。
- 需要查阅时参见 [命令参考](lenh.md) 和 [MCP 工具](mcp-cong-cu.md)。
