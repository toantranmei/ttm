---
title: Tool MCP
description: Mọi tool của ttm-mcp và MCP từ xa - mục đích, tham số, ví dụ gọi và kết quả, scope cần, lỗi.
order: 11
section: reference
---

Trang này áp dụng cho cả `ttm-mcp` (stdio) và MCP từ xa `https://toantranmei.com/api/v1/mcp`: cùng tên tool, cùng tham số,
cùng kết quả. Phiên bản **0.1.5**: 26 tool trong 3 nhóm và 1 resource.

## Quy ước

- Tham số là đối tượng JSON; **tham số lạ bị từ chối**. Tham số `null` hay bỏ trống dùng giá trị mặc định.
- Mọi tham số page/database nhận id hoặc URL bất kỳ của nó.
- Mỗi tool trả **nội dung văn bản** (Markdown cho model, hoặc JSON) và **`structuredContent`** (JSON).
- Chú thích (annotation): tool đọc có `readOnlyHint: true`, `idempotentHint: true`; tool ghi có `readOnlyHint: false`;
  tool có thể xoá hoặc ghi đè dữ liệu có `destructiveHint: true`. Không tool nào chạm ra ngoài toantranmei.com
  (`openWorldHint: false`).

### Lỗi

| Dạng | Khi nào | Model thấy |
| --- | --- | --- |
| Lỗi giao thức `invalid_params` (-32602) | Tên tool lạ; tham số sai kiểu, thiếu, hoặc lạ; kiểm tra cục bộ thất bại (ví dụ `give 1 to 20 pages`) | Thông báo lỗi JSON-RPC |
| Kết quả có `isError: true` | Server từ chối hoặc lỗi | Văn bản `<thông báo> (<code>)`, và `structuredContent: {"error": {"code", "message", "status", "rule", "field", "params"}}` |

`code` là mã của server (`unauthorized`, `invalid_token`, `insufficient_scope`, `not_found`, `too_many_blocks`…, xem
[Mã lỗi](ma-loi.md)) hoặc mã của chính tool:

| Mã | Nghĩa |
| --- | --- |
| `no_run` | Tool cần một lần chạy agent (`TTM_RUN_ID` không đặt) |
| `no_task` | Không truyền `task` và không có task hiện tại (`TTM_TASK`) |
| `no_project` | Không truyền `project` và không có `TTM_PROJECT` |
| `unsupported` | Server này không hỗ trợ tool đó |
| `network`, `decode` | Không gọi được API / phản hồi lạ |

Lỗi của server giữ `status`, `rule` (quy tắc bị vi phạm), `field` (trường đầu vào) và `params` (giá trị liên quan) khi
server gửi; văn bản cũng ghi `rule` và `field`. Agent rẽ nhánh theo `rule` thay vì đọc câu chữ: `split-limit` (chia ít hơn
hoặc nông hơn, `params` có giới hạn), `rev-conflict` (đọc lại task rồi thử lại), `approval-required` (xin duyệt bằng
`factory-request-approval`); `insufficient_scope` có `params.required` là scope còn thiếu.

Lỗi 401 kèm gợi ý ``Sign in again with `ttm login`.``

## Resource

| URI | Tên | Kiểu | Nội dung |
| --- | --- | --- | --- |
| `notee://spec/markdown` | `notee-markdown` | `text/markdown` | Đặc tả Notee-flavored Markdown (có khi bật nhóm `notee`) |

---

## Nhóm workspace

Luôn bật. Scope: `workspace:read`.

### workspace-get-self

Người dùng đang đăng nhập (id, tên, email, gói) và workspace hiện tại. Nên gọi đầu tiên.

Tham số: không có.

```json
{
  "user": { "id": "0190f3a1-…", "email": "an@example.com", "name": "An Nguyễn", "avatar_url": null, "locale": "vi", "plan": "free" },
  "workspace": { "id": "0190f3a2-…", "name": "Team Aurora", "kind": "team", "role": "member", "slug": "aurora" }
}
```

Workspace hiện tại = workspace của profile (`ttm workspace switch`), hoặc workspace mà token bị giới hạn vào (MCP từ xa),
nếu không thì workspace cá nhân.

### workspace-list

Các workspace của người dùng, với loại (`personal`/`team`) và vai trò (`owner`, `admin`, `member`, `guest`).

Tham số: không có. Kết quả: `{"workspaces": [{id, name, kind, role, slug?}]}`.

### workspace-get-users

