---
title: Notee-flavored Markdown
description: ttm、ttm-mcp 和 API 用于读写 Notee 页面的 Notion 风格 Markdown 的完整规范。
order: 12
section: reference
---

Notee 页面以 **Notee-flavored Markdown**（NFM）格式读取（`GET /pages/{id}/markdown`、`ttm notes get`、`notee-fetch`）和写入
（`PATCH`、`ttm notes edit`、`notee-update-page`、`notee-create-pages`）：即普通 Markdown，加上一些 XML 风格的标签，
用来表示 Markdown 无法表达的 block。标签名沿用 Notion-flavored Markdown，因此熟悉 Notion 的 agent 也会熟悉
Notee。

## 基本原则

- **一行就是一个 block。** 相邻的两行是两个独立的段落（不像 CommonMark 那样合并）。空段落写作
  `<empty-block/>`。
- **子项缩进一个 tab**（`\t`）：列表的子项，以及容器标签（`<callout>`、`<details>`、`<columns>`、
  `<column>`、`<table>`、`<tr>`）的内容。读取输入时，服务器也接受 2 个空格的缩进；返回时，服务器始终使用 tab。
- 写入时，服务器根据 Markdown 重建 block 树，与当前文档比较，只应用**有差异的部分**（按 block 做 diff），因此
  正在打开该页面的人光标位置保持不变。
- Block 最多嵌套 64 层。

## Block

| Block | NFM |
| --- | --- |
| 段落 | 一行文字 |
| 空段落 | `<empty-block/>` |
| 标题 1–6 | `#` … `######` + 空格 |
| 无序列表 | `- mục` |
| 有序列表 | `1. mục`（第一个数字即起始编号） |
| 待办事项 | `- [ ] chưa xong`、`- [x] xong` |
| 引用 | `> chữ`（连续多行 `>` 构成一个引用） |
| 代码块 | 使用 ```` ``` ```` 或 `~~~` 围栏并注明语言：```` ```rust ```` |
| Mermaid 图 | 语言为 `mermaid` 的代码块 |
| 公式（block） | 单独一行 `$$`，LaTeX，再以 `$$` 结束；或单行 `$$x^2$$` |
| 分隔线 | `---` |
| 图片 | `![chú thích](url)`（也接受 `<image src="url">chú thích</image>`） |
| Callout | `<callout icon="💡" color="yellow">` … `</callout>` |
| Toggle | `<details>` + `<summary>tiêu đề</summary>` + 内容 … `</details>`；标题样式的 toggle：`<details level="2">` |
| 分栏 | `<columns>` 内含若干 `<column>` … `</column>` |
| 表格 | `<table header-row="true" header-column="false">` 内含 `<tr>`，`<tr>` 内含 `<td>`；也接受 GFM 表格 `\| a \| b \|` |
| 子页面 | `<page url="https://toantranmei.com/notes/<id>">Tiêu đề</page>` |
| 内嵌数据库 | `<database url="…/notes/<id>" inline="true">Tiêu đề</database>` |
| 文件 | `<file src="url">tên.zip</file>` |
| 视频、音频、PDF | `<video src="…">tên</video>`、`<audio src="…">tên</audio>`、`<pdf src="…">tên</pdf>` |
| 目录 | `<table_of_contents/>` |
| 尚不支持的 block（sketch、embed、synced block…） | `<unknown url="…?block=<id>" alt="sketch"/>` |
| 页面被截断的剩余部分 | `<truncated url="…?block=<id>" blocks="n"/>`（最后一行） |

### 完整示例

```markdown
# Kế hoạch quý 4
Mục tiêu chính là phát hành **ttm 0.2**.
<empty-block/>
- Notee
	- Đề xuất sửa
	- Truy vấn SQL
- [x] Viết tài liệu
- [ ] Phát hành
> Ghi chú: hạn chót là ngày 15.
<callout icon="⚠️" color="red">
	Không phát hành vào thứ Sáu.
</callout>
<details>
	<summary>Chi tiết rủi ro</summary>
	Thiếu người review.
</details>
<columns>
	<column>
		Bên trái
	</column>
	<column>
		Bên phải
	</column>
</columns>
<table header-row="true" header-column="false">
	<tr>
		<td>Việc</td>
		<td>Người</td>
	</tr>
	<tr>
		<td>Changelog</td>
		<td><mention-user url="user://0190f3a1-…">Bình</mention-user></td>
	</tr>
</table>
<page url="https://toantranmei.com/notes/0190f3b5-…">Biên bản họp</page>
<database url="https://toantranmei.com/notes/0190f3c0-…" inline="true">Việc nhóm</database>
$$
E = mc^2
$$
<table_of_contents/>
```

## 行内格式

| 格式 | NFM |
| --- | --- |
| 粗体、斜体、删除线 | `**đậm**`、`*nghiêng*`、`~~gạch~~` |
| 代码 | `` `code` `` |
| 链接 | `[chữ](https://…)` |
| 下划线 | `<span underline="true">chữ</span>` |
| 文字颜色 | `<span color="red">chữ</span>` |
| 背景色 | `<span color="red_bg">chữ</span>`（也接受后缀 `_background`） |
| 公式 | `$x^2$` |
| Block 内换行 | `<br>` |
| 提及用户 | `<mention-user url="user://<id>">Tên</mention-user>` |
| 提及页面 | `<mention-page url="https://toantranmei.com/notes/<id>">Tiêu đề</mention-page>` |
| 提及日期 | `<mention-date start="2026-11-04"/>` |

`<span>` 可以组合多个属性：`<span underline="true" color="blue">chữ</span>`。常用颜色：`gray`、`brown`、
`orange`、`yellow`、`green`、`blue`、`purple`、`pink`、`red`；`default` 表示无颜色。

