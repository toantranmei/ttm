---
title: 安全
description: token 如何存储和保护、最小 scope、撤销、服务器能看到和看不到什么、runner 隔离，以及如何报告漏洞。
order: 15
section: more
---

## Token

- **明确的前缀**：`ttm_pat_`（个人 token）、`ttm_oat_`（登录 token、OAuth 应用）、`ttm_ort_`（refresh）、
  `ttm_job_`（runner 和 agent 的单次运行）。不要把 token 粘贴到聊天、issue、commit 或 CI 日志中。token 泄露时：立即在
  `<host>/me/ket-noi` 撤销。
- **服务器只保存 token 的哈希值**（SHA-256）；原始 token 只在创建时显示一次。
- **本机存储**：操作系统的 keyring（macOS Keychain、Windows Credential Manager、Secret Service），服务名为 `ttm`。
  没有 keyring 时使用权限为 **0600** 的 `credentials.toml`，先写入已设好权限的临时文件再重命名，因此任何时刻
  其他人都无法读取。
- **绝不写入日志**：所有显示 token 的地方只保留前缀和最多 4 个字符（`ttm_pat_Ab12…`），即使在 `-vv` 下也是如此。
- **`TTM_TOKEN`** 优先于其他所有来源：便于 CI 使用，但环境变量可能被子进程读取；只在需要它的
  步骤中设置。
- token 通过 `Authorization: Bearer` header 发送。带 token 的请求**不使用 cookie**，因此不受 CSRF 影响。

## 最小权限

- 选择刚好够用的 scope：只读取 Notee 的脚本需要 `workspace:read notee:read`；检查计划的 CI 需要 `factory:read`。
- 创建 PAT 时，或在授权登录时的 *限定工作区* 字段中，**把 token 限定到单个工作区**。
- 希望 agent 只**建议**修改而不直接写入：授予 `notee:comment` 而不授予 `notee:write`，或者为工作区设置
  `agent_write_mode = suggest`。
- 实际权限始终是 scope 与你对资源权限的交集：token 永远不能做超出你本人权限的事。
- 敏感操作无论 scope 如何都**无法通过 token 完成**：创建/列出 token、共享、付款、删除页面、创建
  工作区、管理操作（`token_not_allowed`）。

## 有效期与撤销

- CLI 登录 token 有效期 7 天，通过 90 天的 refresh token 自动续期（PAT 1–365 天，OAuth 应用 1 小时，runner 12 小时，
  作业（job）的 token 随作业时长而定）。
- refresh token 每次使用都会**轮换**；重复使用旧的 refresh token（被盗的迹象）会撤销整个 token 家族。
- `ttm logout` 先在服务器上撤销，然后才删除本机副本。
- `/me/ket-noi` 页面显示每个 token 的最后使用时间；已连接的应用有 **断开连接** 按钮，可撤销该应用的所有 token；
  **Runner** 标签页可远程关闭 runner。

## 登录

- 浏览器流程使用 **PKCE S256** 和随机的 `state`；回调页面只监听 `127.0.0.1`，忽略所有 `state` 不正确的
  请求，并且只存活到收到授权码为止（最长 5 分钟）。
- Device flow：8 位代码有效期 10 分钟，只能使用一次；`/activate` 页面让你在授权前查看应用名称和权限。
- 外部应用（远程 MCP）的授权页面会明确标注 *外部应用*，显示浏览器将返回的地址，并且
  **在应用名称与官方应用相似时发出警告**。只有在你本人刚刚连接该应用时才点击 **允许**。
- OAuth token 绑定到确定的资源 `https://toantranmei.com/api/v1/mcp`；服务器拒绝为其他资源签发 token。

## 服务器能看到什么

| 服务器能看到 | 服务器看不到 |
| --- | --- |
| 你（或你的 agent）发送的所有 API 请求：读取、修改、查询的页面，评论 | 你的 Claude API key 或登录会话 |
| token 的最后使用情况（时间、IP、User-Agent） | 本机上 agent 未读取或未上传的文件 |
| run 的事件：agent 的思考、操作、使用的命令和文件路径、测试结果、token 数 | `config.toml`、keyring、`credentials.toml` 的内容 |
| 来自 hook 的规则检查结果（`rule_check`） | 你的仓库，agent 推送到你的 remote 的分支除外 |
| runner 的机器名、操作系统、版本、harness | |

每个步骤的 prompt 由服务器拼装并下发到你的机器以运行 Claude Code；生成 prompt 的逻辑在服务器上。Claude Code
使用你的账号直接与 Anthropic 通信，不经过 toantranmei.com。