Thành viên (người) của một workspace và vai trò, để tìm người cần nhắc tên hay giao việc.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `workspace_id` | string | | Mặc định workspace hiện tại |
| `query` | string | | Lọc theo tên hoặc email, không phân biệt hoa thường |

```json
{ "query": "bình" }
```

```json
{ "users": [{ "user": { "id": "0190…", "name": "Bình", "email": "binh@example.com", "avatar_url": null }, "role": "member" }] }
```

Lỗi: `forbidden` (khách không xem được danh sách thành viên).

---

## Nhóm notee

### notee-search

Tìm page bạn xem được theo tiêu đề và nội dung (không phân biệt dấu). Scope `notee:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `query` | string | có | Các từ cần tìm, mọi từ phải có mặt; ≤ 8 từ, ≤ 200 ký tự |
| `limit` | integer | | 1–20, mặc định 10 |
| `workspace_id` | string | | Mặc định workspace hiện tại |

```json
{ "query": "ke hoach quy 4", "limit": 5 }
```

Văn bản: `- Kế hoạch quý 4 (id: 0190f3b0-…): …đoạn trích…`. Có cấu trúc:
`{"results": [{id, workspace_id, title, icon, snippet}]}` (`snippet` rỗng khi chỉ tiêu đề khớp).

Lỗi: `invalid_params` khi `query` rỗng; `bad_request` khi câu tìm quá dài.

### notee-fetch

Đọc một page dạng [Notee-flavored Markdown](notee-markdown.md), hoặc property và view của một database. Scope `notee:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `id` | string | có | Id hoặc URL của page, dòng database hoặc database |
| `block` | string | | Một id trong `unknown_block_ids` của page bị cắt, để đọc phần đó |

Văn bản cho page:

```text
<page id="0190f3b0-…" title="Kế hoạch quý 4" url="https://toantranmei.com/notes/0190f3b0-…">
# Mục tiêu
- Phát hành ttm 0.2
</page>
```

Page bị cắt thêm dòng ``The page was truncated. Read the missing parts with notee-fetch and `block` set to one of: …``. Dòng
database có thêm `<properties>{…}</properties>`. Có cấu trúc: `{"kind": "page", id, title, icon, url, parent,
properties?, markdown, truncated, unknown_block_ids}` hoặc `{"kind": "database", id, title, url, properties[], views[]}`.

Văn bản cho database là bảng `| Property | Type |`.

Lỗi: `not_found` (không có hoặc không xem được).

### notee-query-database

Truy vấn dòng của database bằng SQL chỉ đọc hoặc bộ lọc JSON. Scope `notee:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `database` | string | có | Id hoặc URL |
| `sql` | string | một trong hai | `SELECT … FROM rows …`, cột là tên property |
| `filter` | object | một trong hai | `{"and": [{"property", "op", "value"}]}` |
| `sorts` | array | | Đi với `filter`: `[{"property": "Due", "direction": "asc"}]` |
| `page_size` | integer | | 1–100 (mặc định 100) |
| `cursor` | string | | `next_cursor` của trang trước |

```json
{ "database": "0190f3c0-…", "sql": "SELECT \"Name\", \"Status\" FROM rows WHERE \"Status\" = 'Done' ORDER BY \"Due\" LIMIT 20" }
```

```json
{ "columns": ["Name", "Status"], "rows": [{ "id": "", "values": { "Name": "Viết tài liệu", "Status": "Done" } }], "next_cursor": null }
```

Toán tử của `filter`: `eq`, `neq`, `contains`, `not_contains`, `starts_with`, `ends_with`, `gt`, `gte`, `lt`, `lte`,
`in`, `not_in`, `contains_all`, `is_empty`, `is_not_empty`, `between`, `is_relative`. Quy tắc SQL và bộ lọc: [Notee](notee.md).

Lỗi: `invalid_params` khi thiếu cả hai hoặc có cả hai; `sql_syntax`, `sql_not_select`, `sql_table`, `sql_function`,
`sql_forbidden`, `sql_error`, `bad_request` (`unknown_prop`, `filter_shape`, `invalid_cursor`), `query_timeout`,
`result_too_large`, `query_busy`, `sql_sandbox_unavailable`.

### notee-get-comments

Thread comment của một page, kèm đoạn trích mỗi thread neo vào. Scope `notee:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `page` | string | có | Id hoặc URL |
| `include_resolved` | boolean | | Gồm thread đã đóng (mặc định `false`) |

