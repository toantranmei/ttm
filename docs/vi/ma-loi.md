---
title: Mã lỗi
description: Mã lỗi ổn định mà API, ttm và ttm-mcp trả về, ý nghĩa và cách xử lý.
order: 14
section: reference
---

## Lỗi trông như thế nào

Server trả lỗi dạng `application/problem+json`:

```json
{
  "code": "insufficient_scope",
  "title": "this token lacks the scope notee:write",
  "status": 403,
  "params": { "required": "notee:write" }
}
```

| Trường | Ý nghĩa |
| --- | --- |
| `code` | Mã ổn định để rẽ nhánh (bảng dưới) |
| `title` | Câu tiếng Anh cho người đọc |
| `status` | Mã HTTP |
| `field` | Trường đầu vào bị lỗi (với `bad_request`, `conflict`), ví dụ `anchor.quote.exact` |
| `rule` | Quy tắc cụ thể bị vi phạm (với `bad_request`, `conflict`, `precondition_failed`), ví dụ `split-limit` |
| `params` | Giá trị liên quan, ví dụ `{"required": "notee:write"}`, `{"max_depth": 2, "per_run": 10}` |

`ttm` in `error: <title> (<code>, HTTP <status>) <params>` ra stderr; với `--json` thêm `{"error": {"code", "message"}}`
ra stdout. Tool MCP trả `isError: true`, văn bản `<thông báo> (<code>)` và `structuredContent.error.code`.

> **Lưu ý:** với lỗi `bad_request`, `conflict`, `precondition_failed`, `code` chung chung; nguyên nhân cụ thể nằm ở
> `rule` (cột **Rule** dưới đây) và câu thông báo.

## Xác thực và quyền

| Code | HTTP | Nghĩa | Cách xử lý |
| --- | --- | --- | --- |
| `invalid_token` | 401 | Token sai, hết hạn hoặc đã thu hồi | `ttm login` lại; kiểm `TTM_TOKEN` cũ |
| `unauthorized` | 401 | Không có token | `ttm login` hoặc đặt `TTM_TOKEN` |
| `insufficient_scope` | 403 | Token thiếu scope; `params.required` cho biết scope cần | Đăng nhập với scope đó (`ttm login --scope …`) hoặc tạo PAT mới |
| `token_not_allowed` | 403 | Route này không mở cho token (quản lý token, chia sẻ, thanh toán, xoá page, tạo workspace…) | Làm trên web |
| `forbidden` | 403 | Bạn không có quyền trên tài nguyên này | Xin quyền từ chủ page/workspace |
| `upgrade_required` | 426 | `ttm` cũ hơn `min_cli` của server | `ttm self-update` |

## Đăng nhập (OAuth)

| Code | Nghĩa | Cách xử lý |
| --- | --- | --- |
| `access_denied` | Bạn bấm **Từ chối** | Chạy lại `ttm login` |
| `authorization_pending`, `slow_down` | (device flow) đang chờ bạn cho phép / hỏi quá nhanh | `ttm` tự xử lý, không cần làm gì |
| `expired_token` | Mã thiết bị quá 10 phút | `ttm login --device` lại |
| `invalid_grant` | Mã đã dùng hoặc hết hạn; PKCE sai; refresh token hết hạn, không tồn tại, hoặc **bị dùng lại** (cả họ token bị thu hồi) | `ttm login` lại |
| `invalid_client` | Client không tồn tại | Kiểm `--host` |
| `invalid_scope` | Scope lạ trong `--scope` | Dùng tên scope đúng (xem [Đăng nhập](dang-nhap.md)) |
| `invalid_request` | Thiếu tham số | Báo lỗi nếu gặp khi dùng `ttm` |
| `invalid_target` | (MCP từ xa) `resource` không phải `…/api/v1/mcp` | Dùng đúng URL MCP |

## Chung

| Code | HTTP | Nghĩa |
| --- | --- | --- |
| `bad_request` | 400 | Đầu vào sai; xem `rule` và `field` |
| `not_found` | 404 | Không có, hoặc bạn không xem được (server không phân biệt hai trường hợp) |
| `conflict` | 409 | Xung đột trạng thái; xem `rule` |
| `precondition_failed` | 412 | Dữ liệu đổi từ lúc bạn đọc (`rule: rev-conflict`); đọc lại rồi thử lại |
| `rate_limited` | 429 | Quá giới hạn (ví dụ 60 lần ghi Notee mỗi giờ mỗi token); chờ rồi thử lại |
| `internal` | 500 | Lỗi server; thử lại sau, báo lỗi nếu lặp lại |
| `http_<mã>` | bất kỳ | Phản hồi không phải problem+json (proxy, cổng 502…) |

