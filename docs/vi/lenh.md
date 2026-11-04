---
title: Tham chiếu lệnh
description: Mọi lệnh và lệnh con của ttm với cú pháp, cờ, giá trị mặc định, ví dụ, đầu ra JSON và mã thoát.
order: 10
section: reference
---

```text
ttm [OPTIONS] <COMMAND>
```

`ttm --help` và `ttm <lệnh> --help` in trợ giúp gốc (tiếng Anh). Trang này theo phiên bản **0.2.0**. Quy ước đầu ra (dòng kết quả, `Next:`, glyph, huy hiệu, màu): [Giao diện dòng lệnh](giao-dien.md).

## Cờ chung

Dùng được với mọi lệnh, đặt trước hay sau lệnh con đều được (`ttm whoami --json` = `ttm --json whoami`).

| Cờ | Biến | Việc |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | Máy chủ, ví dụ `http://localhost:3000` |
| `--profile <NAME>` | `TTM_PROFILE` | Profile trong `config.toml` |
| `--json` | | Kết quả dạng JSON trên stdout (hình dạng 0.1.6, chỉ thêm khoá mới) |
| `--plain` | `TTM_PLAIN=1` | Chữ thuần cho log: không màu, khung, glyph, emoji |
| `-y`, `--yes` | | Trả lời "có" mọi xác nhận và lấy giá trị mặc định thay vì hỏi (script, CI) |
| `-v`, `--verbose` | `TTM_LOG` | Thêm log ra stderr (`-v` info, `-vv` debug) |
| `-h`, `--help` | | Trợ giúp |
| `-V`, `--version` | | Phiên bản (`ttm 0.2.0`; chỉ ở cấp gốc) |

## Mã thoát

| Mã | Khi nào |
| --- | --- |
| `0` | Thành công |
| `1` | Lỗi (mạng, xác thực, API, file…); `ttm doctor` có mục `FAIL`; `ttm sync --check` thấy lệch; `ttm sync` gặp xung đột; `ttm agent run` kết thúc khác `succeeded` |
| `2` | Sai cú pháp dòng lệnh (cờ lạ, thiếu tham số); thiếu cờ khi không hỏi được (`missing_flag`); cần xác nhận mà không có terminal hay `--yes` (`confirmation_required`); server chưa có tính năng (`unsupported_feature`); giá trị cờ không hợp lệ (`invalid_flag`). Với `ttm hook …`: hành động bị luật chặn |

Lỗi in ra stderr theo một mẫu:

```text
✕ <tóm tắt>
  rule: <rule>  field: <field>
  Fix: <cách sửa>
  Help: https://toantranmei.com/help/ttm/ma-loi#<rule>
```

Với `--plain`, dòng đầu là `ERROR: <tóm tắt>`. Với `--json`, thêm `{"error": {"code", "message", "status?", "rule?",
"field?", "params?"}}` ra stdout (xem [Cấu hình](cau-hinh.md) và [Mã lỗi](ma-loi.md)).

## Tính năng của server

Lệnh mới chỉ gọi API khi `discovery.features` của server có chuỗi tương ứng; thiếu thì in
`✕ This server does not support <x> yet` (rule `unsupported_feature`, thoát 2) và không gọi API.

| Tính năng | Lệnh |
| --- | --- |
| `workflow_v2` | `project init` (mẫu, agent mẫu), `workflow`, `automation` |
| `agent_v2` | `agent list/show/create/edit/pause/resume/archive/skills/mcp`, `skill`, `mcp-server` |
| `ascode_v2` | `workflow validate --file`, `cr` (CR cấu hình), `sync` (thư mục `.ttm/` cấu hình) |
| `templates` | `template` |
| `membership` | dòng Mei-er của `whoami`/`login`, huy hiệu |
| `confirmations` | xác nhận hai bước qua MCP (xem [Tool MCP](mcp-cong-cu.md)) |

## Tóm tắt