## 转义字符

服务器**返回** Markdown 时：

- 普通文字中的 `\`、`*`、`_`、`[`、`]`、`` ` ``、`<`、`~`、`$`、`&` 字符用 `\` 转义，例如 `5\*3`、
  `a\_b`、`\<b>`。
- 以 block 起始字符（`#`、`-`、`+`、`>`、`|`、`=` 或 `1.`）开头的段落会被转义：`\# không phải tiêu đề`、
  `2026\. là năm`。
- 段落开头 / 结尾的空格或 tab 写作 `&#32;` / `&#9;`。
- 在标签的属性和标签文本中：`&amp;`、`&quot;`、`&lt;`、`&gt;`。

**写入**时，服务器能识别上述形式以及实体 `&quot;`、`&#39;`、`&lt;`、`&gt;`、`&#32;`、`&#9;`、`&amp;`。对于
`update_content`，`old_str` 必须与**服务器返回的文字完全一致**，包括 `\` 和 tab。

## 提及、子页面、文件：只能指向已存在的对象

- `<mention-user>` 需要 `url="user://<id người dùng>"`；标签文本是显示的名字。用 `workspace-get-users` 查找 id。缺少
  `url`：会被忽略，并附带警告。
- `<mention-page>`、`<page>`、`<database>` 需要**已存在的页面或数据库**的 URL（或 id）。写 `<page>` 标签并不会创建
  新页面；创建子页面请使用 `notee-create-pages`（或 `ttm notes create --parent`）。没有有效的 URL：会被忽略，并附带
  警告。
- 页面标题、数据库标题和用户名只在你有查看权限时显示；否则标签以空文本显示。
- `<file>`、`<video>`、`<audio>`、`<pdf>`、`<image>` 需要 `src`。Notee 的文件 URL 形如
  `https://toantranmei.com/api/v1/files/<id>`（下载需要登录）。缺少 `src`：会被忽略。
- 你读取后**原样**写回的标签和 block（子页面、数据库、文件、图片、提及）会按原样恢复，包括 Markdown 无法
  表达的属性。

## `<unknown>` 和 `<truncated>`

`<unknown url="…?block=<id>" alt="<loại>"/>` 代替一个 Markdown 无法表达的 block（sketch、embed、synced
block…）。写入时：

- **保留**该标签：根据 `url` 从当前文档中恢复原 block；
- **删除**该标签：该 block 被删除；
- 标签指向页面中不存在的 block：会被忽略，并附带警告 `<unknown url="…"> is not a block of this page; left out`。

超过 **20,000 个节点**的页面在读取时会被截断：Markdown 在该处停止，最后一行是 `<truncated url="…?block=<id>" blocks="n"/>`，
代替从该 block 到页面末尾的**所有**顶层 block；结果中带有 `truncated: true` 和 `unknown_block_ids`。可用
`block=<id>` 分段读取（`ttm notes get --block`、带 `block` 的 `notee-fetch`）。写回时：

- 将 `<truncated/>` 行保留在内容**末尾**：截断点之后的部分保持不变；
- 删除该行：那部分内容会被删除，因此服务器会拒绝（`409 would_delete_truncated`），除非带有 `allow_deleting_content`；
- `<truncated/>` 行不在末尾或重复出现：会被忽略，并附带警告。

重复使用 `<unknown>`/`<truncated>` 多次复制同一个 block，导致超出页面大小限制时会被拒绝（`413 too_many_blocks`）。

## 写入限制

| 限制 | 值 | 错误 |
| --- | --- | --- |
| 单次写入新增、修改或删除的 block（包括嵌套 block） | 1,000（带 `allow_deleting_content` 时删除的 block 不计入） | `413 too_many_blocks` |
| Markdown 大小 | 2 MB（应用 `update_content` 之后） | `413 markdown_too_large` |
| 去掉 `<page>` 或 `<database>`（删除子页面/子数据库） | 需要 `allow_deleting_content` | `409 would_delete_children` |
| 去掉 `<truncated/>` | 需要 `allow_deleting_content` | `409 would_delete_truncated` |
| Agent 写入次数 | 每个 token 每小时 60 次 | `429 rate_limited` |

拆分大的写入：使用多个小的 `update_content`，或先创建页面再逐步追加内容。

## 往返规则（读取后写回）

1. 读取页面、修改文字、写回：只有你修改的 block 会被改动；历史记录显示“<bạn> (via agent <tên>)”，服务器会在写入前
   保存一个版本，以便撤销。
2. 优先使用 `update_content`（`old_str` → `new_str`，每个 `old_str` 必须恰好匹配一处，区分大小写），而不是替换整个
   页面：改动更小，丢失 block 的风险更低。不匹配的替换对会被跳过，并在 `warnings` 中返回。
3. 替换整个页面（`replace_content`）时，保留已读取到的所有 `<unknown/>`、`<truncated/>`、`<page>`、`<database>` 标签。
4. Markdown 无法表达的格式（例如某个 block 的专有属性）在该 block 未修改时会被保留。
5. 数据库行：正文是该行的页面内容；修改列值请使用 `properties`（`notee-update-page`）或
   `notee-upsert-rows`，而不是通过 Markdown。

## 评论与修改建议

评论和修改建议锚定在页面的**文本片段**上（`{exact, prefix?, suffix?}`，即 W3C
`TextQuoteSelector` 风格），无需知道 block id。`exact` 长度为 1–1,000 个字符，且必须恰好匹配一处：找不到则返回
`anchor_not_found`，匹配多处则返回 `ambiguous_anchor`（在 `prefix` 或 `suffix` 中多加几个字）。见 [Notee](notee.md)。
