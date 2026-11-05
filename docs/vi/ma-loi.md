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

`ttm` in lỗi ra stderr theo một mẫu (từ 0.2.0):

```text
✕ <tóm tắt>
  rule: <rule>  field: <field>
  Fix: <cách sửa>
  Help: https://toantranmei.com/help/ttm/ma-loi#<rule>
```

`rule` là `rule` của server, không có thì `code`. Ví dụ:

```text
✕ At most 10 subtasks per run (10 already).
  rule: split-limit
  Fix: split into fewer subtasks (see params.per_run)
  Help: https://toantranmei.com/help/ttm/ma-loi#split-limit
```

Lỗi dùng sai lệnh (`missing_flag`, `confirmation_required`, `unsupported_feature`, `invalid_flag`) thoát với mã **2**;
lỗi khác mã 1. Với `--plain`, dòng đầu là `ERROR: <tóm tắt>`.

Với `--json`, stdout có thêm `{"error": {"code", "message", "status", "rule", "field", "params"}}` (ba trường cuối chỉ
có khi server gửi) để script rẽ nhánh theo `rule`. Tool MCP trả `isError: true`, văn bản `<thông báo> (<code>)` và
`structuredContent.error` cùng các trường đó: agent rẽ nhánh theo `rule` (`split-limit`, `rev-conflict`,
`approval-required`) hoặc đọc `params.required` của `insufficient_scope`.

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
| `invalid_worktree` | `worktree_hint` của job là đường dẫn tuyệt đối, có `..`, hoặc dẫn ra ngoài `worktree_root` | Báo lỗi server; runner không bao giờ tạo worktree ngoài `worktree_root` |
| `ttm_mcp_missing` | Runner không tìm thấy file chạy `ttm-mcp` (cạnh `ttm` hoặc trên `PATH`), là MCP server `ttm` của agent | Cài lại ttm (gói có cả `ttm` và `ttm-mcp`); thiếu nó thì `ttm runner up` không chạy, còn runner đang chạy sẽ ngừng nhận job cho tới khi có lại |

Các mã này được báo lên server làm `error` khi kết thúc run.

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
| `` not signed in to <host> (profile <p>); run `ttm login` `` | Không có token cho (profile, host) này |
| `the saved token has expired; run ttm login` | Token hết hạn, không có refresh token |
| `the session expired and could not be renewed; run ttm login` | Gia hạn thất bại |
| `the server rejected this token (…)` | `ttm login --token` với token không hợp lệ |
| `cannot reach <host>` | Mạng, DNS, hoặc `--host` sai |
| `this directory is not linked to a project; run ttm project link <KEY>` | Lệnh Factory chạy ngoài repo đã liên kết |
| ``no agent harness found; install Claude Code (`claude`) first`` | `claude` không có trên `PATH` của runner |
| `timed out after 5 minutes waiting for the browser` | Chưa bấm **Cho phép** kịp |
| `invalid TOML in <file>: …` | `config.toml`/`credentials.toml`/`runner.toml` sai cú pháp |

## Rule và cách sửa

Mỗi mục dưới đây là đích của dòng `Help:`. Dòng **Fix** là câu `ttm` in ra.

### `missing_flag`

Thiếu cờ bắt buộc và không thể hỏi (có `--yes` hoặc không phải terminal). Mã thoát 2.

**Fix:** pass the flag shown in field, or run in a terminal to be asked

### `confirmation_required`

Thao tác cần xác nhận nhưng không có terminal và không có `--yes`. Mã thoát 2.

**Fix:** confirm in a terminal, or pass --yes

### `invalid_flag`

Giá trị của cờ không hợp lệ (ví dụ key không đúng `^[A-Z][A-Z0-9]{1,9}$`). Mã thoát 2.

**Fix:** fix the value of the flag shown in field (ttm <command> --help lists the choices)

### `unsupported_feature`

Server (hoặc runner) chưa có tính năng này (`discovery.features` thiếu chuỗi tương ứng, hoặc job `requires` tính năng runner không có). `ttm` không gọi API. Mã thoát 2.

**Fix:** this server is older than this ttm: update the server or use another host

### `rev-conflict`

Đối tượng đã đổi từ phiên bản bạn đọc (`If-Match`/`rev` cũ).

**Fix:** someone changed it meanwhile: fetch it again (ttm sync) and redo the change

### `draft-stale`

Bản nháp workflow đã đổi từ lúc bạn đọc (`base_version` cũ).

