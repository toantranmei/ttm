---
title: Cấu hình
description: config.toml và profile, thứ tự chọn host, vị trí file trên từng hệ điều hành, đầu ra JSON, log và mạng.
order: 5
section: guides
---

`ttm` chạy được mà không cần cấu hình gì: không có file thì dùng profile `default` và host `https://toantranmei.com`. Các
file dưới đây được tạo khi bạn đăng nhập, đổi workspace hay cấu hình runner.

## Vị trí file

| Thư mục | macOS, Linux | Windows | Ghi đè bằng |
| --- | --- | --- | --- |
| Cấu hình | `$XDG_CONFIG_HOME/ttm`, mặc định `~/.config/ttm` | `%APPDATA%\ttm` | `TTM_CONFIG_DIR` |
| Bộ đệm | `$XDG_CACHE_HOME/ttm`, mặc định `~/.cache/ttm` | `%LOCALAPPDATA%\ttm\cache` | `TTM_CACHE_DIR` |
| Worktree | `~/.ttm/worktrees` | `%USERPROFILE%\.ttm\worktrees` | `worktree_root` trong `config.toml` hoặc `runner.toml` |

> **Lưu ý:** trên macOS, ttm cũng dùng `~/.config/ttm` và `~/.cache/ttm` (không dùng `~/Library`).

| File | Ở đâu | Nội dung |
| --- | --- | --- |
| `config.toml` | thư mục cấu hình | Profile, host, workspace hiện tại, `worktree_root` |
| `credentials.toml` | thư mục cấu hình | Token khi keyring không dùng được (quyền 0600) |
| `runner.toml` | thư mục cấu hình | Cấu hình runner (xem [Runner](runner.md)) |
| `discovery-<host>.json` | bộ đệm | Discovery của host, dùng lại trong 1 giờ; vẫn dùng bản cũ khi host không trả lời |
| `deprecations` | bộ đệm | Cảnh báo endpoint sắp gỡ đã hiện trong ngày |
| `runner.pid`, `runner.log`, `runner-worktrees.json` | bộ đệm | Tiến trình, log (dịch vụ macOS) và sổ worktree của runner |
| `.ttm/project.toml` | trong repo | Liên kết repo với dự án Factory (xem [Factory](factory.md)) |

`ttm status` in đường dẫn `config.toml` đang dùng.

## config.toml

```toml
# Profile dùng khi không có --profile hay TTM_PROFILE.
default_profile = "work"

# Thư mục worktree cho `ttm sync --resolve` và runner (mặc định ~/.ttm/worktrees). `~/` được mở rộng.
worktree_root = "~/orca/workspaces"

[profiles.default]
host = "https://toantranmei.com"

[profiles.work]
host = "https://toantranmei.com"
workspace = "0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71"

[profiles.dev]
host = "http://localhost:3000"
```

| Khoá | Kiểu | Ghi bởi | Ý nghĩa |
| --- | --- | --- | --- |
| `default_profile` | chuỗi | bạn | Profile mặc định; không có thì `default` |
| `worktree_root` | chuỗi | bạn | Gốc thư mục worktree |
| `profiles.<tên>.host` | URL | `ttm login` | Máy chủ của profile |
| `profiles.<tên>.workspace` | id | `ttm workspace switch` | Workspace hiện tại của profile |

`ttm` ghi file này nguyên khối (ghi file tạm rồi đổi tên), nên chú thích bạn thêm vào sẽ mất ở lần ghi sau. File sai cú
pháp làm mọi lệnh (trừ `ttm completion`) dừng với `invalid TOML in <đường dẫn>: …`.

## Profile

Một profile là cặp (host, workspace) có tên. Token lưu theo cặp (profile, host).

Thứ tự chọn **profile**:

1. `--profile <tên>`
2. `TTM_PROFILE`
3. `default_profile` trong `config.toml`
4. `default`

Thứ tự chọn **host**:

1. `--host <URL>`
2. `TTM_HOST`
3. `host` của profile
4. `https://toantranmei.com` (giá trị lúc build, đổi được bằng `TTM_DEFAULT_HOST` khi build)

Host được chuẩn hoá: bỏ `/` cuối, viết thường tên miền, thêm `https://` nếu thiếu (riêng `localhost` và `127.0.0.1` thêm
`http://`). Chỉ nhận `http` và `https`; giá trị khác báo `invalid host`.

