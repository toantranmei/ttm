---
title: Biến môi trường
description: Mọi biến môi trường mà ttm, ttm-mcp và runner đọc, với giá trị mặc định và thứ tự ưu tiên.
order: 13
section: reference
---

Biến rỗng được coi như không đặt (trừ khi ghi khác).

## Kết nối và xác thực

| Biến | Đọc bởi | Mặc định | Ý nghĩa |
| --- | --- | --- | --- |
| `TTM_HOST` | `ttm`, `ttm-mcp` | host của profile, rồi `https://toantranmei.com` | Máy chủ. Thua `--host`, thắng profile. Khi đặt, `ttm login` ghi host này vào profile và `ttm mcp install` ghim `--host` vào cấu hình client. |
| `TTM_PROFILE` | `ttm`, `ttm-mcp` | `default_profile`, rồi `default` | Profile trong `config.toml`. Thua `--profile`. |
| `TTM_TOKEN` | `ttm`, `ttm-mcp` | (không) | Token dùng cho mọi request, thắng keyring và `credentials.toml`. Không tự gia hạn. Runner không dùng nó cho token riêng của runner. |
| `TTM_NO_KEYRING` | `ttm`, `ttm-mcp`, runner | (keyring bật) | Giá trị khác rỗng và khác `0`: không đọc, không ghi keyring; token vào `credentials.toml`. |
| `TTM_NO_BROWSER` | `ttm` | (mở trình duyệt) | Có giá trị: không mở trình duyệt (`ttm login`, `ttm token …`, `ttm project open`), chỉ in URL. |

## Thư mục

| Biến | Mặc định | Ý nghĩa |
| --- | --- | --- |
| `TTM_CONFIG_DIR` | `~/.config/ttm` (`%APPDATA%\ttm`) | Thư mục `config.toml`, `credentials.toml`, `runner.toml` |
| `TTM_CACHE_DIR` | `~/.cache/ttm` (`%LOCALAPPDATA%\ttm\cache`) | Bộ đệm discovery, pid và log runner, sổ worktree |
| `TTM_HOME` | thư mục người dùng của hệ điều hành | Thay thư mục nhà: mở rộng `~/`, `~/.ttm/worktrees`, vị trí cấu hình client của `ttm mcp install`, file dịch vụ runner. Dùng cho bản cài portable và test (trên Windows `HOME` bị bỏ qua). |
| `XDG_CONFIG_HOME` | `~/.config` | (macOS, Linux) gốc thư mục cấu hình; cũng là gốc cấu hình Claude Desktop trên Linux và file systemd của runner |
| `XDG_CACHE_HOME` | `~/.cache` | (macOS, Linux) gốc thư mục bộ đệm |
| `APPDATA`, `LOCALAPPDATA` | do Windows đặt | (Windows) thư mục cấu hình và bộ đệm; `APPDATA` cũng là nơi tìm cấu hình Claude Desktop |
| `CODEX_HOME` | `~/.codex` | Thư mục cấu hình Codex mà `ttm mcp install --client codex` sửa |

## Log

| Biến | Ý nghĩa |
| --- | --- |
| `TTM_LOG` | Bộ lọc log (`tracing_subscriber::EnvFilter`), ghi đè `-v`/`-vv`. Ví dụ `TTM_LOG=debug`, `TTM_LOG=ttm_api=debug,warn`. |

## Đầu ra

| Biến | Mặc định | Ý nghĩa |
| --- | --- | --- |
| `TTM_PLAIN` | (tắt) | Khác rỗng và khác `0`/`false`/`no`/`off`: như `--plain` (không màu, khung, glyph, emoji) |
| `TTM_COLUMNS` | (không) | Bề ngang đầu ra, thắng `COLUMNS` và kích thước terminal; kẹp 40–200 |
| `COLUMNS` | (không) | Bề ngang khi không có `TTM_COLUMNS` |
| `NO_COLOR` | (không) | Khác rỗng: không màu (thắng `CLICOLOR_FORCE`) |
| `CLICOLOR_FORCE` | (không) | `1`: có màu kể cả khi pipe |
| `COLORTERM` | (không) | `truecolor` hoặc `24bit`: dùng đúng mã màu brand |
| `WT_SESSION` | do Windows Terminal đặt | Như `COLORTERM=truecolor` |
| `TERM` | (không) | `dumb`: không màu; chứa `256color`: bảng 256 màu |

## ttm-mcp

| Biến | Mặc định | Ý nghĩa |
| --- | --- | --- |
| `TTM_TOOLSETS` | mọi nhóm | Như cờ `--toolsets`: nhóm tool bật thêm, cách nhau bằng dấu phẩy (`notee`, `factory`, `admin`; `workspace` luôn bật). Cờ thắng biến. Khi có `TTM_RUN_ID`, `admin` luôn bị bỏ (kèm cảnh báo). |
| `TTM_RUN_ID` | (không) | Id lần chạy agent. Bắt buộc cho `factory-report-progress`, `factory-request-approval`, `factory-attach-artifact`, `factory-renew-lease`; `factory-get-context` dùng context của run. |
| `TTM_TASK` | (không) | Key task mặc định (`AUR-12`) cho `factory-get-task`, `factory-get-context`, `factory-create-subtasks`. |
| `TTM_PROJECT` | (không) | Project mặc định (id hoặc key) cho `factory-get-rules`. |

Runner tự đặt `TTM_TOKEN` (token của job), `TTM_HOST`, `TTM_NO_KEYRING=1`, `TTM_RUN_ID`, `TTM_TASK`, `TTM_PROJECT` cho
`ttm-mcp` mà nó khởi động cho agent. `ttm hook` cũng đọc `TTM_TASK` và `TTM_RUN_ID` để gắn kết quả luật vào đúng run.

## Runner

| Biến | Ý nghĩa |
| --- | --- |
| `HOSTNAME`, `COMPUTERNAME` | Tên máy mặc định của runner (khi `runner.toml` không có `name`); không có thì chạy lệnh `hostname` |
| `PATH` (`PATHEXT` trên Windows) | Nơi tìm `claude`, `git`, `gh`, `ttm-mcp` |
| Biến được `env:VAR` trong `[mcp.local.*]` nhắc tới | Giá trị đưa cho MCP local lúc chạy; thiếu thì MCP đó bị bỏ |

Runner đặt cho agent và git của mỗi job: `GIT_AUTHOR_NAME/EMAIL`, `GIT_COMMITTER_NAME/EMAIL` (khi job có danh tính git),
`GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_0=core.hooksPath`, `GIT_CONFIG_VALUE_0=<thư mục hook của run>` (mọi job); job của đồng
đội khi đã lưu token Claude: `CLAUDE_CONFIG_DIR` (thư mục trống của run) và `CLAUDE_CODE_OAUTH_TOKEN`. Xem
[Runner](runner.md).

## Lúc build

| Biến | Ý nghĩa |
| --- | --- |
| `TTM_DEFAULT_HOST` | Host mặc định nhúng vào binary (`cargo build`). Bản phát hành dùng `https://toantranmei.com`. |

## Mạng

`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`, `NO_PROXY` được client HTTP tôn trọng như thông thường.
