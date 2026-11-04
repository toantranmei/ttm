---
title: 快速开始
description: 五分钟完成安装、登录、选择工作区、读取 Notee 页面，并把 MCP 添加到 Claude Code。
order: 3
section: start
---

准备：一个 [toantranmei.com](https://toantranmei.com) 账号和一个终端。示例使用 macOS/Linux；在 Windows 上请使用
PowerShell，`ttm` 命令相同。

## 1. 安装

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
ttm --version
```

其他渠道（Homebrew、npm、MSI）：见 [安装](cai-dat.md)。

## 2. 登录

```sh
ttm login
```

```text
Open this URL in your browser / Mở URL này trong trình duyệt:
  https://toantranmei.com/ket-noi/cli?response_type=code&client_id=ttm-cli&redirect_uri=http%3A%2F%2F127.0.0.1%3A53124%2Fcallback&…
Waiting for the browser (5 minutes)… On a machine without a browser use `ttm login --device`.
```

浏览器会打开 **“ttm CLI 想要访问你的账号”** 页面，列出所申请的权限。如果你属于多个工作区，页面上会有
**限定在工作区内** 一栏（留空 = 所有工作区）。点击 **允许**；页面会跳转回 `127.0.0.1` 并显示
“Đã đăng nhập, quay lại terminal.”（越南语）和 “Signed in. You can close this tab and return to the terminal.”

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

正在使用 SSH 或在容器中？请使用 `ttm login --device`（见 [登录与 token](dang-nhap.md)）。

## 3. 检查

```sh
ttm whoami
```

```text
An Nguyễn <an@example.com>
host: https://toantranmei.com  profile: default
```

## 4. 选择工作区

```sh
ttm workspace list
```

```text
  Cá nhân  personal · owner · 0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
  Team Aurora [aurora]  team · member · 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

```sh
ttm workspace switch aurora
```

```text
Current workspace for profile `default`: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

`switch` 接受 id、slug、完整名称或名称开头部分（不区分大小写），只要它只匹配一个工作区。所选工作区会保存到 profile
中，`ttm notes search`、`ttm project init`、`ttm-mcp` 都以它为默认值。

## 5. 查找并读取页面

```sh
ttm notes search "ke hoach"
```

```text
0190f3b0-1111-7aaa-8bbb-0123456789ab  Kế hoạch quý 4
0190f3b0-2222-7aaa-8bbb-0123456789ab  Họp tuần
    …chốt kế hoạch phát hành bản 0.2 trước ngày 15…
```

搜索不区分声调符号（"ke hoach" 能找到 "Kế hoạch"）；缩进的行是匹配文字位于正文时显示的摘录。

```sh
ttm notes get 0190f3b0-1111-7aaa-8bbb-0123456789ab
```

```markdown
# Mục tiêu
- Phát hành ttm 0.2
<callout icon="💡" color="yellow">
	Ưu tiên tài liệu tiếng Việt
</callout>
```

也可以直接粘贴页面的 URL 代替 id。保存到文件：`ttm notes get <id> -o ke-hoach.md`。格式详情：
[Notee-flavored Markdown](notee-markdown.md)。

## 6. 把 MCP 添加到 Claude Code

```sh
ttm mcp install --client claude-code
```

```text
Claude Code now starts `/Users/an/.cargo/bin/ttm-mcp` as MCP server `ttm`.
Restart the client to load it. Sign in first with `ttm login` if you have not.
```

该命令会执行 `claude mcp add --scope user ttm -- /Users/an/.cargo/bin/ttm-mcp`。重新打开 Claude Code，输入 `/mcp`
即可看到服务器 `ttm`，然后试着提问：

> 在 Notee 中找到页面“Kế hoạch quý 4”，并总结三个要点。

Claude 会用你的 token 先调用 `notee-search`，再调用 `notee-fetch`。其他应用（Claude Desktop、Cursor、Codex、VS Code）
以及远程 MCP：见 [连接 MCP](mcp.md)。

## 7. 创建 Factory 项目（可选）

在你的仓库中：

```sh
ttm project init
```

向导会询问名称、key、工作区、仓库、工作流、预设 agent 和 runner；每个问题都有对应标志。在脚本或 CI 中，传入全部标志和 `--yes`：

```sh
ttm project init --name "Web todo" --key WEB --workflow balanced --agents create --runner this --yes
```

然后运行 `ttm runner up`，让本机接收 agent 的作业。见 [Factory](factory.md) 和[命令行界面](giao-dien.md)。

## 下一步

- [Notee](notee.md)：编辑页面、修改建议、查询数据库、评论。
- [Factory](factory.md)：把仓库关联到项目、同步计划、为 agent 设定规则。
- [Runner](runner.md)：让 agent 在你的机器上完成任务。
- 出了问题？先运行 `ttm doctor`，再查看 [故障排查与常见问题](xu-ly-su-co.md)。
