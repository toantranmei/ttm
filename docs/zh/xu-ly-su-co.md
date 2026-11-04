---
title: 故障排查与常见问题
description: 如何诊断问题，安装、登录、使用 MCP、Notee、Factory、runner 时按症状分类的常见错误，以及常见问题。
order: 16
section: more
---

## 先诊断

```sh
ttm doctor            # host、discovery、版本、token、keyring、git、claude、ttm-mcp
ttm status --json     # host、profile、token 来源、当前工作区
ttm -vv whoami        # 请求的 debug 日志（token 已遮蔽）
```

在 GitHub 上报告问题时，请附上 `ttm doctor` 的输出、`ttm --version`、操作系统以及安装渠道。

## 安装

**`brew install` 提示 Xcode 版本过旧。**
Xcode 过旧时 Homebrew 会拒绝安装。请在 App Store 中更新 Xcode，或卸载 Xcode 改用 Command Line Tools：
`sudo rm -rf /Applications/Xcode.app && xcode-select --install`。

**用 npm 安装后，`ttm` 提示找不到二进制文件。**
npm 跳过了 `postinstall` 脚本（npm 11 会对全局安装包的脚本发出警告）。重新安装并只允许这两个包运行脚本：
`npm install -g --allow-scripts=@toantranmei/cli,@toantranmei/mcp @toantranmei/cli @toantranmei/mcp`。

**运行安装脚本后出现 `command not found: ttm`。**
打开新的终端，或执行 `source ~/.cargo/env`。检查 `~/.cargo/bin` 是否在 `PATH` 中。

**ARM 上的 Windows。**
没有 ARM 版本；请安装 x64 版本（PowerShell、MSI 或 npm），Windows 会通过模拟运行它。

**macOS 拦截了用浏览器下载的二进制文件。**
`xattr -d com.apple.quarantine ./ttm ./ttm-mcp`，或者改用脚本/Homebrew 安装。

**Windows SmartScreen 对 MSI 发出警告。**
二进制文件没有数字签名。先校验 SHA-256 和 attestation（[安装](cai-dat.md)），再选择 **More info → Run anyway**。

**`ttm self-update` 提示 `cannot write into …`。**
你没有安装目录的写权限（通常是因为用 Homebrew、npm、MSI 安装）。请用对应的包管理器更新。

**`self-update` 提示 `GitHub answered 403`。**
触发了 GitHub 匿名 API 的限流。稍后重试，或用脚本重新安装。

## 登录

**`ttm login` 打开了浏览器，但一直无法完成。**
你正通过 SSH 登录另一台机器（回调页面在那台机器的 `127.0.0.1` 上），或防火墙拦截了本地端口。请使用
`ttm login --device`。

**`timed out after 5 minutes waiting for the browser`。**
重新运行，并在 5 分钟内点击 **允许**。使用 `--no-browser` 可自行打开 URL。

**`the code expired before it was approved`。**
设备码有效期为 10 分钟。重新运行 `ttm login --device`。

**明明已登录，却出现 `` not signed in to …; run `ttm login` ``。**
token 绑定在（profile, host）上。你可能正在使用另一个 profile 或 host：查看 `ttm status`。`TTM_HOST`、`TTM_PROFILE`、
`--host` 都会改变这一组合。

**过一段时间后出现 `invalid_token`。**
token 已过期（对 `ttm login` 而言：超过 90 天未使用 `ttm`，refresh token 已过期）或已被撤销。重新 `ttm login`。如果 `ttm status` 显示来源是
`TTM_TOKEN`，说明该变量保存的是旧 token：删除或替换它。

**`insufficient_scope … [requires scope X]` / `{"required": "X"}`。**
token 缺少 scope X。PAT：新建一个带 X 的 token。用 `ttm login --scope` 缩小了范围：重新运行 `ttm login`（默认申请所有 scope）。

**`keyring unavailable …; saving the token to credentials.toml`。**
机器上没有 keyring（Linux 没有 Secret Service、容器）。token 仍会保存在权限为 0600 的文件中。设置 `TTM_NO_KEYRING=1`
可关闭该警告。

**`ttm doctor` 提示 `token … valid but lacks workspace:read`。**
PAT 没有 `workspace:read`：Notee/Factory 命令仍可运行，但 `whoami`、`workspace list` 不行。

## MCP

