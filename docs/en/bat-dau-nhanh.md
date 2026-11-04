---
title: Quickstart
description: Five minutes from installing to signing in, choosing a workspace, reading a Notee page and adding MCP to Claude Code.
order: 3
section: start
---

You need: an account on [toantranmei.com](https://toantranmei.com) and a terminal. The examples use macOS/Linux; on
Windows, use PowerShell with the same `ttm` commands.

## 1. Install

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
ttm --version
```

Other channels (Homebrew, npm, MSI): see [Installation](cai-dat.md).

## 2. Sign in

```sh
ttm login
```

```text
Open this URL in your browser / Mở URL này trong trình duyệt:
  https://toantranmei.com/ket-noi/cli?response_type=code&client_id=ttm-cli&redirect_uri=http%3A%2F%2F127.0.0.1%3A53124%2Fcallback&…
Waiting for the browser (5 minutes)… On a machine without a browser use `ttm login --device`.
```

Your browser opens the page **"ttm CLI wants to access your account"**, listing the permissions requested. If you belong
to several workspaces, there's a **Limit to workspace** field (leave it empty = all workspaces). Click **Allow**; the page
redirects to `127.0.0.1`, which says "Đã đăng nhập, quay lại terminal." (Vietnamese) and "Signed in. You can close this tab
and return to the terminal."

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

Over SSH or in a container? Use `ttm login --device` (see [Signing in and tokens](dang-nhap.md)).

## 3. Check

```sh
ttm whoami
```

```text
An Nguyễn <an@example.com>
host: https://toantranmei.com  profile: default
```

## 4. Choose a workspace

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

`switch` accepts an id, a slug, the full name or the start of the name (case-insensitive) if it matches only one
workspace. The choice is saved to the profile, and `ttm notes search`, `ttm project init` and `ttm-mcp` use it as the
default.

## 5. Find and read a page

```sh
ttm notes search "ke hoach"
```

```text
0190f3b0-1111-7aaa-8bbb-0123456789ab  Kế hoạch quý 4
0190f3b0-2222-7aaa-8bbb-0123456789ab  Họp tuần
    …chốt kế hoạch phát hành bản 0.2 trước ngày 15…
```

Search ignores diacritics ("ke hoach" finds "Kế hoạch"); an indented line is a snippet, shown when the match is in the
page content.

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

You can paste the page's URL instead of the id. Save to a file: `ttm notes get <id> -o ke-hoach.md`. Format details:
[Notee-flavored Markdown](notee-markdown.md).

## 6. Add MCP to Claude Code

```sh
ttm mcp install --client claude-code
```

```text
Claude Code now starts `/Users/an/.cargo/bin/ttm-mcp` as MCP server `ttm`.
Restart the client to load it. Sign in first with `ttm login` if you have not.
```

The command runs `claude mcp add --scope user ttm -- /Users/an/.cargo/bin/ttm-mcp`. Reopen Claude Code, type `/mcp` to
see the `ttm` server, then try asking:

> Find the page "Kế hoạch quý 4" in Notee and summarize its three main points.

Claude calls `notee-search` and then `notee-fetch` with your token. Other apps (Claude Desktop, Cursor, Codex, VS Code)
and remote MCP: see [Connecting MCP](mcp.md).

## Next steps

- [Notee](notee.md): edit pages, suggest edits, query databases, comment.
- [Factory](factory.md): link a repo to a project, sync the plan, rules for agents.
- [Runner](runner.md): let agents work on tasks on your machine.
- Something wrong? Run `ttm doctor`, then see [Troubleshooting and FAQ](xu-ly-su-co.md).
