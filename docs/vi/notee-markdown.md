---
title: Notee-flavored Markdown
description: Đặc tả đầy đủ Markdown kiểu Notion mà ttm, ttm-mcp và API dùng để đọc và ghi page Notee.
order: 12
section: reference
---

Page Notee được đọc (`GET /pages/{id}/markdown`, `ttm notes get`, `notee-fetch`) và ghi (`PATCH`, `ttm notes edit`,
`notee-update-page`, `notee-create-pages`) dưới dạng **Notee-flavored Markdown** (NFM): Markdown thông thường cộng một số
thẻ kiểu XML cho block Markdown không diễn đạt được. Tên thẻ theo Notion-flavored Markdown, nên agent biết Notion sẽ quen
Notee.

## Nguyên tắc chung

- **Một dòng là một block.** Hai dòng liền nhau là hai đoạn văn riêng (không gộp như CommonMark). Đoạn văn rỗng viết
  `<empty-block/>`.
- **Con thụt vào một tab** (`\t`): mục con của danh sách và nội dung của thẻ chứa (`<callout>`, `<details>`, `<columns>`,
  `<column>`, `<table>`, `<tr>`). Khi đọc, server cũng nhận thụt lề bằng 2 dấu cách; khi trả về, server luôn dùng tab.
- Khi ghi, server dựng lại cây block từ Markdown, so với tài liệu hiện tại và chỉ áp **phần khác** (diff theo block), nên
  người đang mở page giữ nguyên con trỏ.
- Lồng block tối đa 64 cấp.

## Block

| Block | NFM |
| --- | --- |
| Đoạn văn | Một dòng chữ |
| Đoạn rỗng | `<empty-block/>` |
| Tiêu đề 1–6 | `#` … `######` + dấu cách |
| Danh sách chấm | `- mục` |
| Danh sách số | `1. mục` (số đầu tiên là số bắt đầu) |
| Việc cần làm | `- [ ] chưa xong`, `- [x] xong` |
| Trích dẫn | `> chữ` (nhiều dòng `>` liền nhau là một trích dẫn) |
| Khối code | Rào ```` ``` ```` hoặc `~~~` kèm ngôn ngữ: ```` ```rust ```` |
| Sơ đồ Mermaid | Khối code ngôn ngữ `mermaid` |
| Công thức (block) | `$$` trên một dòng, LaTeX, `$$` đóng; hoặc `$$x^2$$` trên một dòng |
| Đường kẻ | `---` |
| Ảnh | `![chú thích](url)` (cũng nhận `<image src="url">chú thích</image>`) |
| Callout | `<callout icon="💡" color="yellow">` … `</callout>` |
| Toggle | `<details>` + `<summary>tiêu đề</summary>` + nội dung … `</details>`; toggle dạng tiêu đề: `<details level="2">` |
| Cột | `<columns>` chứa các `<column>` … `</column>` |
| Bảng | `<table header-row="true" header-column="false">` chứa `<tr>` chứa `<td>`; cũng nhận bảng GFM `\| a \| b \|` |
| Page con | `<page url="https://toantranmei.com/notes/<id>">Tiêu đề</page>` |
| Database nhúng | `<database url="…/notes/<id>" inline="true">Tiêu đề</database>` |
| File | `<file src="url">tên.zip</file>` |
| Video, âm thanh, PDF | `<video src="…">tên</video>`, `<audio src="…">tên</audio>`, `<pdf src="…">tên</pdf>` |
| Mục lục | `<table_of_contents/>` |
| Block chưa hỗ trợ (sketch, embed, synced block…) | `<unknown url="…?block=<id>" alt="sketch"/>` |
| Phần còn lại của page bị cắt | `<truncated url="…?block=<id>" blocks="n"/>` (dòng cuối) |

### Ví dụ đầy đủ

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

## Định dạng trong dòng

| Định dạng | NFM |
| --- | --- |
| Đậm, nghiêng, gạch | `**đậm**`, `*nghiêng*`, `~~gạch~~` |
| Code | `` `code` `` |
| Link | `[chữ](https://…)` |
| Gạch chân | `<span underline="true">chữ</span>` |
| Màu chữ | `<span color="red">chữ</span>` |
| Màu nền | `<span color="red_bg">chữ</span>` (cũng nhận hậu tố `_background`) |
| Công thức | `$x^2$` |
| Xuống dòng trong block | `<br>` |
| Nhắc người | `<mention-user url="user://<id>">Tên</mention-user>` |
| Nhắc page | `<mention-page url="https://toantranmei.com/notes/<id>">Tiêu đề</mention-page>` |
| Nhắc ngày | `<mention-date start="2026-11-04"/>` |

`<span>` gộp được nhiều thuộc tính: `<span underline="true" color="blue">chữ</span>`. Màu thường dùng: `gray`, `brown`,
`orange`, `yellow`, `green`, `blue`, `purple`, `pink`, `red`; `default` là không màu.

## Ký tự thoát

Khi server **trả** Markdown:

- Các ký tự `\`, `*`, `_`, `[`, `]`, `` ` ``, `<`, `~`, `$`, `&` trong chữ thường được thoát bằng `\`, ví dụ `5\*3`,
  `a\_b`, `\<b>`.
- Đoạn văn bắt đầu bằng ký tự mở block (`#`, `-`, `+`, `>`, `|`, `=`, hay `1.`) được thoát: `\# không phải tiêu đề`,
  `2026\. là năm`.
- Dấu cách hoặc tab ở đầu / cuối đoạn văn được viết là `&#32;` / `&#9;`.
- Trong thuộc tính và nhãn của thẻ: `&amp;`, `&quot;`, `&lt;`, `&gt;`.