**应用看不到 `ttm` 服务器。**
`ttm mcp install` 之后请完全重启应用。检查配置中的 `command` 是否为存在的绝对路径（图形界面应用看不到 shell 的
`PATH`）。Claude Code：`claude mcp list`。

**所有工具都返回 `unauthorized`。**
`ttm-mcp` 找不到 token。用 MCP 配置所使用的同一 profile/host 运行 `ttm login`（默认是 `default_profile`）；如果配置中带有
`--profile`/`--host`，就用相同的值登录。没有 keyring 的机器：在 MCP 配置的 `env` 中加入 `TTM_TOKEN`。

**`ttm mcp install` 提示 `the existing config is not valid JSON; fix it first`。**
应用的配置文件已损坏；ttm 不会写入任何内容。修好文件（如果之前安装过，旧备份位于 `<file>.bak-<số>`）后重新运行。

**``Claude Code (`claude`) is not on PATH``。**
安装 Claude Code，或手动运行提示中附带的命令（`claude mcp add --scope user ttm -- …`）。

**`factory-*` 工具返回 `no_run`。**
这些工具是给 runner 一次运行中的 agent 使用的。在 run 之外，只有 `factory-get-task`、`factory-get-context`（传入
`task`）和 `factory-get-rules`（传入 `project`）可用。

**模型选错工具或占用大量 context。**
减少工具数量：`ttm-mcp --toolsets notee`（或 `ttm mcp install --toolsets notee`）。

**在 Claude.ai 中连接远程 MCP 失败。**
检查 URL 是否为 `https://toantranmei.com/api/v1/mcp`。先在网页上登录，再重新点击 **Connect**。误点了拒绝：重新连接。
在 `/me/ket-noi` → **已连接的应用** 中删除旧连接。

## Notee

**`ttm notes edit` 打印 `Page updated.`，但没有按预期修改，并附带 `warning: … old_str not found; skipped`。**
`--old` 必须与服务器返回的 Markdown 精确匹配，区分大小写，包括转义字符（`\*`、`\_`）和缩进用的 tab。用
`ttm notes get` 重新读取，并原样复制那一段。

**`old_str matches N places`。**
加上前后的文字使其只匹配一处，或使用 `--replace-all`。

**没有加 `--suggest`，编辑页面却变成了修改建议（`Suggestion … created`）。**
工作区设置了 `agent_write_mode = suggest`，或 token 只有 `notee:comment`。由有编辑权限的人用
`ttm notes resolve` 审核。

**使用 `--replace` 时出现 `would_delete_children`。**
新内容缺少子页面的 `<page>`/`<database>` 标签。保留这些标签，或者如果确实要删除，加上 `--allow-deleting-content`。

**`too_many_blocks`。**
单次写入修改了超过 1,000 个 block。拆成多次较小的 `--old/--new`。

**agent 连续编辑时出现 `rate_limited`。**
每个 token 每小时可写入 Notee 60 次。把修改合并到更少的写入中。

**评论时出现 `ambiguous_anchor`。**
摘录出现了多次；加上 `--prefix`/`--suffix`（MCP：`prefix`/`suffix`）。

**SQL 提示 `sql_table: Table 'Rows' is not allowed`。**
表名是小写的 `rows`，不加引号。列名需要用双引号：`"Trạng thái"`。

**`query_timeout`。**
查询超过 3 秒：加上 `WHERE`、`LIMIT`，去掉开销大的函数。

**对数据库行执行 `ttm notes get` 看不到列值。**
列值只出现在 `--json` 中（`properties`）。

## Factory

**`this directory is not linked to a project`。**
在仓库根目录运行 `ttm project link <KEY>`，然后提交 `.ttm/project.toml` 和 `.ttm/plan`。

**`ttm sync --check` 在 CI 中以 1 退出。**
仓库中的计划与服务器不一致。在开发机上运行 `ttm sync` 并提交 `.ttm/plan`，或审批待处理的变更请求（change request）。

**`Conflicts with the server`。**
同一字段在两边都被修改了。修改 `.ttm/plan` 后再次 `ttm sync`，或使用 `ttm sync --resolve`（需要 `claude`）。

**本地修改没有同步到服务器。**
有一个变更请求正在等待审批（`ttm project status` 会显示其 id）；在它被处理之前，`ttm sync` 不会再次提交。
请在网页上审批（创建者不能自己审批）。