**Fix:** the draft changed since you read it: show it again and reapply your edit

### `upgrade-preview-stale`

Workflow đã đổi từ lúc xem trước nâng cấp (`preview_hash` không khớp).

**Fix:** the workflow changed since the preview: run the upgrade preview again

### `no-upgrade`

Dự án đã dùng phiên bản workflow mới nhất.

**Fix:** the project already uses the latest workflow version

### `dry-run-required`

Phải chạy thử luật trước khi bật.

**Fix:** test the rule first: ttm automation test <rule> --task <KEY>

### `rule-published`

Luật đã xuất bản không sửa trực tiếp được.

**Fix:** a published rule cannot change: edit the draft and publish again

### `input_required`

Cần thêm dữ liệu đầu vào (mẫu, luật).

**Fix:** this needs more input: pass the missing value shown in field

### `stale`

Change request đã cũ so với server.

**Fix:** the change request is out of date: sync again and send a new one

### `secret_in_file`

File as-code (`.ttm/agents/*.yaml`) chứa khoá giống secret (`token`, `*_token`, `secret`, `password`, `api_key`).

**Fix:** remove the secret from the file and use a secret reference instead

### `invalid_file`

File as-code không hợp lệ theo server.

**Fix:** fix the file at the path shown (ttm workflow validate --file … lists every issue)

### `yaml`

File YAML trong `.ttm/` không parse được (kiểm cục bộ trước khi `ttm sync` gửi đi; có số dòng).

**Fix:** fix the YAML at the line shown, then run ttm sync again

### `purchase_required`

Mẫu có phí và bạn chưa mua.

**Fix:** buy the template on the web first, then install it again

### `sponsors_only`

Mẫu chỉ dành cho Sponsor.

**Fix:** this template is for Sponsors: see https://toantranmei.com/mei-er

### `insufficient_scope`

Token thiếu scope (`params.required`). Token tạo trước 0.2.0 không có `factory:admin`.

**Fix:** sign in again with the missing scope: ttm login --scope '<scopes>'

### `unauthorized`

Không có token hoặc token không hợp lệ.

**Fix:** sign in again: ttm login

### `forbidden`

Vai trò của bạn trong workspace không cho phép.

**Fix:** your role in this workspace does not allow it: ask an owner or admin

### `token_not_allowed`

Route này không mở cho token (chỉ làm trên web).

**Fix:** this action is not open to tokens: do it on the web

### `job_token_not_allowed`

Token của agent đang chạy job không làm được việc này (quản trị, xác nhận).

**Fix:** agents running a job cannot do this: run it yourself with your own token

### `not_found`

Không có đối tượng (hoặc không thuộc về bạn).

**Fix:** check the id or key (list commands show them)

### `seat_limit`

Workspace hết chỗ cho thành viên/agent.

**Fix:** the workspace has no free seat: free one or upgrade the plan on the web

### `plan_limit`

Đã chạm giới hạn của gói (số MCP server, luật…).

**Fix:** the plan limit is reached: upgrade the plan on the web

### `handle_taken`

Handle agent đã có trong workspace.

**Fix:** pick another --handle

### `skill_unchanged`

Phiên bản skill mới giống hệt bản mới nhất (cùng SHA-256).

**Fix:** nothing changed since the last version: edit the skill, then push again

### `skill_limit`

Quá nhiều skill trên agent hoặc workspace.

**Fix:** too many skills on this agent or workspace: remove one first

### `self_review`

Bạn không tự duyệt phiên bản skill của mình được (workspace có ≥ 2 admin).

**Fix:** someone else must review your own skill version

### `skill_md_missing`

Skill không có `SKILL.md` ở gốc.

**Fix:** add a SKILL.md with a name and description front matter

### `front_matter`

Front matter của `SKILL.md` thiếu `name`/`description` hoặc `ttm.tags` sai.

**Fix:** fix the front matter of SKILL.md

### `path_unsafe`

