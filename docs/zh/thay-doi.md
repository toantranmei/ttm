---
title: 更新日志
description: ttm 与 ttm-mcp 的版本历史。
order: 18
section: more
---

`ttm` 和 `ttm-mcp` 以相同版本发布。下载：<https://github.com/toantranmei/ttm/releases>。

## 0.1.5（2026-11-04）

0.1.3 和 0.1.4 已打标签但从未发布（打标签的提交禁用了 CI，发布流程没有运行）；0.1.5 是包含以下改动的第一个版本。

### 错误与错误码

- 服务器错误保留 `rule`、`field`、`params`：`ttm` 在错误信息中打印 `rule`/`field`；`--json` 和 MCP 工具错误
  （`structuredContent.error`）还带有 `status`、`rule`、`field`、`params`，可据此按 `split-limit`、`rev-conflict`、
  `approval-required` 分支，或读取 `insufficient_scope` 的 `params.required`。见 [错误码](ma-loi.md)。
- `factory-create-subtasks` 工具说明写出真实规则 `split-limit`（`409 conflict`），不再是 `split_limit`。

### ttm

- `ttm task split` 接受 camelCase 或 snake_case 键（`acceptance_criteria`、`depends_on`，与 MCP 工具一致），并对未知键给出
  警告。
- `ttm task approve --reject` 打印 `rejected`（以前是 `rejectd`）。
- `ttm hook pre-bash` 保护缓存规则中 `hooks.pre_bash.protected_branches` 列出的分支，其次是 `.ttm/project.toml` 中的默认
  分支，再次是 `main`/`master`（以前总是 `main`/`master`）。
- `ttm notes query --page-size` 只接受 1–100，在调用服务器前给出清晰的错误。
- CLI 登录：访问 token 7 天，自动续期；只要至少每 90 天使用一次 `ttm` 就会保持登录（随即将发布的服务器更新生效）。

### Runner

- 服务（launchd、systemd）带上运行 `install-service` 的 shell 的 `PATH`，并补充 `~/.local/bin`、`~/.cargo/bin`、
  `/opt/homebrew/bin`、`/usr/local/bin` 等，从而能找到 `claude`、`gh`、`git`。新增命令 `ttm runner uninstall-service`。
- 作业的 `worktree_hint` 只能是 `worktree_root` 之下的相对路径；绝对路径、`..` 或指向外部的符号链接会被拒绝
  （`invalid_worktree`）。
- 作业 long-poll 使用 `wait=25`（服务器上限）；等待中的请求不再因 heartbeat 而被取消。

### 发布

- GitHub Releases 提供 macOS（universal）、Linux x64/arm64、Windows x64 的 Claude Desktop `.mcpb` 包。manifest 的版本
  跟随 `Cargo.toml`（由 CI 检查）。

## 0.1.2（2026/11/04）

首个发布版本。

### 发布渠道

- shell 安装脚本（macOS、Linux）和 PowerShell 安装脚本（Windows），提供 7 个 target 的压缩包：macOS arm64/x64、Linux arm64/x64（glibc
  和 musl）、Windows x64；Windows x64 另提供 MSI。
- Homebrew：`brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp`。
- npm：`@toantranmei/cli`（命令 `ttm`）、`@toantranmei/mcp`（命令 `ttm-mcp`），通过 trusted publishing 发布。
- 所有文件均提供 SHA-256 和 GitHub artifact attestation。
- 没有 Windows on ARM 版本（x64 版本可通过模拟运行）。

### ttm

- 登录：浏览器（loopback + PKCE）、`--device`（RFC 8628）、`--token` / stdin；`logout` 会在服务器上撤销；
  `whoami`、`status`、`doctor`。
- Profile 与 `--host` / `TTM_HOST`，带缓存的 discovery `/.well-known/ttm`，token 存于 keyring 或权限为 0600 的 `credentials.toml`，
  CI 使用 `TTM_TOKEN`。
- `workspace list/switch/current`；`token list/create/revoke`（打开网页）。
- Notee：`notes search/get/query/comments/create/edit/comment/rows upsert/rows delete/suggestions/resolve`，通过
  `--suggest` 提交修改建议。
- Factory：`project init/link/open/status`，基于变更请求（change request）的 `sync`，供 CI 使用的 `--check`，以及在单独 worktree 中由
  agent 解决冲突的 `--resolve`；`task list/show/claim/done/approve/split`；`rules apply`（`AGENTS.md`/`CLAUDE.md` 中的托管块，
  Claude Code hook）；`hook pre-bash/stop`。
- Runner：`runner up/down/status/config/install-service`（launchd、systemd `--user`、Task Scheduler），Claude Code harness，
  每个任务一个 worktree，实时事件，通过 `gh` push 并创建 PR，可选的合并队列；只为已声明的仓库运行作业（job）
  （`repo_not_configured`、`invalid_branch`），runner token 每 12 小时续期，token 持续被拒绝时停止。
- `agent run/logs/stop`。
- `mcp install --client claude-code|claude-desktop|cursor|codex`（合并配置、备份、`--dry-run`）。
- `self-update`（校验 SHA-256 和 attestation）、`completion`。
- `TTM_HOME` 替代用户主目录（Windows 会忽略 `HOME`）。

### ttm-mcp

- 通过 stdio 运行的 MCP 服务器，共 26 个工具：`workspace` 组（3）、`notee` 组（15）、`factory` 组（8）；resource `notee://spec/markdown`；
  `--toolsets`。
- 与远程 MCP `https://toantranmei.com/api/v1/mcp` 使用同一套工具。

## 0.1.1（已取消）

已打 tag 但未发布：无法在 dist 的 Linux 构建机上为 Windows on ARM 构建 MSI。0.1.2 版本移除了
该 target。

## 0.1.0（未发布）

在加入 MSI 定义之前就打了 tag，导致发布流程无法运行；没有发布任何包。
