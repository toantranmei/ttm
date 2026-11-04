---
title: Notee-flavored Markdown
description: The full spec of the Notion-style Markdown that ttm, ttm-mcp and the API use to read and write Notee pages.
order: 12
section: reference
---

Notee pages are read (`GET /pages/{id}/markdown`, `ttm notes get`, `notee-fetch`) and written (`PATCH`, `ttm notes edit`,
`notee-update-page`, `notee-create-pages`) as **Notee-flavored Markdown** (NFM): regular Markdown plus a few XML-style
tags for blocks that Markdown cannot express. Tag names follow Notion-flavored Markdown, so agents that know Notion will
feel at home in Notee.

## General rules

- **One line is one block.** Two consecutive lines are two separate paragraphs (they are not merged as in CommonMark).
  Write an empty paragraph as `<empty-block/>`.
- **Children are indented by one tab** (`\t`): list sub-items and the content of container tags (`<callout>`,
  `<details>`, `<columns>`, `<column>`, `<table>`, `<tr>`). When reading input, the server also accepts 2-space
  indentation; in output, the server always uses tabs.
- On write, the server rebuilds the block tree from the Markdown, compares it with the current document and applies
  **only the differences** (a block-level diff), so people who have the page open keep their cursor.
- Blocks nest at most 64 levels deep.

## Blocks

| Block | NFM |
| --- | --- |
| Paragraph | A line of text |
| Empty paragraph | `<empty-block/>` |
| Heading 1–6 | `#` … `######` + a space |
| Bulleted list | `- item` |
| Numbered list | `1. item` (the first number is the start number) |
| To-do | `- [ ] not done`, `- [x] done` |
| Quote | `> text` (consecutive `>` lines form one quote) |
| Code block | A ```` ``` ```` or `~~~` fence with a language: ```` ```rust ```` |
| Mermaid diagram | A code block with language `mermaid` |
| Equation (block) | `$$` on its own line, LaTeX, closing `$$`; or `$$x^2$$` on one line |
| Divider | `---` |
| Image | `![caption](url)` (also accepts `<image src="url">caption</image>`) |
| Callout | `<callout icon="💡" color="yellow">` … `</callout>` |
| Toggle | `<details>` + `<summary>title</summary>` + content … `</details>`; toggle heading: `<details level="2">` |
| Columns | `<columns>` containing `<column>` … `</column>` |
| Table | `<table header-row="true" header-column="false">` containing `<tr>` containing `<td>`; also accepts GFM tables `\| a \| b \|` |
| Child page | `<page url="https://toantranmei.com/notes/<id>">Title</page>` |
| Inline database | `<database url="…/notes/<id>" inline="true">Title</database>` |
| File | `<file src="url">name.zip</file>` |
| Video, audio, PDF | `<video src="…">name</video>`, `<audio src="…">name</audio>`, `<pdf src="…">name</pdf>` |
| Table of contents | `<table_of_contents/>` |
| Unsupported block (sketch, embed, synced block…) | `<unknown url="…?block=<id>" alt="sketch"/>` |
| Rest of the page was cut off | `<truncated url="…?block=<id>" blocks="n"/>` (last line) |

### Full example

```markdown
# Q4 plan
The main goal is to ship **ttm 0.2**.
<empty-block/>
- Notee
	- Suggested edits
	- SQL queries
- [x] Write the docs
- [ ] Release
> Note: the deadline is the 15th.
<callout icon="⚠️" color="red">
	Do not release on a Friday.
</callout>
<details>
	<summary>Risk details</summary>
	Not enough reviewers.
</details>
<columns>
	<column>
		Left
	</column>
	<column>
		Right
	</column>
</columns>
<table header-row="true" header-column="false">
	<tr>
		<td>Task</td>
		<td>Owner</td>
	</tr>
	<tr>
		<td>Changelog</td>
		<td><mention-user url="user://0190f3a1-…">Bình</mention-user></td>
	</tr>
</table>
<page url="https://toantranmei.com/notes/0190f3b5-…">Meeting notes</page>
<database url="https://toantranmei.com/notes/0190f3c0-…" inline="true">Team tasks</database>
$$
E = mc^2
$$
<table_of_contents/>
```

## Inline formatting

| Formatting | NFM |
| --- | --- |
| Bold, italic, strikethrough | `**bold**`, `*italic*`, `~~strikethrough~~` |
| Code | `` `code` `` |
| Link | `[text](https://…)` |
| Underline | `<span underline="true">text</span>` |
| Text color | `<span color="red">text</span>` |
| Background color | `<span color="red_bg">text</span>` (also accepts the `_background` suffix) |
| Equation | `$x^2$` |
| Line break within a block | `<br>` |
| Mention a person | `<mention-user url="user://<id>">Name</mention-user>` |
| Mention a page | `<mention-page url="https://toantranmei.com/notes/<id>">Title</mention-page>` |
| Mention a date | `<mention-date start="2026-11-04"/>` |

`<span>` can combine several attributes: `<span underline="true" color="blue">text</span>`. Common colors: `gray`,
`brown`, `orange`, `yellow`, `green`, `blue`, `purple`, `pink`, `red`; `default` means no color.