```json
{ "threads": [{ "id": "0190f3e0-…", "anchor": { "kind": "text", "block_id": "b7", "quote": "ra mắt tháng 12" },
  "quote": { "exact": "ra mắt tháng 12", "prefix": null, "suffix": null }, "resolved": false,
  "comments": [{ "id": "…", "author": "An Nguyễn", "text": "Chốt ngày cụ thể chưa?", "created_at": "2026-11-04T05:00:00Z" }] }] }
```

### notee-create-pages

Tạo 1–20 page dưới một page, thành dòng của database, hoặc ở cấp cao nhất của workspace. Scope `notee:write`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `parent` | object | có | Đúng **một** trong `page_id`, `database_id`, `workspace_id` |
| `pages` | array | có | 1–20 page, tổng ≤ 1.000 block |
| `pages[].title` | string | có | Tiêu đề |
| `pages[].icon` | string | | Emoji |
| `pages[].content` | string | | Nội dung Markdown |
| `pages[].properties` | object | | Giá trị cột theo tên property, khi cha là database |

```json
{ "parent": { "page_id": "0190f3b0-…" }, "pages": [{ "title": "Biên bản 04/11", "icon": "📝", "content": "## Quyết định\n- Phát hành thứ Sáu" }] }
```

```json
{ "pages": [{ "id": "0190f3c4-…", "title": "Biên bản 04/11", "url": "https://toantranmei.com/notes/0190f3c4-…" }] }
```

Lỗi: `invalid_params` (`give 1 to 20 pages`, `parent needs exactly one of page_id, database_id, workspace_id`),
`too_many_blocks`, `markdown_too_large`, `rate_limited`, `forbidden`.

### notee-update-page

Sửa page: tìm-thay (`update_content`) hoặc thay cả nội dung (`replace_content`); cũng đổi tiêu đề và giá trị cột của dòng
database. `mode: "suggest"` tạo đề xuất sửa thay vì ghi. Scope `notee:comment` (đề xuất) hoặc `notee:write` (ghi). Đọc page
bằng `notee-fetch` trước.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `page` | string | có | Id hoặc URL |
| `update_content` | array | | `[{old_str, new_str, replace_all?}]`; `old_str` khớp chính xác, phân biệt hoa thường, đúng một chỗ trừ khi `replace_all` |
| `replace_content` | string | | Toàn bộ nội dung mới; giữ mọi thẻ `<unknown/>` đã đọc |
| `allow_deleting_content` | boolean | | Cho phép bỏ page con, database con, phần `<truncated/>` (mặc định `false`) |
| `title` | string | | Tiêu đề mới |
| `properties` | object | | Giá trị cột cần đổi (dòng database) |
| `mode` | `"apply"` \| `"suggest"` | | Mặc định theo cài đặt workspace (`agent_write_mode`); luôn `suggest` nếu token thiếu `notee:write` |

Cần ít nhất một thay đổi; không dùng `update_content` cùng `replace_content`. Đề xuất chỉ đổi nội dung.

```json
{ "page": "0190f3b0-…", "update_content": [{ "old_str": "ra mắt tháng 11", "new_str": "ra mắt tháng 12" }], "mode": "suggest" }
```

Văn bản: `Suggestion <id> created; people with edit access can accept or reject it.`, `Page updated.` hoặc
`No change applied.`, kèm `Warning: update_content[0]: old_str not found; skipped`. Có cấu trúc:
`{applied, warnings, seq, snapshot_seq, suggestion_id?}`.

Lỗi: `invalid_params` (`nothing to change`, dùng cả hai cách), `would_delete_children`, `would_delete_truncated`,
`too_many_blocks`, `markdown_too_large`, `insufficient_scope` (`mode: apply` khi thiếu `notee:write`),
`bad_request` (`suggest_content_only`, `one_of`), `rate_limited`, `forbidden`.

### notee-move-pages

Chuyển page tới dưới page khác, vào database hoặc lên cấp cao nhất của workspace. Scope `notee:write`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `pages` | array of string | có | Id hoặc URL các page |
| `parent` | object | có | Đúng một trong `page_id`, `database_id`, `workspace_id` |

Kết quả: `{"pages": [{id, title, url}]}`. Page được chuyển lần lượt; lỗi ở page nào thì dừng ở đó.

### notee-duplicate-page

Nhân bản một page cùng nội dung. Scope `notee:write`.

| Tham số | Kiểu | Bắt buộc |
| --- | --- | --- |
| `page` | string | có |

Kết quả: `{id, title, url}` của bản sao.

### notee-create-database

