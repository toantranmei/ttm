---
title: Tham chiếu lệnh
description: Mọi lệnh và lệnh con của ttm với cú pháp, cờ, giá trị mặc định, ví dụ, đầu ra JSON và mã thoát.
order: 10
section: reference
---

```text
ttm [OPTIONS] <COMMAND>
```

`ttm --help` và `ttm <lệnh> --help` in trợ giúp gốc (tiếng Anh). Trang này theo phiên bản **0.1.3**.

## Cờ chung

Dùng được với mọi lệnh, đặt trước hay sau lệnh con đều được (`ttm whoami --json` = `ttm --json whoami`).

| Cờ | Biến | Việc |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | Máy chủ, ví dụ `http://localhost:3000` |
| `--profile <NAME>` | `TTM_PROFILE` | Profile trong `config.toml` |
| `--json` | | Kết quả dạng JSON trên stdout |
| `-v`, `--verbose` | `TTM_LOG` | Thêm log ra stderr (`-v` info, `-vv` debug) |
| `-h`, `--help` | | Trợ giúp |
| `-V`, `--version` | | Phiên bản (`ttm 0.1.3`; chỉ ở cấp gốc) |

## Mã thoát

| Mã | Khi nào |
| --- | --- |
| `0` | Thành công |
| `1` | Lỗi (mạng, xác thực, API, file…); `ttm doctor` có mục `FAIL`; `ttm sync --check` thấy lệch; `ttm sync` gặp xung đột; `ttm agent run` kết thúc khác `succeeded` |
| `2` | Sai cú pháp dòng lệnh (cờ lạ, thiếu tham số). Với `ttm hook …`: hành động bị luật chặn |

Lỗi luôn in `error: <thông báo>` ra stderr; với `--json`, thêm `{"error": {"code", "message"}}` ra stdout (xem
[Cấu hình](cau-hinh.md) và [Mã lỗi](ma-loi.md)).

## Tóm tắt