**`the ttm markers are broken`。**
`AGENTS.md` 或 `CLAUDE.md` 中只剩下 `<!-- ttm:begin -->` / `<!-- ttm:end -->` 两个标记之一。手动修复后运行
`ttm rules apply`。

**hook 拦截了 commit：`commit message must follow Conventional Commits`。**
使用 `type(scope): 摘要` 的格式，例如 `fix(auth): retry on 503`。项目可以在网页上关闭这条规则。

**`ttm task done` 提示 `conflict`，带有 `approval-required`、`tests-required`…**
工作流阻止了状态切换。见 [错误码](ma-loi.md) 中的表格。

## Runner

**``no agent harness found; install Claude Code (`claude`) first``。**
安装 Claude Code。若以服务方式运行仍然报错：服务使用的是安装时记录的 `PATH`（0.1.3 之前的版本不记录 `PATH`）。请在
`which claude` 能找到它的 shell 中再次运行 `ttm runner install-service`（见 [Runner](runner.md)）。

**作业被拒绝：`refused AUR-12: no local clone is configured for project AUR`。**
`ttm runner config --repo AUR=~/code/aurora`，然后重启 runner。

**`the job's repository … is not the origin of …`。**
该 clone 的 `origin` 不是项目的仓库。修改 remote（`git remote set-url origin …`），或把 `--repo` 指向正确的 clone。

**作业被拒绝：`` refused AUR-12: the job's worktree_hint `…` is an absolute path; worktrees stay under the runner's worktree_root … ``。**
服务器发来的 `worktree_hint` 不在 `worktree_root` 之下（绝对路径、含 `..`，或经符号链接指向外部）。runner 会拒绝它
（`invalid_worktree`），不会创建任何东西；请向服务器管理员反馈。

**`ttm runner down` 之后 runner 又自动运行了。**
是服务（launchd `KeepAlive`、systemd、Task Scheduler）在保持它运行。用 `ttm runner uninstall-service` 才能彻底关闭。

**run 以 `paused` 结束。**
Claude 报告额度用尽；runner 会暂停接收作业直到额度重置，服务器会重新排队这些工作。

**run 成功后没有 pull request。**
需要已登录的 `gh`（`gh auth status`），且仓库有 remote。分支仍会被推送；请手动创建 PR。

**worktree 占满了磁盘空间。**
调小 `--keep-days`，或删除 `~/.ttm/worktrees` 中的目录，然后在仓库中运行 `git worktree prune`。

## 常见问题

**ttm 免费且开源吗？**
是的，MIT 许可，地址 <https://github.com/toantranmei/ttm>。使用限制（agent 数量、并发数…）取决于 toantranmei.com 上
工作区的套餐。

**`ttm-mcp` 和远程 MCP 有什么区别？**
工具相同。`ttm-mcp` 在你的机器上运行，使用 `ttm login` 获得的 token，适合本机应用和 runner。远程 MCP 无需安装，
在 Claude.ai、ChatGPT 中使用 OAuth。

**agent 用的是谁的 Claude token？**
你的：runner 调用你机器上已登录的 Claude Code。服务器从不接收 Claude 的 key，在你机器上运行的 agent 也不会扣除
toantranmei 的额度。

**可以用于自托管服务器或 localhost 吗？**
可以：`--host http://localhost:3000` 或单独的 profile。ttm 从 discovery `/.well-known/ttm` 读取所有地址。

**一台机器上有多个账号？**
每个账号一个 profile：`ttm --profile work login`、`ttm --profile personal login`。用 `default_profile` 选择默认 profile。

**如何在 CI 中使用？**
最小权限的 PAT + `TTM_TOKEN`，例如用 `ttm sync --check` 拦截会让计划不一致的 PR。见 [登录与 token](dang-nhap.md)。

**有 Cargo/crates.io 版本吗？**
尚未发布到 crates.io；可用 `cargo install --git https://github.com/toantranmei/ttm ttm-cli` 从源码安装。

**有 Claude Desktop 的 `.mcpb` 包吗？**
有，从 0.1.3 起：在 GitHub Releases 下载 `ttm-mcp-<操作系统>.mcpb`（见 [MCP](mcp.md)）。更早的版本请使用
`ttm mcp install --client claude-desktop` 或远程 MCP。

**我的数据会被用来训练模型吗？**
ttm 不会把数据发送给第三方。你交给 Claude、ChatGPT… 的内容遵循相应提供商的政策。