Mã cục bộ của `ttm`: `network` (không kết nối được), `decode` (phản hồi lạ), `invalid_url`, `error` (lỗi khác: chưa đăng
nhập, file không đọc được…).

## Notee: ghi page

| Code / Rule | HTTP | Nghĩa | Cách xử lý |
| --- | --- | --- | --- |
| `too_many_blocks` | 413 | Lần ghi thêm, đổi hoặc xoá hơn 1.000 block; hoặc lặp `<unknown>`/`<truncated>` quá mức | Chia nhỏ (nhiều `update_content`, hoặc tạo rồi thêm dần) |
| `markdown_too_large` | 413 | Markdown hơn 2 MB | Chia nhỏ |
| `would_delete_children` | 409 | Nội dung mới bỏ mất page con hoặc database con | Giữ thẻ `<page>`/`<database>`, hoặc thêm `allow_deleting_content` / `--allow-deleting-content` nếu thật sự muốn xoá |
| `would_delete_truncated` | 409 | Bỏ dòng `<truncated/>` của page dài | Giữ dòng đó ở cuối, hoặc `allow_deleting_content` |
| rule `one_of` | 400 | Không có gì để đổi, hoặc dùng cả `update_content` lẫn `replace_content` | Chọn một |
| rule `suggest_content_only` | 400 | Đề xuất kèm `title`/`properties` | Đổi tiêu đề, cột bằng `mode: apply` |
| rule `mode` | 400 | `mode` khác `apply`/`suggest` | |
| rule `is_database` | 400 | Đọc database như page | Dùng `notee-fetch` (trả schema) hoặc `GET /databases/{id}/schema` |
| rule `batch_size` | 400 | Tạo 0 hoặc hơn 20 page một lần | 1–20 |
| rule `parent_shape` | 400 | `parent` có nhiều hơn một trong `page_id`, `database_id`, `workspace_id` | Đúng một |
| `rate_limited` | 429 | Quá 60 lần ghi của agent mỗi giờ (theo token) | Gộp thay đổi vào ít lần ghi hơn |

Cảnh báo (không phải lỗi, nằm trong `warnings`): `update_content[i]: old_str not found; skipped`,
`… old_str matches N places; give more context or replace_all; skipped`, `old_str is empty`, thẻ bị bỏ
(`<unknown url=…> is not a block of this page; left out`, `<page> without the url of an existing page; left out`…).

## Notee: comment và đề xuất

| Code / Rule | HTTP | Nghĩa | Cách xử lý |
| --- | --- | --- | --- |
| `anchor_not_found` | 422 | Đoạn trích không có trong page (hoặc tài nguyên không có chữ) | Chép đúng chữ từ `notee-fetch`/`ttm notes get` |
| `ambiguous_anchor` | 422 | Đoạn trích xuất hiện nhiều chỗ | Thêm `prefix` và/hoặc `suffix` |
| rule `quote_length` | 400 | `exact` rỗng hoặc hơn 1.000 ký tự | |
| `stale` (trong `stale_change_ids`) | | Thay đổi của đề xuất có đoạn neo đã mất | Đề xuất lại trên nội dung mới |

## Notee: database

| Code / Rule | HTTP | Nghĩa | Cách xử lý |
| --- | --- | --- | --- |
| `sql_syntax` | 400 | Sai cú pháp SQL | Xem vị trí trong thông báo |
| `sql_not_select` | 400 | Không phải đúng một câu `SELECT` (có DML, nhiều câu) | Một câu `SELECT` |
| `sql_table` | 400 | Đọc bảng khác `rows` | Chỉ `FROM rows` và CTE của bạn |
| `sql_function` | 400 | Hàm không được phép | Xem danh sách hàm ở [Notee](notee.md) |
| `sql_forbidden` | 400 | Mệnh đề bị cấm (`SELECT INTO`, `FOR UPDATE`, tham số `$1`, CTE tên `rows`, `WITH` lồng, `LATERAL`, ép kiểu `reg*`) | Bỏ mệnh đề đó |
| `sql_error` | 400 | Postgres báo lỗi khi chạy | Sửa theo thông báo |
| `query_timeout` | 408 | Truy vấn quá 3 giây | Lọc bớt, thêm `LIMIT` |
| `result_too_large` | 413 | Một trang kết quả quá 4 MB | Chọn ít cột, giảm `page_size` |
| `query_busy` | 429 | Quá 2 truy vấn cùng lúc của bạn, hoặc server bận | Thử lại sau giây lát |
| `sql_sandbox_unavailable` | 503 | Server chưa bật được SQL an toàn | Dùng bộ lọc JSON |
| rule `sql_or_filter` | 400 | Có cả `sql` lẫn `filter` | Một trong hai |
| rule `unknown_prop` | 400 | Property không tồn tại trong `filter`/`sorts` | Đọc schema |
| rule `filter_shape` | 400 | Bộ lọc sai dạng | `{"property", "op", "value"}`, `{"and": []}`, `{"or": []}` |
| rule `invalid_cursor` | 400 | Cursor hỏng | Dùng đúng `next_cursor` |
| `row_deleted` | 409 | Dòng đang ở thùng rác | Khôi phục trên web hoặc tạo dòng mới |
| rule `row_elsewhere` | 400 | Id là dòng của database khác | |