## Escaping

When the server **returns** Markdown:

- The characters `\`, `*`, `_`, `[`, `]`, `` ` ``, `<`, `~`, `$`, `&` in plain text are escaped with `\`, for example
  `5\*3`, `a\_b`, `\<b>`.
- A paragraph that starts with a block-opening character (`#`, `-`, `+`, `>`, `|`, `=`, or `1.`) is escaped:
  `\# not a heading`, `2026\. is the year`.
- Leading or trailing spaces or tabs in a paragraph are written as `&#32;` / `&#9;`.
- In tag attributes and labels: `&amp;`, `&quot;`, `&lt;`, `&gt;`.

When **writing**, the server understands those forms and the entities `&quot;`, `&#39;`, `&lt;`, `&gt;`, `&#32;`,
`&#9;`, `&amp;`. With `update_content`, `old_str` must match **exactly the text the server returned**, including `\`
characters and tabs.

## Mentions, child pages, files: point only at things that exist

- `<mention-user>` needs `url="user://<user id>"`; the label is the displayed name. Find ids with `workspace-get-users`.
  Missing `url`: dropped, with a warning.
- `<mention-page>`, `<page>`, `<database>` need the URL (or id) of an **existing page or database**. Writing a `<page>`
  tag does not create a new page; create child pages with `notee-create-pages` (or `ttm notes create --parent`). No
  valid URL: dropped, with a warning.
- Page titles, database titles and people's names show only if you have permission to view them; otherwise the tag
  appears with an empty label.
- `<file>`, `<video>`, `<audio>`, `<pdf>`, `<image>` need `src`. Notee files have URLs of the form
  `https://toantranmei.com/api/v1/files/<id>` (downloading requires being signed in). Missing `src`: dropped.
- Tags and blocks that you read and write back **verbatim** (child pages, databases, files, images, mentions) are
  restored exactly as the original, including attributes that Markdown does not show.

## `<unknown>` and `<truncated>`

`<unknown url="…?block=<id>" alt="<type>"/>` stands in for a block that Markdown cannot express (sketch, embed, synced
block…). On write:

- **keep** the tag: the original block is restored from the current document by `url`;
- **delete** the tag: that block is deleted;
- a tag that points to a block not in the page: dropped, with the warning
  `<unknown url="…"> is not a block of this page; left out`.

Pages over **20,000 nodes** are cut off on read: the Markdown stops there and the last line is
`<truncated url="…?block=<id>" blocks="n"/>`, standing in for **every** top-level block from that block to the end of the
page; the result has `truncated: true` and `unknown_block_ids`. Read the rest piece by piece with `block=<id>`
(`ttm notes get --block`, `notee-fetch` with `block`). When writing back:

- keep the `<truncated/>` line at the **end** of the content: the part after the cut stays as is;
- delete it: that part is deleted, so the server refuses (`409 would_delete_truncated`) unless `allow_deleting_content`
  is set;
- a `<truncated/>` line that is not at the end, or appears more than once: dropped, with a warning.

Repeating `<unknown>`/`<truncated>` to copy the same block many times until the page exceeds its size limit is refused
(`413 too_many_blocks`).

## Write limits

| Limit | Value | Error |
| --- | --- | --- |
| Blocks added, changed or deleted in one write (nested blocks included) | 1,000 (deleted blocks do not count with `allow_deleting_content`) | `413 too_many_blocks` |
| Markdown size | 2 MB (after applying `update_content`) | `413 markdown_too_large` |
| Dropping a `<page>` or `<database>` (deletes the child page/database) | requires `allow_deleting_content` | `409 would_delete_children` |
| Dropping `<truncated/>` | requires `allow_deleting_content` | `409 would_delete_truncated` |
| Agent writes | 60 per hour per token | `429 rate_limited` |

To split up a large write: use several small `update_content` edits, or create the page and then add content
gradually.

## Round-trip rules (read, then write back)

1. Read the page, edit the text, write it back: only the blocks you changed are touched; the history records
   “<you> (via agent <name>)”, and the server takes a snapshot before the write so it can be undone.
2. Prefer `update_content` (`old_str` → `new_str`, each `old_str` matching exactly one place, case-sensitive) over
   replacing the whole page: the change is smaller and less likely to lose blocks. Pairs that do not match are skipped
   and returned in `warnings`.
3. When replacing the whole page (`replace_content`), keep every `<unknown/>`, `<truncated/>`, `<page>`, `<database>` tag
   you read.
4. Formatting that Markdown cannot express (for example a block's own attributes) is kept as long as that block does not
   change.
5. Database rows: the body is the row's page content; change column values with `properties` (`notee-update-page`) or
   `notee-upsert-rows`, not through Markdown.

## Comments and suggestions

Comments and suggested edits anchor to a **text quote** of the page (`{exact, prefix?, suffix?}`, in the style of the W3C
`TextQuoteSelector`), so you don't need block ids. `exact` is 1–1,000 characters long and must match exactly one place:
no match gives `anchor_not_found`, several matches give `ambiguous_anchor` (add a few words of `prefix` or `suffix`). See
[Notee](notee.md).
