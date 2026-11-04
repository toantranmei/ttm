---
title: Notee
description: Tìm, đọc, tạo và sửa page Notee, đề xuất sửa, truy vấn và ghi database, comment neo theo đoạn trích, bằng CLI hoặc MCP.
order: 7
section: guides
---

Mọi việc dưới đây làm được bằng **CLI** (`ttm notes …`) hoặc **MCP** (tool `notee-*`). Hai bên gọi cùng API nên quy tắc,
giới hạn và lỗi như nhau. Page được đọc và ghi dưới dạng [Notee-flavored Markdown](notee-markdown.md).

| Việc | CLI | Tool MCP | Scope |
| --- | --- | --- | --- |
| Tìm page | `ttm notes search` | `notee-search` | `notee:read` |
| Đọc page hoặc schema database | `ttm notes get` | `notee-fetch` | `notee:read` |
| Tạo page | `ttm notes create` | `notee-create-pages` | `notee:write` |
| Sửa page | `ttm notes edit` | `notee-update-page` | `notee:write` (hoặc `notee:comment` để đề xuất) |
| Đề xuất sửa | `ttm notes edit --suggest`, `notes suggestions`, `notes resolve` | `notee-update-page` (`mode: suggest`), `notee-get-suggestions`, `notee-resolve-suggestion` | `notee:comment`, `notee:write` để duyệt |
| Truy vấn database | `ttm notes query` | `notee-query-database` | `notee:read` |
| Ghi, xoá dòng | `ttm notes rows upsert/delete` | `notee-upsert-rows`, `notee-delete-rows` | `notee:write` |
| Comment | `ttm notes comments`, `ttm notes comment` | `notee-get-comments`, `notee-create-comment` | `notee:read`, `notee:comment` |
| Di chuyển, nhân bản, tạo/sửa database | (chưa có lệnh) | `notee-move-pages`, `notee-duplicate-page`, `notee-create-database`, `notee-update-database` | `notee:write` |

## Id và URL

Mọi tham số page/database nhận **id** (UUID) hoặc **URL bất kỳ** của nó: `ttm` lấy UUID cuối cùng trong đường dẫn, kể cả
khi nó nằm ở đuôi một slug.

```sh
ttm notes get 0190f3b0-1111-7aaa-8bbb-0123456789ab
ttm notes get https://toantranmei.com/notes/0190f3b0-1111-7aaa-8bbb-0123456789ab
ttm notes get "https://toantranmei.com/notes/ke-hoach-quy-4-0190f3b0-1111-7aaa-8bbb-0123456789ab?x=1"
```

Workspace thì luôn là id (xem `ttm workspace list`).

## Tìm

