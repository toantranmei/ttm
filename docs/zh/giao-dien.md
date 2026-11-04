---
title: 命令行界面
description: ttm 0.2 的输出约定：结果行、Next、状态符号、徽章、颜色、plain 与 JSON 模式、宽度。
order: 19
section: reference
---

从 0.2.0 起，`ttm` 的所有命令都通过同一个库（`ttm-ui`）输出，因此各处的样子一致。命令和输出始终为英文；你的内容（任务标题、
页面名称等）原样显示。

## 每个命令都遵循同一模式

1. **第一行是结果**：`✓ Signed in as An <an@example.com>`、`! Plan out of sync…`、`2 workspace(s), current: Aurora`。
2. **详情**：表格、键值对、差异。
3. **最后一行建议下一条命令**：`Next: ttm project init`。

错误以同一模式输出到 stderr：

```text
✕ updating AUR-12: AUR-12 changed since rev 4.
  rule: rev-conflict  field: rev
  Fix: someone changed it meanwhile: fetch it again (ttm sync) and redo the change
  Help: https://toantranmei.com/help/ttm/ma-loi#rev-conflict
```

`Help:` 指向[错误码](ma-loi.md)中该 rule 的条目。

## 状态符号

| 符号 | `--plain` | 含义 | 类别 |
| --- | --- | --- | --- |
| `○` | `todo` | 未开始 | `inbox`、`refinement`、`ready` |
| `●` | `doing` | 进行中 | `ux-design`、`in-progress` |
| `◐` | `review` | 审核中 / 等待 | `design-review`、`qc`、`approval`（黄色 = 等人处理） |
| `✓` | `done` | 完成 | `done` |
| `✕` | `blocked` | 失败 / 被阻止 | `blocked` |
| `⏸` | `paused` | 暂停 | `paused` |

检查结果（`ttm doctor`、`ttm status`）：`✓` 正常，`!` 警告，`✕` 失败（`--plain`：`ok`、`warn`、`FAIL`）。提示框：`✓` 成功，
`!` 警告，`✕` 错误，`›` 说明（`--plain`：`OK:`、`WARNING:`、`ERROR:`、`NOTE:`）。

## 徽章

| 徽章 | `--plain` | 含义 |
| --- | --- | --- |
| `🤖` | `[agent]` | Agent |
| `👤` | `[person]` | 人 |
| `· Supporter` | `[Supporter]` | Supporter 会员 |
| `✦ Mei-er` | `[Mei-er]` | Mei-er 会员 |
| `✦✦ Sponsor` | `[Sponsor]` | Sponsor 会员 |

会员徽章始终为金色（该颜色专属 Mei-er）。所有者是会员的工作区中的 agent 显示所有者的徽章。

## 颜色

| Token | 颜色 | 用途 |
| --- | --- | --- |
| teal | `#5bb7a8` | 成功、运行中 |
| yellow | `#f2b84b` | 等待、警告 |
| coral | `#e97962` | 错误、被阻止 |
| purple | `#b597d6` | 审核、设计 |
| blue | `#55a4cf` | 信息、链接、建议的命令 |
| gold | `#d4a72c` | 仅 Mei-er |
| ink | 粗体，终端文字颜色 | 强调 |
| muted | 淡色 | 次要文字 |

支持真彩色的终端（`COLORTERM=truecolor|24bit`、Windows Terminal）使用精确颜色；`TERM=*256color*` 使用最接近的 256 色；其余使用
16 种基本色。

## 模式

| 情况 | 输出 |
| --- | --- |
| stdout 是终端 | 颜色、边框、符号、emoji；spinner 在 stderr |
| 管道、`NO_COLOR`（非空）、`TERM=dumb` | 无颜色，保留边框和符号 |
| `CLICOLOR_FORCE=1` | 即使在管道中也有颜色（`NO_COLOR` 优先） |
| `--plain` 或 `TTM_PLAIN=1` | 无颜色、边框、符号、emoji：用文字代替符号（适合日志） |
| `--json` | stdout 仅输出 JSON（保持 0.1.6 结构，只新增键）；错误仍在 stderr |

表格从不换行：过长的单元格按显示宽度以 `…` 截断（带声调的越南文、中文和 emoji 都能正确计算）。详情行中的路径和 URL 不会被截断，
便于复制。

## 宽度

`TTM_COLUMNS` > `COLUMNS` > 终端大小 > 80，限制在 40–200 列。

## 向导与 `--yes`

设置类命令（`ttm project init`、`ttm agent create`、`ttm mcp-server add` 等）在终端中逐个提问；**每个问题都有对应的标志**。
已给出的标志不会再问。`-y/--yes` 采用默认值并自动确认；缺少必需标志时报 `missing_flag`（退出码 2）。不是终端且没有 `--yes` →
`missing_flag`（"pass --workflow or run in a terminal"）或 `confirmation_required`。向导结束后，`ttm` 在 stderr 打印带全部标志的
等价命令，便于粘贴到脚本：

```text
Equivalent: ttm project init --name 'Web todo' --key WEB --workflow balanced --agents create --runner this --yes
```

密钥永远不会作为标志值：使用 `--secret-env 变量名`（读取环境变量）或在终端中隐藏输入。

## 实时输出

只有 `ttm inbox --watch` 和 `ttm agent logs -f` 持续输出（每个事件一行，没有全屏界面）。加 `--json` 时两者输出 NDJSON（每行一个对象）。