```sh
ttm --host toantranmei.com status        # → https://toantranmei.com
ttm --host localhost:3000 status         # → http://localhost:3000
```

Thứ tự chọn **workspace** cho lệnh cần nó (`notes search`, `notes create --workspace`, `project init`, tool MCP):
tham số của lệnh, rồi workspace của profile. `ttm-mcp` khi chưa chọn workspace dùng workspace cá nhân.

Xem hết những gì máy đang dùng:

```sh
ttm status
```

```text
ttm 0.1.2
host:      https://toantranmei.com
profile:   work (/Users/an/.config/ttm/config.toml)
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
workspace: 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
server:    API v1 (min ttm 0.1.0), features: notee, factory
```

`ttm status` không lỗi khi chưa đăng nhập hay mất mạng (`server: unreachable`).

## Biến môi trường

Các biến hay dùng nhất:

| Biến | Việc |
| --- | --- |
| `TTM_HOST` | Máy chủ (sau `--host`) |
| `TTM_PROFILE` | Profile (sau `--profile`) |
| `TTM_TOKEN` | Token, thắng keyring và file |
| `TTM_NO_KEYRING=1` | Không dùng keyring, chỉ dùng `credentials.toml` |
| `TTM_LOG` | Mức log (cú pháp `tracing`), ví dụ `debug` |

Danh sách đầy đủ (kể cả biến của `ttm-mcp` và runner): [Biến môi trường](bien-moi-truong.md).

## Đầu ra JSON

Mọi lệnh nhận `--json` (đặt ở đâu trên dòng lệnh cũng được): kết quả là JSON đẹp trên **stdout**, thông báo phụ vẫn đi
ra stderr.

```sh
ttm whoami --json
```

```json
{
  "host": "https://toantranmei.com",
  "profile": "default",
  "user": {
    "id": "0190f3a1-0000-7000-8000-000000000001",
    "email": "an@example.com",
    "name": "An Nguyễn",
    "avatar_url": null,
    "locale": "vi",
    "plan": "free"
  },
  "workspace": null
}
```

Khi lỗi, `--json` in thêm một đối tượng lỗi ra stdout (và dòng `error: …` ra stderr), mã thoát khác 0:

```json
{
  "error": {
    "code": "insufficient_scope",
    "message": "this token lacks the scope notee:write (insufficient_scope, HTTP 403) {\"required\":\"notee:write\"}"
  }
}
```

`code` là mã ổn định của server (xem [Mã lỗi](ma-loi.md)), mã OAuth (`access_denied`, `invalid_grant`…), hoặc mã cục bộ:
`network` (không kết nối được), `decode` (phản hồi lạ), `upgrade_required` (ttm quá cũ), `invalid_url`, `error` (lỗi khác
như chưa đăng nhập, file không đọc được).

Ví dụ dùng trong script:

```sh
ttm notes search "OKR" --json | jq -r '.[].id'
ttm notes query "$DB" --sql 'SELECT "Name" FROM rows' --json | jq '.rows[].values'
```

## Log

Log đi ra **stderr** (stdout của `ttm-mcp` dành cho giao thức MCP).

| Cờ | Mức |
| --- | --- |
| (không) | `warn`: cảnh báo, ví dụ endpoint sắp gỡ, server cần bản mới |
| `-v` | `info` (và `debug` cho các module của ttm) |
| `-vv` | `debug` |
| `TTM_LOG=<filter>` | Ghi đè, cú pháp `tracing_subscriber::EnvFilter`, ví dụ `TTM_LOG=ttm_api=debug,warn` |

Token không bao giờ xuất hiện trong log: mọi chỗ hiển thị token chỉ giữ tiền tố và tối đa 4 ký tự.

## Mạng

- Mỗi request có timeout **30 giây** và gửi `User-Agent: ttm/<phiên bản> (<os>; <arch>)`.
- Lỗi kết nối, timeout và lỗi 5xx của request đọc (GET, PUT, DELETE) được thử lại tối đa **3 lần** (chờ 0,5 s, 1 s, 2 s).
- `429` được thử lại tối đa 3 lần cho mọi loại request, chờ theo `Retry-After` (tối đa 60 giây mỗi lần).
- Endpoint có header `Deprecation` sinh một cảnh báo mỗi ngày, kèm ngày gỡ (`Sunset`) nếu có.
- Proxy: theo biến chuẩn `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`.