```sh
ttm notes search "ke hoach" --limit 5
ttm notes search "OKR" --workspace 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

- Tìm trong tiêu đề và nội dung các page bạn xem được, **không phân biệt dấu và hoa thường**; mọi từ phải có mặt.
- Tiêu đề khớp xếp trước, rồi page sửa gần đây. Server trả tối đa 20 kết quả; `--limit` (mặc định 10) cắt bớt.
- Mặc định tìm trong workspace hiện tại của profile; chưa chọn workspace thì tìm mọi workspace.
- Câu tìm tối đa 8 từ và 200 ký tự (`400 bad_request: query too long`).

## Đọc

```sh
ttm notes get <page>                    # Markdown ra stdout
ttm notes get <page> -o page.md         # ghi ra file, in "Saved <tiêu đề> (page.md)."
ttm notes get <page> --json             # thêm id, title, icon, url, parent, properties, truncated, unknown_block_ids
ttm notes get <database>                # tiêu đề và danh sách property
```

```text
# Việc nhóm
- Name (title)
- Status (status)
- Due (date)
- Owner (person)
```

- Id là **dòng database**: `get` trả nội dung page của dòng; giá trị các cột nằm trong `properties` (chỉ có với `--json`).
- **Page dài** (hơn 20.000 node) bị cắt: dòng cuối Markdown là `<truncated url="…" blocks="n"/>`, kết quả có
  `truncated: true` và `unknown_block_ids`. CLI nhắc trên stderr:
  `note: page truncated; read the rest with --block <id>: b1, b2`. Đọc từng phần bằng `--block <id>` (MCP: `block`).
- Block Markdown không biểu diễn được (sketch, embed, synced block…) hiện dưới dạng `<unknown url="…" alt="…"/>`; id của
  chúng cũng nằm trong `unknown_block_ids`.

Với MCP, `notee-fetch` trả Markdown bọc trong `<page id="…" title="…" url="…">…</page>` cho model và dữ liệu có cấu trúc
(`kind: "page"` hoặc `kind: "database"`) cho client.

## Tạo page

```sh
ttm notes create --parent <page> -f ke-hoach.md
ttm notes create --workspace <workspace-id> -f note.md --title "Ghi chú họp" --icon "📝"
ttm notes create --database <database> -f task.md --title "Viết tài liệu"     # tạo một dòng
cat note.md | ttm notes create --parent <page> -f -
```

```text
Created Ghi chú họp (https://toantranmei.com/notes/0190f3c4-…)
```

- Chọn đúng **một** chỗ đặt: `--parent` (page cha), `--database` (thành dòng của database) hoặc `--workspace` (cấp cao
  nhất của workspace).
- Tiêu đề: `--title`, nếu không thì dòng `# ` đầu tiên của file, nếu không thì tên file (bỏ đuôi). Dòng `# ` đó vẫn nằm
  trong nội dung; xoá nó khỏi file nếu không muốn tiêu đề lặp lại.
- Tối đa 1.000 block và 2 MB Markdown mỗi page.

MCP `notee-create-pages` tạo 1–20 page một lần (tổng ≤ 1.000 block), mỗi page có `title`, `icon`, `content` (Markdown) và
`properties` (giá trị cột theo tên khi cha là database):

```json
{
  "parent": { "database_id": "0190f3c0-…" },
  "pages": [
    { "title": "Viết tài liệu", "properties": { "Status": "Doing", "Due": "2026-11-15" } },
    { "title": "Phát hành 0.2", "icon": "🚀", "content": "## Việc cần làm\n- [ ] Changelog" }
  ]
}
```

## Sửa page

### Tìm và thay (khuyên dùng)

```sh
ttm notes edit <page> --old "Q3" --new "Q4"
ttm notes edit <page> --old "Trạng thái: nháp" --new "Trạng thái: duyệt" --old "v0.1" --new "v0.2"
ttm notes edit <page> --old "TODO" --new "DONE" --replace-all
```

- Mỗi `--old` đi với `--new` cùng vị trí; số lượng phải bằng nhau. Các cặp áp **theo thứ tự** lên Markdown hiện tại.
- `--old` khớp đúng chữ, **phân biệt hoa thường**, và phải khớp **đúng một chỗ**. `--replace-all` thay mọi chỗ (áp cho mọi
  cặp của lệnh).
- Cặp không khớp (hoặc khớp nhiều chỗ khi không có `--replace-all`) bị **bỏ qua**, không làm hỏng các cặp khác; server trả
  cảnh báo:

```text
Page updated.
warning: update_content[1]: old_str matches 3 places; give more context or replace_all; skipped
```

Tìm-thay giữ nguyên con trỏ của người đang mở page và tạo bản ghi lịch sử nhỏ. Đọc page bằng `ttm notes get` trước để
chép `--old` chính xác (kể cả ký tự thoát như `\*` và tab thụt lề).

### Thay cả nội dung

```sh
ttm notes get <page> -o page.md
$EDITOR page.md
ttm notes edit <page> --replace -f page.md
```

Giữ nguyên mọi thẻ `<unknown …/>`, `<truncated …/>`, `<page>` và `<database>` bạn đã đọc: thẻ `<unknown>` và
`<truncated>` được khôi phục từ bản gốc khi ghi; xoá chúng là xoá block tương ứng.

### Quy tắc khi ghi

| Tình huống | Kết quả |
| --- | --- |
| Nội dung mới bỏ mất page con hoặc database con | `409 would_delete_children`, trừ khi có `--allow-deleting-content` |
| Bỏ dòng `<truncated/>` của page dài | `409 would_delete_truncated`, trừ khi có `--allow-deleting-content` |
| Thêm, đổi hoặc xoá hơn 1.000 block | `413 too_many_blocks`: chia nhỏ lần ghi (block bị xoá không tính khi có `--allow-deleting-content`) |
| Markdown hơn 2 MB | `413 markdown_too_large` |
| Hơn 60 lần ghi trong một giờ (theo token) | `429 rate_limited` |
| Không có gì thay đổi | `No change applied.` |

Mỗi lần ghi qua API, server **chụp một phiên bản** của page trước rồi mới áp thay đổi dưới dạng diff theo block. Lịch sử
phiên bản ghi tác giả là “<bạn> (via agent <tên>)” (CLI dùng tên `ttm-cli`, MCP dùng tên ứng dụng client) và có nút hoàn
tác thay đổi của agent.

## Đề xuất sửa (suggestion)

```sh
ttm notes edit <page> --old "ra mắt tháng 11" --new "ra mắt tháng 12" --suggest
```

```text
Suggestion 0190f3d1-… created.
```

Đề xuất không đổi page; editor hiện nó như tracked change để người có quyền sửa **chấp nhận** hoặc **từ chối**, từng thay
đổi hoặc cả đề xuất.

- Khi không truyền `--suggest` (MCP: không truyền `mode`), token đi theo cài đặt **agent_write_mode** của workspace:
  `direct` (mặc định) ghi thẳng, `suggest` thì mọi lần ghi của agent thành đề xuất.
- Token chỉ có `notee:comment` (không có `notee:write`) **luôn** đề xuất; ép `mode: apply` sẽ bị `403 insufficient_scope`.
- Đề xuất chỉ đổi nội dung; MCP gửi `title` hoặc `properties` kèm `mode: suggest` bị từ chối (`suggest_content_only`).

Xem và duyệt:

```sh
ttm notes suggestions <page>          # đề xuất đang chờ
ttm notes suggestions <page> --all    # cả đã chấp nhận, đã từ chối, đã lỗi thời
```

```text
0190f3d1-…  pending  An Nguyễn (via claude-code)  1 change(s)
    [c1] replace
      - ra mắt tháng 11
      + ra mắt tháng 12
```

```sh
ttm notes resolve <suggestion> --accept                       # chấp nhận mọi thay đổi
ttm notes resolve <suggestion> --accept --change c1 --change c3
ttm notes resolve <suggestion> --reject
```

```text
Suggestion 0190f3d1-…: accepted
Stale (anchor text gone): c3
```

Thay đổi có đoạn neo không còn trong page (đã bị sửa sau khi đề xuất) được trả về là **stale** và không áp. Duyệt cần
`notee:write` và quyền sửa page.

## Database

### Xem schema

```sh
ttm notes get <database> --json
```

Schema có `properties` (`id`, `name`, `type`, `options` là tên lựa chọn của select/multi-select/status, `config`) và
`views`. Kiểu property: `title`, `text`, `number`, `select`, `multi_select`, `status`, `date`, `person`, `files`,
`checkbox`, `url`, `email`, `phone`, `relation`, và các kiểu tính toán (`formula`, `rollup`, `created_time`, `last_edited_time`,
`created_by`, `last_edited_by`, `unique_id`). Property trỏ tới database bạn không xem được bị ẩn.

### Truy vấn bằng SQL

```sh
ttm notes query <database> --sql 'SELECT "Name", "Status", "Due" FROM rows WHERE "Status" = '\''Doing'\'' ORDER BY "Due" LIMIT 20'
```

```text
0190f3c5-…  {"Due":"2026-11-15","Name":"Viết tài liệu","Status":"Doing"}
0190f3c6-…  {"Due":"2026-11-20","Name":"Phát hành 0.2","Status":"Doing"}
```

Quy tắc (giống chế độ SQL của view, chạy trên Postgres ở chế độ chỉ đọc):

- Đúng **một** câu `SELECT` (được `WITH` ở cấp ngoài cùng). Không `INSERT/UPDATE/DELETE`, không `SELECT INTO`, không
  `FOR UPDATE`, không tham số `$1`, không `LATERAL`, không `WITH` trong truy vấn con.
- Bảng duy nhất là **`rows`** (cùng các CTE bạn tự đặt, trừ tên `rows`), cộng các hàm bảng `jsonb_array_elements`,
  `jsonb_array_elements_text`, `jsonb_each`, `jsonb_each_text`, `jsonb_object_keys`, `unnest`. Không tên có schema.
- **Cột = tên property**, đặt trong dấu nháy kép khi có chữ hoa, dấu cách hay dấu tiếng Việt: `"Trạng thái"`. Thêm các cột
  hệ thống `id`, `position`, `created_time`, `last_edited_time`, `created_by`, `last_edited_by` (trừ khi một property
  trùng tên).
- Dạng giá trị: select/status là **tên lựa chọn**; multi-select là mảng JSON tên lựa chọn; ngày là chuỗi ISO 8601;
  checkbox là boolean; formula và rollup là giá trị đã tính.
- Chỉ các hàm thuần trong danh sách cho phép: chuỗi (`lower`, `upper`, `trim`, `concat`, `replace`, `split_part`,
  `regexp_replace`, `to_char`…), số (`abs`, `round`, `floor`, `power`…), ngày (`now`, `current_date`, `date_trunc`,
  `date_part`, `extract`, `age`, `make_date`…), tổng hợp (`count`, `sum`, `avg`, `min`, `max`, `string_agg`,
  `array_agg`, `jsonb_agg`, `percentile_cont`…), cửa sổ (`row_number`, `rank`, `lag`, `lead`…), JSON và mảng
  (`jsonb_array_length`, `jsonb_extract_path_text`, `to_jsonb`, `cardinality`…). Hàm `pg_*`, `set_config`, I/O, sleep bị
  cấm.
- Mỗi truy vấn chạy tối đa **3 giây**, mỗi trang kết quả tối đa **4 MB**, mỗi người tối đa **2 truy vấn cùng lúc**.

| Lỗi | Nghĩa |
| --- | --- |
| `sql_syntax` | Sai cú pháp (thông báo kèm vị trí) |
| `sql_not_select` | Không phải đúng một câu SELECT |
| `sql_table` | Đọc bảng khác `rows` |
| `sql_function` | Hàm không được phép |
| `sql_forbidden` | Mệnh đề không được phép (`SELECT INTO`, `FOR UPDATE`, tham số, CTE tên `rows`…) |
| `sql_error` | Postgres từ chối khi chạy (sai kiểu, chia cho 0…) |
| `query_timeout` (408) | Quá 3 giây |
| `result_too_large` (413) | Một trang quá 4 MB: chọn ít cột hơn hoặc giảm `--page-size` |
| `query_busy` (429) | Quá nhiều truy vấn cùng lúc: thử lại sau giây lát |
| `sql_sandbox_unavailable` (503) | Server chưa sẵn sàng cho SQL; dùng bộ lọc JSON |

### Truy vấn bằng bộ lọc JSON

```sh
ttm notes query <database> --filter filter.json
echo '{"property": "Status", "op": "eq", "value": "Done"}' | ttm notes query <database> --filter -
```

`filter.json` là bộ lọc trần hoặc đối tượng có `filter` và `sorts`:

```json
{
  "filter": {
    "and": [
      { "property": "Status", "op": "in", "value": ["Todo", "Doing"] },
      { "property": "Due", "op": "lte", "value": "2026-11-30" },
      { "or": [
        { "property": "Owner", "op": "is_not_empty" },
        { "property": "Priority", "op": "eq", "value": "High" }
      ] }
    ]
  },
  "sorts": [{ "property": "Due", "direction": "asc" }]
}
```

- Điều kiện: `{"property", "op", "value"}` (`prop` cũng được, `op` mặc định `eq`); lồng bằng `{"and": [...]}`,
  `{"or": [...]}`.
- Lựa chọn của select, multi-select, status viết bằng **tên**.
- `sorts`: `[{"property", "direction": "asc" | "desc"}]` (mặc định `asc`).

| Toán tử | Dùng cho |
| --- | --- |
| `eq`, `neq` | mọi kiểu (với multi-select, person, relation: chứa / không chứa giá trị) |
| `contains`, `not_contains` | text; multi-select, person, relation |
| `starts_with`, `ends_with` | text |
| `gt`, `gte`, `lt`, `lte` | number, date (trước/sau ngày) |
| `in`, `not_in` | select, status (một trong / không thuộc); multi-select (`in` = chứa bất kỳ) |
| `contains_all` | multi-select, person, relation |
| `is_empty`, `is_not_empty` | mọi kiểu |
| `between` | number, date |
| `is_relative` | date: `today`, `this_week`… |

Property không tồn tại: `400 bad_request` (`rule: unknown_prop`); bộ lọc sai dạng: `rule: filter_shape`.

### Phân trang

```sh
ttm notes query <database> --sql 'SELECT * FROM rows' --page-size 50
# stderr: more rows: --cursor eyJvIjo1MH0
ttm notes query <database> --sql 'SELECT * FROM rows' --page-size 50 --cursor eyJvIjo1MH0
```

Trang có 1–100 dòng (mặc định 100). Kết quả JSON: `columns`, `rows` (`id`, `values` theo tên cột), `next_cursor` (`null`
ở trang cuối). Với SQL, cột `id` rỗng nếu câu `SELECT` không chọn `id`.

Ở MCP, truyền đúng **một** trong `sql` hoặc `filter` cho `notee-query-database`.

### Ghi và xoá dòng

`rows.json`:

```json
[
  { "values": { "Name": "Viết changelog", "Status": "Todo", "Due": "2026-11-20" } },
  { "id": "0190f3c5-…", "values": { "Status": "Done" } }
]
```

```sh
ttm notes rows upsert <database> -f rows.json
ttm notes rows delete <database> 0190f3c5-… 0190f3c6-…
```

```text
2 row(s) saved: 0190f3c7-…, 0190f3c5-…
```

- Không có `id` thì tạo dòng mới; có `id` thì chỉ đổi các cột đã nêu.
- 1–100 dòng mỗi lần. Giá trị theo **tên property**, lựa chọn theo **tên**.
- Xoá dòng chuyển vào thùng rác. Sửa dòng đã ở thùng rác: `409 row_deleted`; id thuộc database khác: `row_elsewhere`.
- Ghi dòng không cho phép qua SQL.

## Comment

```sh
ttm notes comments <page>          # thread đang mở
ttm notes comments <page> --all    # cả thread đã đóng
```

```text
Thread 0190f3e0-… on "ra mắt tháng 12"
  An Nguyễn: Chốt ngày cụ thể chưa?
  Bình: Ngày 15.
```

Viết comment:

```sh
ttm notes comment <page> -m "Cần thêm số liệu"                                  # comment cho cả page
ttm notes comment <page> --quote "ra mắt tháng 12" -m "Chốt ngày cụ thể?"       # neo vào đoạn trích
ttm notes comment <page> --quote "9h" --suffix " sáng thứ Hai" -m "Đổi sang 10h"
ttm notes comment <page> --thread 0190f3e0-… -m "Đồng ý"                        # trả lời thread
```

- Neo theo **đoạn trích văn bản** (kiểu W3C `TextQuoteSelector`): `exact` (1–1.000 ký tự) phải khớp đúng một chỗ trong
  page.
- Không thấy: `422 anchor_not_found`. Khớp nhiều chỗ: `422 ambiguous_anchor`; thêm vài chữ ngay trước (`--prefix`) hoặc
  ngay sau (`--suffix`).
- Neo bền khi văn bản xung quanh đổi. Người được nhắc tên và người theo dõi page nhận thông báo.
- `--thread` không dùng chung với `--quote`.

MCP `notee-create-comment` nhận `page`, `text`, `quote` (`{exact, prefix?, suffix?}`) hoặc `thread_id`.

## Ví dụ cho agent

Gợi ý câu lệnh cho Claude khi đã cài MCP:

- *Tìm page “Họp tuần”, đọc nó, rồi thêm mục “Việc tuần sau” cuối page bằng update_content.*
- *Đề xuất sửa (mode suggest) mọi chỗ ghi “quý 3” thành “quý 4” trong page X, đừng ghi thẳng.*
- *Trong database “Việc nhóm”, liệt kê việc trạng thái Doing quá hạn bằng SQL, rồi comment vào từng page nhắc người phụ
  trách.*