| Lệnh | Việc |
| --- | --- |
| [`login`](#login) | Đăng nhập (trình duyệt, `--device`, `--token`) |
| [`logout`](#logout) | Thu hồi token và xoá khỏi máy |
| [`whoami`](#whoami) | Người dùng đang đăng nhập |
| [`status`](#status) | Host, profile, token, workspace đang dùng |
| [`doctor`](#doctor) | Kiểm tra môi trường |
| [`workspace`](#workspace) | `list`, `switch`, `current`, `members` |
| [`token`](#token) | `list`, `create`, `revoke` (mở trang web) |
| [`notes`](#notes) | `search`, `get`, `query`, `comments`, `create`, `edit`, `comment`, `rows`, `suggestions`, `resolve` |
| [`project`](#project) | `init`, `link`, `open`, `status` |
| [`sync`](#sync) | Đồng bộ kế hoạch |
| [`task`](#task) | `list`, `show`, `claim`, `done`, `approve`, `split` |
| [`rules`](#rules) | `apply` |
| [`hook`](#hook) | `pre-bash`, `stop` (do agent gọi) |
| [`workflow`](#workflow) | `show`, `validate`, `simulate`, `diff`, `publish`, `upgrade`, `copy` |
| [`automation`](#automation) | `list`, `show`, `test`, `enable`, `disable`, `runs` |
| [`runner`](#runner) | `up`, `down`, `status`, `config` (+ `config mcp`, `config claude-token`), `install-service`, `uninstall-service` |
| [`agent`](#agent) | `run`, `logs`, `stop`, `list`, `show`, `create`, `edit`, `pause`, `resume`, `archive`, `skills`, `mcp` |
| [`skill`](#skill) | `list`, `push`, `pull`, `show`, `review`, `import` |
| [`mcp-server`](#mcp-server) | `list`, `add`, `connect`, `remove` (MCP từ xa của workspace) |
| [`cr`](#cr) | `list`, `show`, `approve`, `reject` |
| [`inbox`](#inbox) | Hộp thư, `--watch`, `read` |
| [`template`](#template) | `list`, `publish`, `install` |
| [`mcp`](#mcp) | `install` |
| [`self-update`](#self-update) | Cập nhật ttm |
| [`completion`](#completion) | Script gợi ý lệnh |

---

## login

```text
ttm login [--device | --token <PAT>] [--scope <SCOPE>] [--no-browser]
```

Đăng nhập và lưu token cho (profile, host) hiện tại. Mặc định mở trình duyệt (loopback `127.0.0.1` + PKCE, chờ 5 phút).
Ghi host vào profile khi có `--host`/`TTM_HOST` hoặc profile chưa có host.

| Cờ | Mặc định | Việc |
| --- | --- | --- |
| `--device` | | Device flow: in mã để nhập ở `<host>/activate`. Không dùng chung với `--token`. |
| `--token <PAT>` | | Lưu một token có sẵn; `-` đọc từ stdin |
| `--scope <SCOPE>` | `workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run factory:admin offline_access` | Scope xin cấp, cách nhau bằng dấu cách (không áp cho `--token`) |
| `--no-browser` | | Không mở trình duyệt, chỉ in URL |

```sh
ttm login
ttm login --device
printf %s "$PAT" | ttm login --token -
ttm --profile dev login --host http://localhost:3000 --scope "workspace:read notee:read"
```

```text
✓ Signed in as An Nguyễn <an@example.com>
  host https://toantranmei.com · profile default · token in keyring
✦ Mei-er since 2025-11  Thank you for being a Mei-er. You keep this going.
Next: ttm project init
```

Thành viên Mei-er có thêm một dòng vàng gold (hạng, tháng bắt đầu, lời cảm ơn theo hạng; đúng ngày kỷ niệm có lời chúc).
`Next:` là `ttm project init` khi thư mục chưa liên kết project, ngược lại `ttm whoami`. Token cũ thiếu `factory:admin`
vẫn dùng được; lệnh quản trị báo `insufficient_scope` và gợi ý `ttm login`.

JSON: `host`, `profile`, `method` (`browser` | `device` | `token`), `stored_in` (`keyring` | `file` | `env`), `user`
(hoặc `null` khi token thiếu `workspace:read`), `meta.membership` (`{tier, status, since, until, badge}` hoặc `null`).

## logout

```text
ttm logout
```

Thu hồi token truy cập và refresh token trên server, rồi xoá khỏi keyring và `credentials.toml`. Vẫn xoá cục bộ khi server
không xác nhận. Không bỏ được `TTM_TOKEN` (chỉ cảnh báo).

```text
✓ Signed out of https://toantranmei.com
  profile default · revoked on the server: yes
Next: ttm login
```

JSON: `host`, `profile`, `revoked` (server xác nhận), `removed` (có token bị xoá khỏi máy).

## whoami

```text
ttm whoami
```

Gọi `/me` (cần `workspace:read`).

```text
An Nguyễn <an@example.com>
     user  An Nguyễn
    email  an@example.com
     host  https://toantranmei.com
  profile  default
workspace  Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
✦ Mei-er since 2025-11  Thank you for being a Mei-er. You keep this going.
```

Người chưa là thành viên thấy một dòng mờ `Support toantranmei → https://toantranmei.com/mei-er` (chỉ ở `whoami`, không
lệnh nào khác nhắc). Với `--plain`: `user: An Nguyễn`… và huy hiệu `[Mei-er]`.

JSON: `host`, `profile`, `user` (`id`, `email`, `name`, `avatar_url`, `locale`, `plan`, `membership?`), `workspace` (`id`,
`name`, `kind`, `role`, `slug` hoặc `null`), `meta.membership`.

## status

```text
ttm status
```

Những gì máy này sẽ dùng; không cần đăng nhập hay mạng (luôn thoát 0).

```text
✓ Signed in to https://toantranmei.com
  version  ttm 0.2.0
     host  https://toantranmei.com
  profile  default (/Users/an/.config/ttm/config.toml)
    token  ✓ ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
  keyring  ✓ OS keyring
workspace  not chosen
   server  ✓ API v1 (min ttm 0.1.0), features: notee, factory, workflow_v2, agent_v2
Next: ttm workspace switch <name>
```

JSON: `version`, `host`, `profile`, `config_file`, `signed_in`, `token` (`source`, `masked`, `expires_at`, `expired`,
`scope`), `workspace`, `server` (`api_version`, `min_cli`, `features`) hoặc `null`.

## doctor

```text
ttm doctor
```

Mỗi dòng một kiểm tra: `host`, `discovery` (tải mới, không dùng bộ đệm), `version` (so với `min_cli`), `token` (gọi
`/me`), `keyring`, `git`, `claude`, `ttm-mcp`. Mức `ok`, `warn`, `FAIL`. **Thoát 1 nếu có `FAIL`**.

```text
! 1 warning(s), no failure
CHECK      STATUS  DETAIL
────────────────────────────────────────────────────────────────────────
host       ✓       https://toantranmei.com (profile default)
discovery  ✓       api_base https://toantranmei.com/api/v1
version    ✓       ttm 0.2.0, server API v1
token      ✓       ttm_oat_Ab12… (keyring) as an@example.com
keyring    ✓       OS keyring available
git        ✓       git version 2.50.1
claude     !       Claude Code not found on PATH (needed by ttm runner)
ttm-mcp    ✓       /Users/an/.cargo/bin/ttm-mcp
Next: install Claude Code, then ttm doctor
```

`--plain`: `ok`, `warn`, `FAIL` thay cho `✓`, `!`, `✕`, không có dòng kẻ dưới tiêu đề.

JSON: mảng `{name, level, detail}`.

## workspace

### workspace list

```text
ttm workspace list
```

Các workspace bạn là thành viên; `●` (`--plain`: `*`) đánh dấu workspace hiện tại của profile.

```text
2 workspace(s), current: Team Aurora
   NAME         SLUG    ROLE    KIND      ID
────────────────────────────────────────────────────────────────────────
   Cá nhân      -       owner   personal  0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
●  Team Aurora  aurora  member  team      0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

JSON: mảng `{id, name, kind, role, slug?, current}`.

### workspace switch

```text
ttm workspace switch <WORKSPACE>
```

Đặt workspace hiện tại cho profile. `<WORKSPACE>`: id, slug hoặc tên (khớp chính xác, không phân biệt hoa thường), hoặc
phần đầu tên nếu chỉ khớp một workspace. Khớp nhiều: `matches several workspaces (…); use the id`.

```text
✓ Current workspace: Team Aurora
  id 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71 · profile default
Next: ttm project init
```

### workspace current

```text
ttm workspace current
```

In tên và id (tên chỉ khi server trả lời được). JSON: `profile`, `workspace_id`, `workspace`.

### workspace members

```text
ttm workspace members [--workspace <ID>]
```

Thành viên của workspace (mặc định workspace hiện tại): tên kèm huy hiệu Mei-er, email, vai trò. Cần `workspace:read`.

```text
NAME                    EMAIL              ROLE
An Nguyễn [Mei-er]      an@example.com     owner
Lan                     lan@example.com    member
```

JSON: mảng `{user{id, name, email, avatar_url, membership?}, role, membership?}`.

## token

Token được quản lý trên web; các lệnh này mở `<host>/me/ket-noi` (và in URL). JSON: `{"url": "…"}`.

| Lệnh | Cờ | Việc |
| --- | --- | --- |
| `ttm token list` | | Mở danh sách token |
| `ttm token create` | `--name <NAME>`, `--scope <SCOPE>` (phẩy hoặc dấu cách), `--expires <DAYS>` (`1`–`365`, ví dụ `30d`) | Mở form tạo token, gửi kèm giá trị điền sẵn |
| `ttm token revoke [ID]` | | Mở trang thu hồi |

```sh
ttm token create --name CI --scope notee:read,workspace:read --expires 90d
```

## notes

Tham số page/database nhận id hoặc URL. Xem hướng dẫn [Notee](notee.md).

### notes search

```text
ttm notes search <QUERY> [--limit <N>] [--workspace <ID>]
```

| Cờ | Mặc định | Việc |
| --- | --- | --- |
| `--limit <N>` | `10` | Số kết quả tối đa (server trả tối đa 20) |
| `--workspace <ID>` | workspace hiện tại | Giới hạn trong một workspace |

```text
0190f3b0-1111-7aaa-8bbb-0123456789ab  Kế hoạch quý 4
0190f3b0-2222-7aaa-8bbb-0123456789ab  Họp tuần
    …chốt kế hoạch phát hành bản 0.2 trước ngày 15…
```

JSON: mảng `{id, workspace_id, title, icon, snippet}`.

### notes get

```text
ttm notes get <ID> [--block <BLOCK>] [-o <FILE>]
```

In page dạng Notee-flavored Markdown, hoặc tiêu đề và property của database.

| Cờ | Việc |
| --- | --- |
| `--block <BLOCK>` | Đọc một phần của page bị cắt (id trong `unknown_block_ids`) |
| `-o`, `--output <FILE>` | Ghi Markdown ra file thay vì stdout |

JSON (page): `id`, `title`, `icon`, `url`, `parent`, `properties` (dòng database), `markdown`, `truncated`,
`unknown_block_ids`. JSON (database): `id`, `title`, `url`, `properties[]`, `views[]`.

### notes query

```text
ttm notes query <DATABASE> (--sql <SQL> | --filter <FILE>) [--page-size <N>] [--cursor <CURSOR>]
```

| Cờ | Việc |
| --- | --- |
| `--sql <SQL>` | SQL chỉ đọc trên bảng `rows` |
| `--filter <FILE>` | File JSON `{"filter": …, "sorts": …}` hoặc chỉ bộ lọc; `-` đọc stdin |
| `--page-size <N>` | 1–100 dòng (server mặc định 100); ngoài khoảng đó `ttm` dừng trước khi gọi server: `invalid value '500' for '--page-size <N>': 500 is out of range: --page-size takes 1 to 100 rows per page` |
| `--cursor <CURSOR>` | `next_cursor` của trang trước |

Phải có đúng một trong `--sql`, `--filter`. Mỗi dòng in `<id>  <values JSON>`; còn trang sau thì stderr in
`more rows: --cursor <CURSOR>`. JSON: `columns`, `rows[{id, values}]`, `next_cursor`.

### notes comments

```text
ttm notes comments <PAGE> [--all]
```

Thread comment đang mở; `--all` gồm cả thread đã đóng. JSON: mảng thread (`id`, `anchor`, `quote`, `resolved_at`,
`comments[]`).

### notes create

```text
ttm notes create (--parent <PAGE> | --database <DATABASE> | --workspace <WORKSPACE>) -f <FILE> [--title <TITLE>] [--icon <ICON>]
```

| Cờ | Việc |
| --- | --- |
| `--parent <PAGE>` | Tạo dưới page này |
| `--database <DATABASE>` | Tạo thành dòng của database |
| `--workspace <WORKSPACE>` | Tạo ở cấp cao nhất của workspace (id) |
| `-f`, `--file <FILE>` | File Markdown (bắt buộc); `-` đọc stdin |
| `--title <TITLE>` | Tiêu đề (mặc định: dòng `# ` đầu tiên, nếu không thì tên file) |
| `--icon <ICON>` | Emoji |

```text
✓ Created Ghi chú họp
  id 0190f3c4-…
  https://toantranmei.com/notes/0190f3c4-…
Next: ttm notes get 0190f3c4-…
```

JSON: `{id, title, url}`.

### notes edit

```text
ttm notes edit <PAGE> (--old <TEXT> --new <TEXT>)... [--replace-all] [--suggest] [--allow-deleting-content]
ttm notes edit <PAGE> --replace -f <FILE> [--suggest] [--allow-deleting-content]
```

| Cờ | Việc |
| --- | --- |
| `--old <TEXT>` | Chữ cần tìm (lặp lại được), phân biệt hoa thường, phải khớp đúng một chỗ |
| `--new <TEXT>` | Chữ thay cho `--old` cùng vị trí |
| `--replace-all` | Thay mọi chỗ khớp |
| `--replace` | Thay cả nội dung bằng file của `-f` (không dùng chung với `--old`/`--new`) |
| `-f`, `--file <FILE>` | File cho `--replace`; `-` đọc stdin |
| `--suggest` | Tạo đề xuất sửa thay vì ghi thẳng |
| `--allow-deleting-content` | Cho phép bỏ page con, database con, phần `<truncated/>` |

```text
Page updated.
warning: update_content[0]: old_str not found; skipped
```

Hoặc `Suggestion <id> created.`, hoặc `No change applied.` JSON: `applied`, `warnings`, `seq`, `snapshot_seq`,
`suggestion_id`.

### notes comment

```text
ttm notes comment <PAGE> -m <MESSAGE> [--quote <QUOTE> [--prefix <PREFIX>] [--suffix <SUFFIX>] | --thread <THREAD>]
```

| Cờ | Việc |
| --- | --- |
| `-m`, `--message <MESSAGE>` | Nội dung (bắt buộc) |
| `--quote <QUOTE>` | Đoạn trích để neo; bỏ trống = comment cho cả page |
| `--prefix <PREFIX>`, `--suffix <SUFFIX>` | Vài chữ ngay trước / sau đoạn trích, khi nó xuất hiện nhiều lần (cần `--quote`) |
| `--thread <THREAD>` | Trả lời trong thread này (không dùng chung với `--quote`) |

```text
Comment thread 0190f3e0-… created.
```

### notes rows upsert

```text
ttm notes rows upsert <DATABASE> -f <FILE>
```

File JSON `[{"id"?: "…", "values": {"<property>": …}}]`, 1–100 dòng; `-` đọc stdin.

```text
2 row(s) saved: 0190f3c7-…, 0190f3c5-…
```

JSON: `{"ids": [...]}` theo thứ tự đầu vào.

### notes rows delete

```text
ttm notes rows delete <DATABASE> <IDS>...
```

Chuyển các dòng vào thùng rác. `2 row(s) deleted.`

### notes suggestions

```text
ttm notes suggestions <PAGE> [--all]
```

Đề xuất sửa đang chờ (`--all`: cả `accepted`, `rejected`, `stale`), với id từng thay đổi và Markdown trước/sau.

### notes resolve

```text
ttm notes resolve <SUGGESTION> (--accept [--change <ID>]... | --reject)
```

| Cờ | Việc |
| --- | --- |
| `--accept` | Chấp nhận |
| `--reject` | Từ chối |
| `--change <ID>` | Chỉ chấp nhận các thay đổi này (lặp lại được, cần `--accept`) |

```text
Suggestion 0190f3d1-…: accepted
Stale (anchor text gone): c3
```

JSON: `id`, `status`, `stale_change_ids`.

## project

### project init

```text
ttm project init [--name <NAME>] [--key <KEY>] [--workspace <ID>] [--repo <URL>] [--workflow <W>]
                 [--agents create|reuse|none] [--runner this|none] [--yes]
```

Tạo project trên server cho repo hiện tại rồi liên kết (như `link`). Cần `factory:write`. Trong terminal là **wizard**: tên
→ key (gợi ý từ tên, `^[A-Z][A-Z0-9]{1,9}$`) → workspace → repo → workflow → agent mẫu (xem trước từ
`GET /factory/agent-presets`; agent cùng vai trò đã có thì hỏi dùng lại hay tạo mới) → runner → xác nhận. Mọi câu hỏi có
cờ; `--yes` lấy mặc định. Xong wizard, stderr có dòng `Equivalent: ttm project init … --yes`. Tương thích 0.1.6: không
phải terminal, có `--key` mà không có `--workflow`/`--agents`/`--runner` thì tạo ngay như cũ, không hỏi. Workflow, agent
mẫu cần tính năng `workflow_v2`.

| Cờ | Mặc định | Việc |
| --- | --- | --- |
| `--name <NAME>` | tên thư mục gốc của repo | Tên project |
| `--key <KEY>` | gợi ý từ tên | Tiền tố key task (2–10 chữ hoa/số, bắt đầu bằng chữ) |
| `--workspace <ID>` | workspace hiện tại | |
| `--repo <URL>` (cũ: `--repo-url`) | `git remote get-url origin` | |
| `--workflow <W>` | `balanced` | `professional` (9 trạng thái, 6 agent), `balanced` (5 trạng thái, 3 agent, ít token hơn khoảng 40%), `kanban`, `scrum`, `autopilot` hoặc slug mẫu |
| `--agents <A>` | `create` | `create` tạo agent mẫu, `reuse` dùng lại agent cùng vai trò, `none` không agent |
| `--runner <R>` | `this` | `this`: ghi `[repos] KEY = <thư mục này>` vào `runner.toml`; `none` |

```text
OK: Created project WEB (Web todo)
  linked /home/an/web-todo · plan rev 1 · workflow balanced
AGENT              CODE   STATUS
[agent] DEV · Code  dev    created
[agent] QA · Test   qa     created
[agent] Lead        lead   reused
Next: ttm runner up
```

JSON: nội dung `project.toml` (+ `agents[{code, agent_id, created}]` khi có).

### project link

```text
ttm project link <PROJECT>
```

Liên kết repo với project có sẵn (key hoặc id), ghi `.ttm/project.toml`, `.ttm/plan`, `.ttm/base`, `.ttm/.gitignore`.

```text
✓ Linked /Users/an/code/aurora to AUR (Aurora)
  plan rev 42
  Commit .ttm/project.toml and .ttm/plan.
Next: ttm rules apply
```

### project open

```text
ttm project open
```

Mở `<host>/factory?project=<id>`. JSON: `{"url": "…"}`.

### project status

```text
ttm project status
```

```text
project: AUR (0190f400-…)
host:    https://toantranmei.com
rev:     42
pending change request: 0190f4a1-…
```

## sync

```text
ttm sync [--check | --resolve]
```

Kéo kế hoạch về `.ttm/plan`, gửi sửa local thành change request. Xem [Factory](factory.md).

| Cờ | Việc |
| --- | --- |
| `--check` | Chỉ báo local và server có lệch không; **thoát 1 khi lệch**. Không ghi gì. |
| `--resolve` | Khi xung đột, để agent local (`claude`) hợp nhất trong worktree riêng và gửi change request mới |

JSON: `local_rev`, `server_rev`, `local_changes`, `server_changes`, `change_request`, `skipped`, `nothing_to_send`,
`conflicts[{task, field, base, local, server}]`, `in_sync`.

## task

Chạy trong repo đã liên kết (trừ `show`, `claim`, `done`, `approve`, `split` vốn nhận key bất kỳ).

| Lệnh | Cờ | Việc | Scope |
| --- | --- | --- | --- |
| `ttm task list` | `--state <STATE>` (id trạng thái hoặc loại), `--mine` | Task của project | `factory:read` |
| `ttm task show <KEY>` | | Tiêu đề, trạng thái, vai trò, người làm, mô tả | `factory:read` |
| `ttm task claim <KEY>` | | Giao task cho chính bạn | `factory:write` |
| `ttm task done <KEY>` | | Chuyển sang `done` (guard, cổng duyệt vẫn áp) | `factory:write` |
| `ttm task approve <KEY>` | `--reject` | Duyệt / từ chối bước đang chờ | `factory:write` |
| `ttm task split <KEY> -f <FILE>` | `-f`, `--file` (bắt buộc; `-` = stdin) | Tạo task con từ JSON (khoá camelCase hoặc snake_case, xem [Factory](factory.md)) | `factory:write` |

```text
AUR · 2 task(s)
   KEY     TITLE                          ROLE  ASSIGNEE            EST
────────────────────────────────────────────────────────────────────────
●  AUR-12  Đăng nhập bằng Google          DEV   🤖 Dev bot ✦ Mei-er   3
◐  AUR-13  Trang hồ sơ                    QA    👤 Lan                2
Next: ttm task show AUR-12
```

```text
╭─ AUR-12 · Đăng nhập bằng Google ─╮
│ ● doing                          │
╰──────────────────────────────────╯
   status  doing (in-progress)
     role  DEV
 assignee  🤖 Dev bot
delegator  👤 An ✦ Mei-er

Acceptance criteria
  ✓ Google button shows
  ○ Email is verified

Cho phép đăng nhập bằng tài khoản Google.

Next: ttm agent run AUR-12
```

Đầu ra khác: `✓ AUR-12 is now yours`, `✓ AUR-12 is now done`, `✓ AUR-12 approved`, `✓ AUR-12 rejected` (`--reject`),
`✓ 2 subtask(s) created under AUR-12` JSON: đối tượng task (camelCase, `id` là key), hoặc `{tasks, warnings}` cho `split`.

## rules

### rules apply

```text
ttm rules apply
```

Ghi khối `<!-- ttm:begin -->…<!-- ttm:end -->` vào `AGENTS.md` và `CLAUDE.md`, gộp hook vào `.claude/settings.json`, lưu
`.ttm/cache/rules.json`. In các file đã đổi hoặc `✓ Rules already up to date` JSON: `changed`, `checks`.

## hook

Do Claude Code gọi qua hook (cài bởi `rules apply`); đọc JSON của hook trên stdin. **Thoát 2** khi chặn, kèm lý do trên
stderr; thoát 0 khi cho phép hoặc ngoài repo đã liên kết.

| Lệnh | Hook | Kiểm |
| --- | --- | --- |
| `ttm hook pre-bash` | `PreToolUse` (matcher `Bash`) | `no_push_main` (nhánh được bảo vệ theo luật đã lưu, xem [Factory](factory.md)), `no_verify`, `conventional_commits` trên `tool_input.command` |
| `ttm hook stop` | `Stop` | `tests_before_done`: lệnh test có trong `transcript_path` (bỏ qua khi `stop_hook_active`) |

## runner

| Lệnh | Việc |
| --- | --- |
| `ttm runner up` | Chạy runner ở foreground (`Ctrl-C` để dừng); đăng ký máy nếu cần, dọn worktree cũ |
| `ttm runner down` | Dừng tiến trình trong `runner.pid` |
| `ttm runner status` | Id, tên, đang chạy, sức chứa, harness, repo |
| `ttm runner config [cờ]` | Sửa `runner.toml` |
| `ttm runner install-service` | Cài dịch vụ chạy khi đăng nhập (launchd, systemd `--user`, Task Scheduler), mang theo `PATH` của shell đang chạy lệnh |
| `ttm runner uninstall-service` | Dừng và gỡ dịch vụ (plist, unit hoặc tác vụ Task Scheduler) |

Cờ của `runner config`:

| Cờ | Mặc định | Việc |
| --- | --- | --- |
| `--max-parallel <N>` | từ CPU và RAM | Số agent cùng lúc |
| `--accept-team-jobs <true\|false>` | `false` | Nhận job cho task của đồng đội |
| `--worktree-root <DIR>` | `~/.ttm/worktrees` | Nơi tạo worktree |
| `--keep-days <N>` | `3` | Ngày giữ worktree đã xong |
| `--repo <KEY=PATH>` | | Clone local của project (lặp lại được) |
| `--name <NAME>` | tên máy | Tên hiện trên web |
| `--auto-merge <true\|false>` | `false` | Tự merge đầu merge queue và push |

### runner config mcp

MCP local của máy này (bảng `[mcp.local.<name>]` trong `runner.toml`, xem [Runner](runner.md)). File được sửa giữ nguyên
comment. Server chỉ được báo tên, `allow` và `team_jobs`, không bao giờ lệnh hay biến môi trường.

| Lệnh | Cờ | Việc |
| --- | --- | --- |
| `ttm runner config mcp add <NAME> --command <CMD>` | `--arg <ARG>` (lặp), `--env K=env:VAR` hoặc `K=<giá trị không bí mật>` (lặp) | Thêm/thay một MCP local. Tên `^[a-z0-9][a-z0-9-]{0,31}$`, khác `ttm`. Giá trị trông như token bị từ chối: dùng `env:VAR` |
| `ttm runner config mcp allow <AGENT>` | `--mcp <NAME>` (lặp; mặc định mọi MCP local) | Cho agent (handle) dùng |
| `ttm runner config mcp deny <AGENT>` | `--mcp <NAME>` (lặp) | Bỏ quyền |
| `ttm runner config mcp team-jobs <NAME> on\|off` | | Cho job của người khác giao dùng MCP này (mặc định `off`) |
| `ttm runner config mcp remove <NAME>` | | Xoá |
| `ttm runner config mcp list` | | Bảng tên, lệnh, `allow`, team jobs + dòng "Announced to the server: names and allowed agents only" |

```sh
ttm runner config mcp add github-local --command npx --arg -y --arg @modelcontextprotocol/server-github --env GITHUB_TOKEN=env:GITHUB_TOKEN
ttm runner config mcp allow dev --mcp github-local
```

### runner config claude-token

```text
printf %s "$TOKEN" | ttm runner config claude-token
ttm runner config claude-token --remove
```

Lưu (vào keyring) token OAuth Claude Code từ `claude setup-token`, đọc trên stdin, dùng cho job của đồng đội trong thư mục
cấu hình Claude riêng mỗi run. Có token thì runner báo tính năng `team_isolation`. Xem [Runner](runner.md).

`ttm runner status` có thêm `features` và bảng MCP local (tên, allow, team jobs).

JSON của `status`: `runner_id`, `name`, `running`, `pid`, `capacity`, `accept_team_jobs`, `harnesses`, `repos`,
`worktree_root`, `features`, `local_mcp`. Của `config`: nội dung `runner.toml`. Của `down`: `stopped`, `pid`. Của `install-service`: `installed`, `path` (`PATH` của dịch vụ; `null` trên Windows). Của
`uninstall-service`: `removed` (`null` khi chưa cài).

## agent

| Lệnh | Cờ | Việc |
| --- | --- | --- |
| `ttm agent run <KEY>` | | Chạy task ngay trên máy này, in sự kiện; thoát 0 chỉ khi `succeeded` |
| `ttm agent logs <RUN>` | `-f`, `--follow` | Sự kiện của run; `-f` hỏi lại mỗi 2 giây |
| `ttm agent stop <RUN>` | | Kill switch |

JSON của `run`: `run_id`, `status`, `branch`, `worktree`, `pr_url`, `paused_until`. `logs --json` in mỗi sự kiện một dòng
JSON (`id`, `at`, `kind`, `message`, `data`). `logs` dạng người đọc: `10:00:02 › edited src/login.rs` (glyph theo `kind`:
`·` thought, `›` action, `◆` mcp_call, `✕` error, `NN%` progress).

### Quản trị agent

Cần tính năng `agent_v2`; mọi lệnh có `--workspace <ID>` (mặc định workspace hiện tại). `<AGENT>` là handle (`dev`,
`@dev`), tên hoặc id. Đọc: `factory:read`; sửa: `factory:admin` (owner/admin của workspace).

| Lệnh | Việc |
| --- | --- |
| `ttm agent list` | Bảng: 🤖 tên + huy hiệu chủ workspace, `@handle`, vai trò, model, skill (`◐` = có bản chờ duyệt), MCP, trạng thái |
| `ttm agent show <AGENT>` | Header + danh tính git, hồ sơ, skill, MCP, dự án |
| `ttm agent create [--preset <CODE>] [trường]` | Wizard trong terminal (preset hoặc từng câu); `--yes` + cờ cho script |
| `ttm agent edit <AGENT> [trường] [--git-email <E>] [--git-name <N>] [--bio <TEXT>]` | Chỉ qua cờ; `""` = về mặc định |
| `ttm agent pause\|resume\|archive <AGENT>` | Có xác nhận (`--yes`) |
| `ttm agent skills add <AGENT> <SLUG>[@<V>]` / `skills rm <AGENT> <SLUG>` | Gắn (ghim phiên bản với `@V`) / gỡ skill |
| `ttm agent mcp add <AGENT> <SERVER\|local:NAME> [--tools a,b\|*] [--required]` / `mcp rm <AGENT> <SERVER>` | Cấp / bỏ MCP; `--required` = chỉ chạy trên runner cấp được MCP local đó |

Trường của `create`/`edit`: `--name`, `--handle`, `--role`, `--model` (`opus`, `sonnet`, `haiku` hoặc id), `--budget`
(token mỗi run), `--daily-budget`, `--turns`, `--permission` (`read`, `edit`, `full`, `custom`), `--skills a,b`,
`--instructions-file <FILE>` (`-` = stdin).

```text
3 agent(s)
   NAME              HANDLE  ROLE  MODEL   SKILLS        MCP  STATUS
🤖 DEV · Code ✦ Mei-er @dev  DEV   sonnet  ttm-tdd ◐     1    active
```

## workflow

Cần `workflow_v2` (`validate --file` cần `ascode_v2`). `--project <KEY>` mặc định là project đã liên kết. Sửa cần
`factory:admin`.

| Lệnh | Việc |
| --- | --- |
| `ttm workflow show [--version <N\|draft>]` | Header (tên · vN · draft?), bảng trạng thái (glyph theo category, tên, vai trò, tự giao 🤖/👤, WIP), bảng chuyển (từ → tới, guard, ⚡ số luật) |
| `ttm workflow validate [--file .ttm/workflow.yaml]` | Có `--file`: kiểm file as-code ở server; không: kiểm draft. Mỗi lỗi một dòng `✕ <rule\|code> <field>: <message>`; **thoát 1 khi có lỗi** |
| `ttm workflow simulate <TASK> --to <STATUS> [--version <V>] [--roles a,b]` | Từng bước ✓/✕ + lý do; thoát 1 khi bị chặn |
| `ttm workflow diff [<FROM> <TO>]` | Mặc định phiên bản của project và draft; `+`/`-`/`~` theo trạng thái, chuyển, luật |
| `ttm workflow publish [--note <TEXT>]` | Hiện diff → xác nhận → phát hành draft thành phiên bản mới (`note` ≤ 280) |
| `ttm workflow upgrade [--to <N>]` | Hiện preview (bảng task được chuyển trạng thái) → xác nhận → nâng project |
| `ttm workflow copy [--name <NAME>]` | Chép thành workflow sửa được của workspace |

```text
OK: AUR-12 can move to qc
  ok  doing -> qc  (transition t3, 1 rule)
```

## automation

Luật Tự động hoá của project (cần `workflow_v2`; `--project <KEY>`). `enable`/`disable` cần `factory:admin`.

| Lệnh | Việc |
| --- | --- |
| `ttm automation list` | Bảng: `●`/`⏸` bật/tắt, tên, trigger tóm tắt, số lượt 7 ngày |
| `ttm automation show <RULE>` | Khi / Nếu / Thì dạng cây |
| `ttm automation test <RULE> --task <KEY>` | Chạy thử (không đổi gì): mỗi hành động ✓ sẽ làm / ✕ bỏ qua + lý do |
| `ttm automation enable\|disable <RULE>` | Bật / tắt |
| `ttm automation runs [<RULE>] [--limit 20]` | Lượt chạy gần đây |

`<RULE>`: id, slug hoặc tên.

## skill

Skill của workspace (cần `agent_v2`; `--workspace <ID>`). Xem [Runner](runner.md) cho cách runner cài skill.

| Lệnh | Việc |
| --- | --- |
| `ttm skill list [--tag <T>] [--builtin]` | Bảng skill (`--builtin` kèm skill dựng sẵn) |
| `ttm skill push <DIR> [--slug <S>] [--note <TEXT>]` | Kiểm cục bộ (SKILL.md + front matter, ≤ 5 MB, ≤ 200 file, đường dẫn an toàn), hiện manifest + sha256, tạo skill nếu chưa có (`--slug` mặc định tên thư mục), tải zip tất định lên → `v<N> pending review` hoặc `v<N> ready`. Không đổi gì → `No changes` |
| `ttm skill pull <SLUG> [--version <V>] [--dir <DIR>]` | Tải, kiểm sha, giải nén (mặc định `.ttm/skills/<slug>` trong repo đã liên kết, ngược lại `./<slug>`) |
| `ttm skill show <SLUG> [--version <V>]` | Phiên bản và trạng thái duyệt |
| `ttm skill review list` | Hàng chờ duyệt |
| `ttm skill review approve <SLUG>@<V> [--note <TEXT>]` | Hiện diff với bản hiện hành → xác nhận → duyệt |
| `ttm skill review reject <SLUG>@<V> --note <TEXT>` | Từ chối (cần ghi chú) |
| `ttm skill import --git <URL> --path <P> [--ref main] [--slug <S>]` | Nhập từ GitHub/GitLab |

```text
OK: ttm-tdd v3 pending review
  sha256 3c9e7159… · 3 file(s) · 1.2 KB
Next: ttm skill review approve ttm-tdd@3
```

## mcp-server

MCP từ xa của workspace, agent dùng qua gateway của server (cần `agent_v2`; `--workspace <ID>`).

| Lệnh | Việc |
| --- | --- |
| `ttm mcp-server list` | Bảng: tên, URL (cắt), auth, trạng thái, số tool, đã kết nối (workspace/me) |
| `ttm mcp-server add [--name] [--url] [--transport http\|sse] [--auth none\|workspace\|delegator] [--secret-header <H>] [--secret-env <VAR>]` | Wizard; secret chỉ qua `--secret-env VAR` hoặc nhập ẩn trong terminal, không bao giờ là giá trị cờ |
| `ttm mcp-server connect <NAME> [--for workspace\|me] [--no-browser]` | Mở trang OAuth (hoặc chỉ in URL), chờ tới khi kết nối (hỏi lại mỗi 2 giây, tối đa 5 phút) |
| `ttm mcp-server remove <NAME>` | Xác nhận, cảnh báo agent đang dùng |

## cr

Change request: kế hoạch (từ `ttm sync`) và cấu hình (workflow, luật, agent, skill; cần `ascode_v2`).

| Lệnh | Cờ | Việc |
| --- | --- | --- |
| `ttm cr list` | `--project <KEY>`, `--kind plan\|workflow\|rule\|agent\|skill`, `--status pending\|applied\|rejected\|all` (mặc định `pending`) | Bảng: id ngắn, loại, trạng thái, tác giả + huy hiệu, số file, tuổi |
| `ttm cr show <ID>` | | Header + diff từng file |
| `ttm cr approve <ID>` | | Xác nhận rồi áp dụng (409 `stale` khi server đã đổi) |
| `ttm cr reject <ID>` | `--note <TEXT>` | Từ chối |

## inbox

```text
ttm inbox [--unread] [--kind factory,…] [--watch] [--interval 5]
ttm inbox read [<ID>… | --all]
```

Thông báo của bạn: bảng `●` chưa đọc, loại/lý do, nội dung (cắt), tuổi, link. `--watch` in danh sách rồi hỏi lại mỗi
`--interval` giây (tối thiểu 2), mỗi thông báo mới một dòng; `Ctrl-C` thoát êm; với `--json` in NDJSON.

```text
● 14:02 skill_review  Skill ttm-tdd v3 waiting for review → https://toantranmei.com/factory/skills/…
```

## template

Thư viện mẫu công khai (cần `templates`).

| Lệnh | Việc |
| --- | --- |
| `ttm template list [--kind workflow\|agent\|skill\|rule-pack\|bundle] [--query <Q>]` | Tìm mẫu |
| `ttm template publish --from workflow\|agent\|skill <SOURCE> [--slug] [--name] [--price <VND>] [--workspace]` | Xem trước khi xuất (hiện mục bị loại) → xác nhận → phát hành. Giá đặt trên web: lệnh in link `/me/items/<id>` |
| `ttm template install <SLUG> [--workspace <ID>]` | Preview (sẽ tạo: trạng thái, luật, agent, skill + cờ "runs code") → xác nhận → cài; có skill chờ duyệt thì `Next: ttm skill review list` |

## mcp

### mcp install

```text
ttm mcp install --client <CLIENT> [--name <NAME>] [--command <PATH>] [--toolsets <TOOLSETS>] [--dry-run]
```

| Cờ | Mặc định | Việc |
| --- | --- | --- |
| `--client <CLIENT>` | (bắt buộc) | `claude-code`, `claude-desktop`, `cursor`, `codex` |
| `--name <NAME>` | `ttm` | Tên máy chủ trong cấu hình client |
| `--command <PATH>` | `ttm-mcp` cạnh `ttm`, rồi trên `PATH` | Lệnh client chạy |
| `--toolsets <TOOLSETS>` | | Truyền `--toolsets` cho `ttm-mcp` |
| `--dry-run` | | Chỉ in, không ghi |

`--host`/`--profile` truyền tường minh (hoặc `TTM_HOST`) được ghim vào `args`. File cấu hình được gộp và sao lưu
(`<file>.bak-<giây unix>`). JSON: `client`, `file`, `backup`, `changed`, `dry_run`, `entry{name, command, args}`. Chi tiết:
[Kết nối MCP](mcp.md).

## self-update

```text
ttm self-update [--check] [--version <VERSION>]
```

| Cờ | Việc |
| --- | --- |
| `--check` | Chỉ báo có bản mới |
| `--version <VERSION>` | Cài đúng phiên bản này (ví dụ `0.2.0`), kể cả cũ hơn |

Tải từ GitHub Releases, kiểm SHA-256 và (khi có `gh`) attestation, thay `ttm` và `ttm-mcp` cùng thư mục. JSON: `current`,
`latest`, `update_available`, `updated`, `attestation`.

## completion

```text
ttm completion <SHELL>
```

`<SHELL>`: `bash`, `elvish`, `fish`, `powershell`, `zsh`. In script ra stdout; không đọc cấu hình. Xem
[Cài đặt](cai-dat.md).

## ttm-mcp

```text
ttm-mcp [--host <URL>] [--profile <NAME>] [--toolsets <LIST>] [-v...]
```

Máy chủ MCP qua stdio. Xem [Kết nối MCP](mcp.md) và [Tool MCP](mcp-cong-cu.md).
