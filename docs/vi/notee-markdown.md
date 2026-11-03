---
title: Notee-flavored Markdown
description: Đặc tả Markdown kiểu Notion mà ttm và MCP dùng để đọc, ghi page Notee.
order: 5
---

# Notee-flavored Markdown

Page Notee được đọc và ghi bằng Markdown chuẩn (CommonMark + bảng và task list kiểu GFM) cộng một số thẻ dạng XML cho các
block Markdown không diễn tả được. Tên thẻ theo Markdown của Notion, nên agent biết Notion là biết Notee.

## Markdown chuẩn

| Block | Markdown |
| --- | --- |
| Tiêu đề 1–3 | `#`, `##`, `###` |
| Đoạn văn, **đậm**, *nghiêng*, ~~gạch~~, `code`, [liên kết](url) | như thường lệ |
| Danh sách chấm / số | `- mục`, `1. mục` (thụt 2 dấu cách để lồng) |
| Việc cần làm | `- [ ] chưa xong`, `- [x] xong` |
| Trích dẫn | `> văn bản` |
| Khối code | rào ba dấu kèm ngôn ngữ: ```` ```rust ```` |
| Đường kẻ | `---` |
| Ảnh | `![chú thích](url)` |
| Công thức | `$$ … $$` trên dòng riêng |
| Sơ đồ mermaid | ```` ```mermaid ```` |

## Thẻ

| Block | Thẻ |
| --- | --- |
| Callout | `<callout icon="💡" color="yellow">văn bản</callout>` |
| Toggle | `<details><summary>Tiêu đề</summary>nội dung con</details>` |
| Cột | `<columns><column>…</column><column>…</column></columns>` |
| Bảng (ô định dạng) | `<table><tr><td>…</td></tr></table>` (bảng đơn giản dùng được dấu `|` kiểu GFM) |
| Page con | `<page url="…">Tiêu đề</page>` |
| Database nhúng | `<database url="…">Tiêu đề</database>` |
| Nhắc người | `<mention-user url="…">Tên</mention-user>` |
| Nhắc page | `<mention-page url="…">Tiêu đề</mention-page>` |
| Nhắc ngày | `<mention-date start="2026-11-04" end="2026-11-05"/>` |
| File | `<file src="…">ten.pdf</file>` (URL ký sẵn, sống ngắn) |
| Block chưa hỗ trợ (sketch, …) | `<unknown url="…" alt="sketch"/>` |

## Quy tắc khi ghi

- Giữ nguyên mọi thẻ `<unknown …/>` đã nhận: nó đại diện cho block Markdown không mang được và được khôi phục từ bản gốc
  khi ghi lại. Xoá thẻ là xoá block đó.
- Xoá thẻ `<page>` hoặc `<database>` là xoá page/database con; máy chủ từ chối nếu thiếu `allow_deleting_content: true`.
- Mỗi lần ghi thêm hoặc đổi tối đa **1.000 block** (tính cả block lồng nhau). Sửa lớn hơn thì chia nhỏ.
- Ưu tiên `update_content` (`old_str` → `new_str`, mỗi `old_str` khớp đúng một chỗ, phân biệt hoa thường) thay vì thay cả
  page: con trỏ của người đang soạn giữ nguyên và lịch sử gọn hơn.

## Page lớn

Page dài hơn giới hạn đọc trả về `truncated: true` và `unknown_block_ids`. Đọc từng phần thiếu bằng `notee-fetch` (hoặc
`ttm notes get --block <id>`).

## Comment

Comment neo vào một đoạn trích văn bản của page (`{exact, prefix?, suffix?}`). Đoạn trích xuất hiện nhiều lần thì máy chủ
trả `ambiguous_anchor`; thêm vài chữ `prefix` hoặc `suffix`.
