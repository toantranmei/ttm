---
title: Lệnh
description: Danh sách lệnh ttm, cờ chung và đầu ra JSON.
order: 3
---

# Lệnh

Cờ chung cho mọi lệnh: `--host <URL>`, `--profile <tên>`, `--json` (đầu ra JSON cho script và agent), `-v` / `-vv`
(log chi tiết ra stderr).

| Lệnh | Việc |
| --- | --- |
| `ttm login [--device] [--token <PAT>]` | Đăng nhập |
| `ttm logout` | Thu hồi token và xoá khỏi máy |
| `ttm whoami` | Người dùng đang đăng nhập |
| `ttm status` | Máy chủ, profile, token, workspace đang dùng |
| `ttm doctor` | Kiểm tra máy chủ, discovery, token, phiên bản, keyring, git, claude |
| `ttm workspace list` | Các workspace của bạn (`*` là workspace hiện tại) |
| `ttm workspace switch <id|slug|tên>` | Chọn workspace cho profile |
| `ttm workspace current` | Workspace hiện tại |
| `ttm token list` / `create` / `revoke [id]` | Mở trang `/me/ket-noi` (xem lưu ý dưới) |
| `ttm notes search|get|query|comments` | Tìm, đọc page (Markdown), truy vấn database (SQL hoặc bộ lọc JSON), đọc comment |
| `ttm notes create|edit|comment|rows` | Tạo page từ file, sửa (`--old/--new`, `--replace`, `--suggest`), comment neo trích dẫn, ghi dòng database |
| `ttm notes suggestions|resolve` | Xem và chấp nhận/từ chối đề xuất sửa |
| `ttm project init|link|open|status` | Liên kết repo với project Factory |
| `ttm sync [--check] [--resolve]` | Đồng bộ kế hoạch `.ttm/plan`, gửi change request |
| `ttm task list|show|claim|done|approve|split` | Task của project |
| `ttm rules apply` | Ghi luật vào AGENTS.md/CLAUDE.md và hook của agent |
| `ttm runner up|down|status|config|install-service` | Chạy agent theo job từ máy chủ trên máy này |
| `ttm agent run|logs|stop` | Chạy ngay một task, xem log, dừng |
| `ttm mcp install --client <client>` | Cấu hình client MCP dùng `ttm-mcp` |
| `ttm self-update [--check]` | Cập nhật ttm |
| `ttm completion <bash|zsh|fish|powershell|elvish>` | Script tự hoàn thành lệnh |

## Token

API quản lý token chỉ nhận phiên đăng nhập trên web, không nhận token (một token không thể tự tạo token khác). Vì vậy
`ttm token create|list|revoke` mở trình duyệt tới `<máy chủ>/me/ket-noi`; token mới chỉ hiện một lần ở đó.

```sh
ttm token create --scope notee:read,workspace:read --expires 30d --name "CI"
```

## Tự hoàn thành

```sh
ttm completion zsh > "${fpath[1]}/_ttm"
ttm completion bash > ~/.local/share/bash-completion/completions/ttm
ttm completion fish > ~/.config/fish/completions/ttm.fish
```

## JSON

Với `--json`, kết quả là JSON trên stdout; lỗi có dạng `{"error": {"code": "...", "message": "..."}}` và mã thoát khác 0.
