---
title: Notee 风格 Markdown
description: ttm 与 MCP 用于读写 Notee 页面的 Notion 风格 Markdown。
order: 5
---

# Notee 风格 Markdown

Notee 页面以标准 Markdown(CommonMark + GFM 表格与任务列表)读写,并为 Markdown 无法表达的块增加少量类 XML 标签。标签名称
沿用 Notion 的 Markdown,了解 Notion 的智能体即可直接使用 Notee。

## 标准 Markdown

| 块 | Markdown |
| --- | --- |
| 标题 1–3 | `#`、`##`、`###` |
| 段落、**粗体**、*斜体*、~~删除线~~、`代码`、[链接](url) | 与常规相同 |
| 无序 / 有序列表 | `- 项`、`1. 项`(缩进 2 个空格表示嵌套) |
| 待办 | `- [ ] 未完成`、`- [x] 已完成` |
| 引用 | `> 文本` |
| 代码块 | 带语言的围栏:```` ```rust ```` |
| 分隔线 | `---` |
| 图片 | `![说明](url)` |
| 公式块 | 独占一行的 `$$ … $$` |
| Mermaid 图 | ```` ```mermaid ```` |

## 标签

| 块 | 标签 |
| --- | --- |
| 标注 | `<callout icon="💡" color="yellow">文本</callout>` |
| 折叠 | `<details><summary>标题</summary>子内容</details>` |
| 分栏 | `<columns><column>…</column><column>…</column></columns>` |
| 表格(富文本单元格) | `<table><tr><td>…</td></tr></table>`(简单表格可用 GFM 竖线语法) |
| 子页面 | `<page url="…">标题</page>` |
| 嵌入数据库 | `<database url="…">标题</database>` |
| 提及用户 | `<mention-user url="…">姓名</mention-user>` |
| 提及页面 | `<mention-page url="…">标题</mention-page>` |
| 提及日期 | `<mention-date start="2026-11-04" end="2026-11-05"/>` |
| 文件 | `<file src="…">名称.pdf</file>`(短时有效的签名 URL) |
| 暂不支持的块(草图等) | `<unknown url="…" alt="sketch"/>` |

## 写入规则

- 原样保留收到的每个 `<unknown …/>` 标签:它代表 Markdown 无法承载的块,写回时会从原文恢复。删除该标签即删除该块。
- 删除 `<page>` 或 `<database>` 标签即删除对应子页面或数据库;未设置 `allow_deleting_content: true` 时服务器会拒绝。
- 每次写入最多新增或修改 **1,000 个块**(嵌套块也计数)。更大的修改请拆分。
- 优先使用 `update_content`(`old_str` → `new_str`,每个 `old_str` 必须恰好匹配一处,区分大小写),而不是替换整页:这样正在
  编辑的人光标不会跳动,历史记录也更简洁。

## 大页面

超过读取上限的页面会返回 `truncated: true` 和 `unknown_block_ids`。用 `notee-fetch`(或 `ttm notes get --block <id>`)
逐段读取缺失部分。

## 评论

评论锚定到页面文本的引文(`{exact, prefix?, suffix?}`)。引文出现多次时服务器返回 `ambiguous_anchor`;请补充几个字的
`prefix` 或 `suffix`。