| Lệnh | Việc |
| --- | --- |
| [`login`](#login) | Đăng nhập (trình duyệt, `--device`, `--token`) |
| [`logout`](#logout) | Thu hồi token và xoá khỏi máy |
| [`whoami`](#whoami) | Người dùng đang đăng nhập |
| [`status`](#status) | Host, profile, token, workspace đang dùng |
| [`doctor`](#doctor) | Kiểm tra môi trường |
| [`workspace`](#workspace) | `list`, `switch`, `current` |
| [`token`](#token) | `list`, `create`, `revoke` (mở trang web) |
| [`notes`](#notes) | `search`, `get`, `query`, `comments`, `create`, `edit`, `comment`, `rows`, `suggestions`, `resolve` |
| [`project`](#project) | `init`, `link`, `open`, `status` |
| [`sync`](#sync) | Đồng bộ kế hoạch |
| [`task`](#task) | `list`, `show`, `claim`, `done`, `approve`, `split` |
| [`rules`](#rules) | `apply` |
| [`hook`](#hook) | `pre-bash`, `stop` (do agent gọi) |
| [`runner`](#runner) | `up`, `down`, `status`, `config`, `install-service`, `uninstall-service` |
| [`agent`](#agent) | `run`, `logs`, `stop` |
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
| `--scope <SCOPE>` | `workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run offline_access` | Scope xin cấp, cách nhau bằng dấu cách (không áp cho `--token`) |
| `--no-browser` | | Không mở trình duyệt, chỉ in URL |

```sh
ttm login
ttm login --device
printf %s "$PAT" | ttm login --token -
ttm --profile dev login --host http://localhost:3000 --scope "workspace:read notee:read"
```

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

JSON: `host`, `profile`, `method` (`browser` | `device` | `token`), `stored_in` (`keyring` | `file` | `env`), `user`
(hoặc `null` khi token thiếu `workspace:read`).

## logout

```text
ttm logout
```

Thu hồi token truy cập và refresh token trên server, rồi xoá khỏi keyring và `credentials.toml`. Vẫn xoá cục bộ khi server
không xác nhận. Không bỏ được `TTM_TOKEN` (chỉ cảnh báo).

```text
Signed out of https://toantranmei.com (profile `default`).
```

JSON: `host`, `profile`, `revoked` (server xác nhận), `removed` (có token bị xoá khỏi máy).

## whoami

```text
ttm whoami
```

Gọi `/me` (cần `workspace:read`).

```text
An Nguyễn <an@example.com>
host: https://toantranmei.com  profile: default
workspace: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

JSON: `host`, `profile`, `user` (`id`, `email`, `name`, `avatar_url`, `locale`, `plan`), `workspace` (`id`, `name`,
`kind`, `role`, `slug` hoặc `null`).

## status

```text
ttm status
```

Những gì máy này sẽ dùng; không cần đăng nhập hay mạng (luôn thoát 0).

```text
ttm 0.1.3
host:      https://toantranmei.com
profile:   default (/Users/an/.config/ttm/config.toml)
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
workspace: not chosen (ttm workspace switch)
server:    API v1 (min ttm 0.1.0), features: notee, factory
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
[ok  ] host       https://toantranmei.com (profile default)
[ok  ] discovery  api_base https://toantranmei.com/api/v1
[ok  ] version    ttm 0.1.3, server API v1
[ok  ] token      ttm_oat_Ab12… (keyring) as an@example.com
[ok  ] keyring    OS keyring available
[ok  ] git        git version 2.50.1
[warn] claude     Claude Code not found on PATH (needed by ttm runner)
[ok  ] ttm-mcp    /Users/an/.cargo/bin/ttm-mcp
```

JSON: mảng `{name, level, detail}`.

## workspace

### workspace list

```text
ttm workspace list
```

Các workspace bạn là thành viên; `*` đánh dấu workspace hiện tại của profile.

```text
* Cá nhân  personal · owner · 0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
  Team Aurora [aurora]  team · member · 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

JSON: mảng `{id, name, kind, role, slug?, current}`.

### workspace switch

```text
ttm workspace switch <WORKSPACE>
```

Đặt workspace hiện tại cho profile. `<WORKSPACE>`: id, slug hoặc tên (khớp chính xác, không phân biệt hoa thường), hoặc
phần đầu tên nếu chỉ khớp một workspace. Khớp nhiều: `matches several workspaces (…); use the id`.

```text
Current workspace for profile `default`: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

### workspace current

```text
ttm workspace current
```

In tên và id (tên chỉ khi server trả lời được). JSON: `profile`, `workspace_id`, `workspace`.

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
Created Ghi chú họp (https://toantranmei.com/notes/0190f3c4-…)
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
ttm project init --key <KEY> [--name <NAME>] [--repo-url <URL>] [--workspace <ID>]
```

Tạo project trên server cho repo hiện tại rồi liên kết (như `link`). Cần `factory:write`.

| Cờ | Mặc định |
| --- | --- |
| `--key <KEY>` | (bắt buộc) tiền tố key task, được viết hoa |
| `--name <NAME>` | tên thư mục gốc của repo |
| `--repo-url <URL>` | `git remote get-url origin` |
| `--workspace <ID>` | workspace hiện tại |

JSON: nội dung `project.toml`.

### project link

```text
ttm project link <PROJECT>
```

Liên kết repo với project có sẵn (key hoặc id), ghi `.ttm/project.toml`, `.ttm/plan`, `.ttm/base`, `.ttm/.gitignore`.

```text
Linked /Users/an/code/aurora to AUR (Aurora), plan rev 42.
Commit .ttm/project.toml and .ttm/plan; run `ttm rules apply` to install the project rules.
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
AUR-12     doing          DEV       Dev bot          Đăng nhập bằng Google
```

```text
AUR-12  Đăng nhập bằng Google
state: doing  role: DEV  assignee: Dev bot

Cho phép đăng nhập bằng tài khoản Google.
```

Đầu ra khác: `AUR-12 is now yours.`, `AUR-12 is now done.`, `AUR-12 approved; state done.`, `AUR-12 rejected; state doing.` (`--reject`),
`2 subtask(s) created under AUR-12.` JSON: đối tượng task (camelCase, `id` là key), hoặc `{tasks, warnings}` cho `split`.

## rules

### rules apply

```text
ttm rules apply
```

Ghi khối `<!-- ttm:begin -->…<!-- ttm:end -->` vào `AGENTS.md` và `CLAUDE.md`, gộp hook vào `.claude/settings.json`, lưu
`.ttm/cache/rules.json`. In các file đã đổi hoặc `Rules already up to date.` JSON: `changed`, `checks`.

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

JSON của `status`: `runner_id`, `name`, `running`, `pid`, `capacity`, `accept_team_jobs`, `harnesses`, `repos`,
`worktree_root`. Của `config`: nội dung `runner.toml`. Của `down`: `stopped`, `pid`. Của `install-service`: `installed`, `path` (`PATH` của dịch vụ; `null` trên Windows). Của
`uninstall-service`: `removed` (`null` khi chưa cài).

## agent

| Lệnh | Cờ | Việc |
| --- | --- | --- |
| `ttm agent run <KEY>` | | Chạy task ngay trên máy này, in sự kiện; thoát 0 chỉ khi `succeeded` |
| `ttm agent logs <RUN>` | `-f`, `--follow` | Sự kiện của run; `-f` hỏi lại mỗi 2 giây |
| `ttm agent stop <RUN>` | | Kill switch |

JSON của `run`: `run_id`, `status`, `branch`, `worktree`, `pr_url`, `paused_until`. `logs --json` in mỗi sự kiện một dòng
JSON (`id`, `at`, `kind`, `message`, `data`).

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