Khi **ghi**, server hiểu các dạng đó và các thực thể `&quot;`, `&#39;`, `&lt;`, `&gt;`, `&#32;`, `&#9;`, `&amp;`. Với
`update_content`, `old_str` phải khớp **đúng chữ server trả về**, kể cả dấu `\` và tab.

## Nhắc tên, page con, file: chỉ trỏ tới thứ đã có

- `<mention-user>` cần `url="user://<id người dùng>"`; nhãn là tên hiện ra. Tìm id bằng `workspace-get-users`. Thiếu
  `url`: bị bỏ, kèm cảnh báo.
- `<mention-page>`, `<page>`, `<database>` cần URL (hoặc id) của **page hay database có sẵn**. Viết thẻ `<page>` không tạo
  page mới; tạo page con bằng `notee-create-pages` (hoặc `ttm notes create --parent`). Không có URL hợp lệ: bị bỏ, kèm
  cảnh báo.
- Tiêu đề page, database và tên người chỉ hiện khi bạn có quyền xem; nếu không, thẻ hiện với nhãn rỗng.
- `<file>`, `<video>`, `<audio>`, `<pdf>`, `<image>` cần `src`. File của Notee có URL dạng
  `https://toantranmei.com/api/v1/files/<id>` (cần đăng nhập để tải). Thiếu `src`: bị bỏ.
- Thẻ và block bạn đọc rồi ghi lại **nguyên văn** (page con, database, file, ảnh, nhắc tên) được khôi phục đúng như bản
  gốc, kể cả thuộc tính Markdown không thể hiện.

## `<unknown>` và `<truncated>`

`<unknown url="…?block=<id>" alt="<loại>"/>` đứng thay một block Markdown không diễn đạt được (sketch, embed, synced
block…). Khi ghi:

- **giữ nguyên** thẻ: block gốc được khôi phục từ tài liệu hiện tại theo `url`;
- **xoá** thẻ: block đó bị xoá;
- thẻ trỏ tới block không có trong page: bị bỏ, kèm cảnh báo `<unknown url="…"> is not a block of this page; left out`.

Page hơn **20.000 node** bị cắt khi đọc: Markdown dừng ở đó và dòng cuối là `<truncated url="…?block=<id>" blocks="n"/>`,
đứng thay **mọi** block cấp cao nhất từ block đó tới hết page; kết quả có `truncated: true` và `unknown_block_ids`. Đọc
từng phần bằng `block=<id>` (`ttm notes get --block`, `notee-fetch` với `block`). Khi ghi lại:

- giữ dòng `<truncated/>` ở **cuối** nội dung: phần sau chỗ cắt giữ nguyên;
- xoá nó: phần đó bị xoá, nên server từ chối (`409 would_delete_truncated`) trừ khi có `allow_deleting_content`;
- dòng `<truncated/>` không ở cuối hoặc lặp lại: bị bỏ, kèm cảnh báo.

Lặp `<unknown>`/`<truncated>` để chép cùng block nhiều lần đến mức vượt kích thước page bị từ chối (`413 too_many_blocks`).

## Giới hạn khi ghi

| Giới hạn | Giá trị | Lỗi |
| --- | --- | --- |
| Block thêm, đổi hoặc xoá trong một lần ghi (tính cả block lồng) | 1.000 (block xoá không tính khi có `allow_deleting_content`) | `413 too_many_blocks` |
| Kích thước Markdown | 2 MB (sau khi áp `update_content`) | `413 markdown_too_large` |
| Bỏ `<page>` hay `<database>` (xoá page/database con) | cần `allow_deleting_content` | `409 would_delete_children` |
| Bỏ `<truncated/>` | cần `allow_deleting_content` | `409 would_delete_truncated` |
| Số lần ghi của agent | 60 mỗi giờ mỗi token | `429 rate_limited` |

Chia lần ghi lớn: dùng nhiều `update_content` nhỏ, hoặc tạo page rồi thêm nội dung dần.

## Quy tắc khứ hồi (đọc rồi ghi lại)

1. Đọc page, sửa chữ, ghi lại: chỉ block bạn đổi bị đụng; lịch sử ghi “<bạn> (via agent <tên>)”, và server chụp một phiên
   bản trước khi ghi để hoàn tác được.
2. Ưu tiên `update_content` (`old_str` → `new_str`, mỗi `old_str` khớp đúng một chỗ, phân biệt hoa thường) hơn thay cả
   page: thay đổi nhỏ hơn, ít rủi ro làm mất block hơn. Cặp không khớp bị bỏ qua và trả về trong `warnings`.
3. Khi thay cả page (`replace_content`), giữ mọi thẻ `<unknown/>`, `<truncated/>`, `<page>`, `<database>` đã đọc.
4. Định dạng Markdown không diễn đạt được (ví dụ thuộc tính riêng của một block) được giữ khi block đó không đổi.
5. Dòng database: thân là nội dung page của dòng; đổi giá trị cột bằng `properties` (`notee-update-page`) hoặc
   `notee-upsert-rows`, không qua Markdown.

## Comment và đề xuất

Comment và đề xuất sửa neo vào **đoạn trích văn bản** của page (`{exact, prefix?, suffix?}`, kiểu W3C
`TextQuoteSelector`), không cần biết id block. `exact` dài 1–1.000 ký tự và phải khớp đúng một chỗ: không thấy thì
`anchor_not_found`, nhiều chỗ thì `ambiguous_anchor` (thêm vài chữ `prefix` hoặc `suffix`). Xem [Notee](notee.md).