## Runner

- 只有**出站连接**（heartbeat 和 long-poll），不开放任何端口。
- 只在**你声明的克隆**（`ttm runner config --repo`）中工作，并检查克隆的 `origin` 与作业的仓库一致；
  绝不克隆服务器发来的 URL。分支名在使用前会经过检查。
- **每个任务一个 worktree、一个分支**；你自己的 worktree 不受影响。`pre-bash` hook 会阻止直接 push 到受保护分支
  （默认是项目的默认分支）、`--no-verify` 以及不符合约定的 commit。
- worktree 始终位于 runner 的 `worktree_root` 之下：作业的 `worktree_hint` 只是相对路径；绝对路径、`..` 或指向外部的
  符号链接都会被拒绝（`invalid_worktree`）。
- 作业的 token 只能用于它自己的 run，保存在权限为 0600 的 MCP 配置文件中，run 结束后即被删除。
- agent 以**你的操作系统用户权限**运行，受作业所允许的工具列表限制。这不是
  沙箱：只为你信任的项目运行 runner，合并前审查 pull request，并考虑使用单独的操作系统
  账户或在虚拟机中运行 runner。
- Kill switch：网页上的停止按钮或 `ttm agent stop <run>`；runner 会在 ≤ 20 秒内收到指令。
- **提交中没有 Claude/Anthropic 行**：每个作业的 git 都经过该 run 专用的 `commit-msg` hook，它会删除 `Co-Authored-By: Claude…`、
  `Generated with Claude…`、`noreply@anthropic.com` 等行；Claude Code 运行时关闭提交和 PR 中的署名。agent 的提交使用 agent 自己的
  名字和邮箱（不是你的，也不是 Anthropic 的）。
- **只使用在网页上审核过的 skill**，通过 10 分钟有效的签名 URL 下载，按 SHA-256 校验（不一致则拒绝作业），安全解压，从不提交
  （`info/exclude`），run 结束后删除。
- **本地 MCP 默认关闭**：只有 `allow` 中的 agent 可用，别人的作业仅在 `team_jobs = true` 时可用。服务器只知道名称、`allow` 和
  `team_jobs`；命令、参数和环境变量（通过 `env:VAR` 的密钥）不离开本机。
- **同事的作业看不到你的 Claude 配置**（`--setting-sources project,local`，用 `ttm runner config claude-token` 保存 token 后还会使用
  空的 `CLAUDE_CONFIG_DIR`）。

## MCP 服务器与密钥

远程 MCP 服务器的密钥（OAuth token、密钥 header）**加密保存在服务器上**，只在服务器的 MCP 网关中使用。agent 用作业 token 调用网关；
密钥永远不会到达 runner 所在机器，也不会进入 prompt。用 `ttm mcp-server add` 添加服务器时，密钥只能通过 `--secret-env 变量名` 或在
终端中隐藏输入，绝不作为标志值（避免进入 shell 历史）。

## 通过 MCP 管理：两步确认

- 管理工具（修改工作流、自动化、agent，审核 skill，安装模板，批准任务的人工关卡）需要 `factory:admin` scope，位于 `admin` 工具组。
- **运行作业的 agent 永远拿不到这些工具**：作业 token 没有 `factory:admin`，设置了 `TTM_RUN_ID` 时 `ttm-mcp` 去掉 `admin` 工具组，服务器
  在所有确认端点拒绝作业 token（`job_token_not_allowed`）。
- 每个管理工具只**预览并创建一个确认**（`confirm_id`，10 分钟有效）；只有你同意且客户端调用 `factory-confirm` 后才会生效。服务器
  以你自己的权限原样重放预览过的请求，每个确认只执行一次，确认后被改动的请求会被拒绝（`confirm_tampered`）。

## 安全更新

`ttm self-update` 拒绝安装没有 checksum 的版本，校验每个压缩包的 SHA-256，并在有 `gh` 时校验 **GitHub artifact attestation**
（Sigstore）；attestation 不正确则中止。手动校验：[安装](cai-dat.md)。

二进制文件尚未经过 Apple 或 Microsoft 的代码签名；脚本、Homebrew、npm 渠道不会添加 quarantine 标记。源代码开源（MIT），位于
<https://github.com/toantranmei/ttm>。

## 报告漏洞

请私下发送邮件至 **toantran.mei@gmail.com**，附上描述、复现步骤和版本（`ttm --version`）。**不要为漏洞创建公开
issue**。我们会在 72 小时内回复，告知修复进度，并在你愿意的情况下在发布说明中致谢。