Tạo database. Scope `notee:write`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `parent` | object | có | Đúng một trong `page_id`, `database_id`, `workspace_id` |
| `title` | string | có | |
| `properties` | array | | `[{"name": "Status", "type": "select", "options": [{"name": "Todo"}, {"name": "Done"}]}]`; kiểu: `title`, `text`, `number`, `select`, `multi_select`, `status`, `date`, `person`, `checkbox`, `url`, `email`, `phone`, `relation` |

Kết quả: `{id, title, url}`.

### notee-update-database

Thêm, sửa, xoá property và view, đổi tên database. Scope `notee:write`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `database` | string | có | Id hoặc URL |
| `changes` | object | có | `{"add_properties": [...], "update_properties": [...], "remove_properties": ["Name"], "views": [...], "title": "…"}` |

Kết quả: phản hồi của server (JSON).

### notee-upsert-rows

Tạo hoặc cập nhật 1–100 dòng. Scope `notee:write`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `database` | string | có | Id hoặc URL |
| `rows` | array | có | 1–100 phần tử `{id?, values}` |
| `rows[].id` | string | | Có: cập nhật dòng; không: tạo dòng |
| `rows[].values` | object | có | Giá trị theo tên property; lựa chọn theo tên |

```json
{ "database": "0190f3c0-…", "rows": [{ "values": { "Name": "Viết changelog", "Status": "Todo" } }] }
```

Kết quả: `{"ids": ["0190f3c7-…"]}`. Lỗi: `invalid_params` (`give 1 to 100 rows`), `row_deleted`, `bad_request`
(`row_elsewhere`).

### notee-delete-rows

Xoá dòng (chuyển vào thùng rác). Scope `notee:write`. `destructiveHint: true`.

| Tham số | Kiểu | Bắt buộc |
| --- | --- | --- |
| `database` | string | có |
| `ids` | array of string | có (không rỗng) |

### notee-create-comment

Comment neo vào một đoạn trích của page, comment cho cả page, hoặc trả lời một thread. Scope `notee:comment`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `page` | string | có | Id hoặc URL |
| `text` | string | có | Nội dung (không rỗng) |
| `quote` | object | | `{exact, prefix?, suffix?}`; bỏ trống = comment cho cả page |
| `quote.exact` | string | có (trong `quote`) | Đoạn chữ đúng như trong page, 1–1.000 ký tự |
| `quote.prefix`, `quote.suffix` | string | | Vài chữ ngay trước / sau, khi `exact` xuất hiện nhiều lần |
| `thread_id` | string | | Trả lời thread này thay vì mở thread mới |

```json
{ "page": "0190f3b0-…", "text": "Nguồn số liệu?", "quote": { "exact": "tăng 30%", "suffix": " so với quý trước" } }
```

Lỗi: `anchor_not_found`, `ambiguous_anchor` (thêm `prefix`/`suffix`), `bad_request` (`quote_length`).

### notee-get-suggestions

Đề xuất sửa trên một page: tác giả, agent, từng thay đổi với đoạn neo và Markdown trước/sau. Scope `notee:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `page` | string | có | Id hoặc URL |
| `include_resolved` | boolean | | Gồm đề xuất đã chấp nhận, từ chối, lỗi thời (mặc định chỉ `pending`) |

Mỗi đề xuất: `id`, `status` (`pending`, `accepted`, `rejected`, `stale`), `author`, `via_agent`, `created_at`, `changes[]`
(`id`, `kind` = `insert`/`delete`/`replace`, `anchor`, `before_nfm`, `after_nfm`).

### notee-resolve-suggestion

Chấp nhận (tất cả hoặc một số thay đổi) hoặc từ chối một đề xuất. Cần quyền sửa page. Scope `notee:write`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `suggestion` | string | có | Id đề xuất |
| `decision` | `"accept"` \| `"reject"` | có | |
| `change_ids` | array of string | | Chỉ chấp nhận các thay đổi này (bỏ qua khi từ chối) |

Kết quả: `{id, status, stale_change_ids}`; thay đổi có đoạn neo đã mất là `stale` và không được áp.

---

## Nhóm factory

Dành cho agent **trong một lần chạy** của runner (biến `TTM_RUN_ID`, `TTM_TASK`, `TTM_PROJECT`, xem
[Kết nối MCP](mcp.md)). Token của job có `factory:run` (bao gồm `factory:read`).

### factory-get-task

Task đang làm (hoặc task khác theo key): tiêu đề, trạng thái, vai trò, người làm, task cha. Scope `factory:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `task` | string | | Key (`AUR-12`) hoặc id; mặc định task của run |