Đường dẫn trong skill không an toàn (`..`, tuyệt đối, `\`, `.git/`, symlink).

**Fix:** remove the file or rename it to a plain relative path

### `too_large`

Skill vượt 5 MB sau giải nén (hoặc tỉ lệ nén > 100).

**Fix:** make the skill smaller (at most 5 MB)

### `too_many_files`

Skill có quá 200 file.

**Fix:** keep at most 200 files (list build output in .gitignore)

### `zip_invalid`

Gói skill không phải zip hợp lệ.

**Fix:** pack the skill again with ttm skill push <dir>

### `skill_hash_mismatch`

Runner tải skill nhưng SHA-256 không khớp với job: job bị từ chối.

**Fix:** the skill download is corrupt: run the job again

### `ttm_mcp_missing`

Runner không tìm thấy file chạy được `ttm-mcp` (cạnh `ttm` hoặc trên `PATH`). Thiếu nó agent không có tool Factory, nên job bị từ chối trước khi agent chạy; `ttm runner up` không khởi động, còn runner đang chạy ngừng nhận job cho tới khi file có lại.

**Fix:** ttm-mcp is missing next to ttm: reinstall ttm (it ships both), then restart the runner

### `secrets_disabled`

Server chưa bật mã hoá secret.

**Fix:** secrets are turned off on this server: ask its operator

### `storage_disabled`

Server chưa bật lưu file.

**Fix:** file storage is turned off on this server: ask its operator

### `oauth_client_unavailable`

MCP server này không đăng nhập OAuth được.

**Fix:** this MCP server cannot sign in by OAuth: use --auth workspace with a secret

### `repo_not_configured`

Runner không có clone của dự án trong `runner.toml`, hoặc `origin` không khớp.

**Fix:** link the repository first: ttm project link <KEY>

### `invalid_branch`

Tên nhánh của job không hợp lệ.

**Fix:** fix the task or branch on the web

### `invalid_worktree`

`worktree_hint` của job ra ngoài `worktree_root`.

**Fix:** report it: the runner never creates worktrees outside its root

### `confirm_expired`

Xác nhận đã quá 10 phút.

**Fix:** the confirmation expired (10 minutes): start the action again

### `confirm_used`

Xác nhận đã được áp dụng (mỗi xác nhận chỉ chạy một lần, kể cả khi thao tác lỗi).

**Fix:** this confirmation was already applied: start the action again for a new one

### `confirm_cancelled`

Xác nhận đã bị huỷ.

**Fix:** this confirmation was cancelled: start the action again

### `confirm_tampered`

Request đã lưu không khớp mã băm của xác nhận.

**Fix:** the staged request does not match its confirmation: start the action again

### `confirm_stale`

(Chỉ phía ttm) request đổi giữa lúc xác nhận và áp dụng. Server không trả mã này: draft đổi thì route đích trả `draft-stale`/`upgrade-preview-stale`.

**Fix:** the request changed since it was confirmed: start the action again

### `bearer_required`

Xác nhận hai bước chỉ nhận token Bearer (không cookie).

**Fix:** confirmations need a token: use ttm or an MCP client, not the browser session

### `not_confirmable`

`(method, path)` không thuộc danh sách thao tác xác nhận được.

**Fix:** this change cannot go through a confirmation: run it directly

### `payload_too_large`

Body > 256 KiB hoặc preview > 64 KiB.

**Fix:** the request is too large: send a smaller change

### `split-limit`

Vượt giới hạn tách task con (`params.per_run`).

**Fix:** split into fewer subtasks (see params.per_run)

### `approval-required`

Bước này cần người duyệt.

**Fix:** a person must approve this step: ttm task approve <KEY>

### `agent-own-runner`

Agent quyền đầy đủ/tuỳ chỉnh chỉ chạy trên runner của người giao.

**Fix:** this agent runs only on its delegator's runners: start one with ttm runner up

### `too_many_blocks`

Page quá dài cho một lần đọc.

**Fix:** the page is too long for one call: read it by parts (--block)

### `upgrade_required`

`ttm` cũ hơn `min_cli` của server.

**Fix:** update ttm: ttm self-update

### `network`

Lỗi mạng hoặc host sai.

**Fix:** check the network and the host (ttm doctor)

### `rate_limited`

Gửi quá nhiều request (ví dụ > 30 xác nhận/phút).

**Fix:** wait a little and retry

### `conflict`

Đối tượng đã đổi trong lúc bạn sửa.

**Fix:** it changed meanwhile: fetch it again and retry

### `bad_request`

Dữ liệu đầu vào sai (xem `field`, `rule`).

**Fix:** fix the input named in field

### `authorization_pending`

Đăng nhập thiết bị chưa được duyệt.

**Fix:** finish signing in on the web page, then retry

### `invalid_grant`

Mã/refresh token hết hạn hoặc đã dùng.

**Fix:** sign in again: ttm login
