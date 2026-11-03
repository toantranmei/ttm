---
title: Notee-flavored Markdown
description: The Notion-style Markdown that ttm and MCP use to read and write Notee pages.
order: 5
---

# Notee-flavored Markdown

Notee pages are read and written as standard Markdown (CommonMark + GFM tables and task lists) plus a few XML-like
tags for blocks Markdown cannot express. Tag names follow Notion's Markdown so agents that know Notion know Notee.

## Standard Markdown

| Block | Markdown |
| --- | --- |
| Heading 1–3 | `#`, `##`, `###` |
| Paragraph, **bold**, *italic*, ~~strike~~, `code`, [link](url) | as usual |
| Bulleted / numbered list | `- item`, `1. item` (indent 2 spaces to nest) |
| To-do | `- [ ] open`, `- [x] done` |
| Quote | `> text` |
| Code block | fenced with the language: ```` ```rust ```` |
| Divider | `---` |
| Image | `![caption](url)` |
| Math block | `$$ … $$` on their own lines |
| Mermaid diagram | ```` ```mermaid ```` |

## Tags

| Block | Tag |
| --- | --- |
| Callout | `<callout icon="💡" color="yellow">text</callout>` |
| Toggle | `<details><summary>Title</summary>children</details>` |
| Columns | `<columns><column>…</column><column>…</column></columns>` |
| Table (rich cells) | `<table><tr><td>…</td></tr></table>` (simple tables may use GFM pipes) |
| Child page | `<page url="…">Title</page>` |
| Embedded database | `<database url="…">Title</database>` |
| Mention a user | `<mention-user url="…">Name</mention-user>` |
| Mention a page | `<mention-page url="…">Title</mention-page>` |
| Mention a date | `<mention-date start="2026-11-04" end="2026-11-05"/>` |
| File | `<file src="…">name.pdf</file>` (signed URL, short-lived) |
| Unsupported block (sketch, …) | `<unknown url="…" alt="sketch"/>` |

## Rules for writing

- Keep every `<unknown …/>` tag you received exactly as it is: it stands for a block that Markdown cannot carry and is
  restored from the original when you write the page back. Removing it deletes that block.
- Removing a `<page>` or `<database>` tag deletes that child; the server refuses unless `allow_deleting_content` is
  true.
- One write may add or change at most **1,000 blocks** (nested blocks count). Split larger edits.
- Prefer `update_content` (`old_str` → `new_str`, each `old_str` matching exactly once, case-sensitive) to replacing the
  whole page: it keeps the cursors of people editing and makes a smaller history entry.

## Large pages

A page longer than the read limit comes back with `truncated: true` and `unknown_block_ids`. Read each missing part
with `notee-fetch` (or `ttm notes get --block <id>`).

## Comments

Comments anchor to a quote of the page text (`{exact, prefix?, suffix?}`). When the quote appears more than once the
server answers `ambiguous_anchor`; add a few words of `prefix` or `suffix`.