## Factory

Hầu hết là `409 conflict` với `rule` sau:

| Rule | Nghĩa | Cách xử lý |
| --- | --- | --- |
| `split-limit` | Vượt giới hạn task con (độ sâu, mỗi run, mỗi task gốc) hoặc vai trò không được chia; `params` có giới hạn hiện tại | Chia lại ít hơn hoặc nông hơn |
| `approval-required` | Chuyển trạng thái cần duyệt; hoặc change request phải do vai trò khác duyệt | `ttm task approve` bởi người có vai trò |
| `tests-required`, `ci-required` | Lần chạy cuối chưa có test / CI đạt | Chạy test, đính `test_result` |
| `blocked-by-dependency` | Task phụ thuộc hoặc task con chưa xong | |
| `wip-limit` | Cột đích đủ WIP | |
| `sprint-required` | Cần đưa task vào sprint | |
| `budget-exceeded` | Hết ngân sách token của task | |
| `transition-not-allowed` | Workflow không cho chuyển (hoặc trạng thái lạ) | Xem workflow trên web |
| `rev-conflict` (412) | Task đổi từ lúc đọc | Chạy lại lệnh |
| `run-active` | Task đã có run đang chạy | Chờ hoặc `ttm agent stop` |
| `run-finished` | Run đã kết thúc | |
| `not-offered` | Run không còn chờ nhận | |
| `no-approval-pending` | Task không chờ duyệt | |
| `not-pending`, `not-author` | Change request đã quyết / người tạo không tự duyệt | |
| `merge-finished` | Phần tử merge queue đã xong | |
| `key_taken` | Key project đã dùng | Chọn `--key` khác |
| `ambiguous-key` | Key trùng ở nhiều project | Dùng id task |
| `dependency-cycle`, `not-found` (`field: dependencies`) | Phụ thuộc vòng hoặc không tồn tại | |

`ttm sync` nhận `409` có danh sách `conflicts` (task, trường, base, local, server) và in chúng; xem [Factory](factory.md).

## Runner

| Code | Nghĩa | Cách xử lý |
| --- | --- | --- |
| `repo_not_configured` | Project của job không có clone trong `runner.toml`, hoặc `origin` của clone không trùng repo của job | `ttm runner config --repo KEY=<đường dẫn clone>` |
| `invalid_branch` | Tên nhánh của job không hợp lệ theo git | Sửa task/nhánh trên web |

Hai mã này được báo lên server làm `error` khi kết thúc run.

## Tool MCP

| Code | Nghĩa |
| --- | --- |
| `no_run` | Tool cần một lần chạy agent (`TTM_RUN_ID`) |
| `no_task` | Thiếu `task` và không có `TTM_TASK` |
| `no_project` | Thiếu `project` và không có `TTM_PROJECT` |
| `unsupported` | Server không hỗ trợ tool này |

## Thông báo thường gặp của ttm

| Thông báo | Nghĩa |
| --- | --- |
| `not signed in to <host> (profile <p>); run ttm login` | Không có token cho (profile, host) này |
| `the saved token has expired; run ttm login` | Token hết hạn, không có refresh token |
| `the session expired and could not be renewed; run ttm login` | Gia hạn thất bại |
| `the server rejected this token (…)` | `ttm login --token` với token không hợp lệ |
| `cannot reach <host>` | Mạng, DNS, hoặc `--host` sai |
| `this directory is not linked to a project; run ttm project link <KEY>` | Lệnh Factory chạy ngoài repo đã liên kết |
| `no agent harness found; install Claude Code (claude) first` | `claude` không có trên `PATH` của runner |
| `timed out after 5 minutes waiting for the browser` | Chưa bấm **Cho phép** kịp |
| `invalid TOML in <file>: …` | `config.toml`/`credentials.toml`/`runner.toml` sai cú pháp |