Kết quả: đối tượng task (camelCase): `id` (key), `key`, `uuid`, `title`, `state`, `category`, `role`, `assignee`,
`assigneeMember{kind, id, name}`, `parentId`, `estimate`, `description`, `rev`… Lỗi: `no_task`, `not_found`.

### factory-get-context

Context server đã ghép cho task: mô tả, tiêu chí chấp nhận, page Notee liên quan, task anh em, file gợi ý. Nên đọc trước
khi bắt đầu. Scope `factory:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `task` | string | | Mặc định task của run |

Không truyền `task` trong một run: context của run; ngược lại: context của task. Khi server trả `markdown`, model nhận
Markdown đó.

### factory-report-progress

Báo tiến độ lên Execution Map (hiện trực tiếp cho cả nhóm). Scope `factory:run`. Cần run.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `message` | string | có | Một hai câu: đã làm / đang làm gì |
| `percent` | integer | | 0–100 |

Kết quả: `{"ok": true}`, văn bản `Progress reported.` Lỗi: `invalid_params` (`percent must be 0-100`), `no_run`,
`conflict` (`run-finished`).

### factory-request-approval

Xin một người (hoặc vai trò mà workflow quy định) duyệt trước khi làm tiếp. Scope `factory:run`. Cần run.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `reason` | string | có | Cần duyệt gì và vì sao |

Văn bản: `Approval requested. Wait for the decision before doing what needs approval.` Người duyệt nhận thông báo.

### factory-create-subtasks

Chia task thành task con thật. Scope `factory:run` hoặc `factory:write`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `parent` | string | | Key task cha; mặc định task của run |
| `tasks` | array | có | Không rỗng; tối đa 10 mỗi run |
| `tasks[].title` | string | có | |
| `tasks[].role` | string | | `PM`, `BA`, `UI/UX`, `DEV`, `GUARDIAN`, `QC` hoặc vai trò của workspace |
| `tasks[].description` | string | | |
| `tasks[].estimate` | integer | | Điểm hoặc giờ, như project dùng |
| `tasks[].type` | string | | `feature`, `bug`, `chore`… |
| `tasks[].priority` | string | | |
| `tasks[].labels` | array of string | | |
| `tasks[].depends_on` | array of string | | Key task có sẵn, hoặc `#<n>` = task con thứ n (từ 0) của lần gọi này |
| `tasks[].acceptance_criteria` | array of string | | |

```json
{ "tasks": [{ "title": "API callback", "role": "DEV", "estimate": 2 }, { "title": "Test đăng nhập", "role": "QC", "depends_on": ["#0"] }] }
```

Kết quả: `{tasks: [...], warnings: [...]}`. Giới hạn: 10 mỗi run, 30 dưới một task gốc, sâu 2 cấp, vai trò được phép chia.
Vượt: lỗi `conflict` (quy tắc `split-limit`) với thông báo và `params` (`max_depth`, `per_run`, `per_root`, `depth`,
`run_count`, `root_count`) để agent chia lại.

### factory-attach-artifact

Đính kết quả vào run: pull request, kết quả test, ảnh chụp, link, ghi chú. Scope `factory:run`. Cần run.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `kind` | string | có | `pr` (ghi làm PR của run), `test_result`, `ci`, `screenshot`, `link`, `note` |
| `url` | string | | |
| `text` | string | | Nội dung (log test, ghi chú) |
| `title` | string | | |
| `passed` | boolean | | Với `test_result` và `ci` |

Cần ít nhất một trong `url`, `text`, `passed`.

```json
{ "kind": "test_result", "title": "cargo test", "passed": true, "text": "test result: ok. 42 passed" }
```

### factory-get-rules

Luật của project cho agent (nhánh, commit, test trước khi xong). Scope `factory:read`.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `project` | string | | Key hoặc id; mặc định project của run (`TTM_PROJECT`) |

Văn bản: khối Markdown được quản lý (cùng nội dung `ttm rules apply` ghi vào `AGENTS.md`). Có cấu trúc: `rev`,
`managed_markdown`, `hooks`, `checks[{id, description, command, enabled}]`.

### factory-renew-lease

Giữ hoặc gia hạn quyền sửa một vùng code (glob) để agent khác không sửa cùng lúc. Scope `factory:run`. Cần run.

| Tham số | Kiểu | Bắt buộc | Mô tả |
| --- | --- | --- | --- |
| `paths` | array of string | có | Glob, ví dụ `src/auth/**` |

Kết quả: `{granted, expires_at, held_by, paths}`. Vùng đang do run khác giữ: `granted: false` và `held_by` cho biết ai giữ;
làm việc khác hoặc chờ rồi gọi lại.
