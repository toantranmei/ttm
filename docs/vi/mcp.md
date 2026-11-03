---
title: Kết nối MCP
description: Dùng ttm-mcp với Claude Code, Claude Desktop, Cursor và Codex, hoặc MCP từ xa không cần cài.
order: 4
---

# Kết nối MCP

`ttm-mcp` là máy chủ MCP chạy trên máy bạn (stdio), dùng token của `ttm login` (hoặc `TTM_TOKEN`). AI làm việc thay mặt
bạn, trong phạm vi quyền của token.

## Cài nhanh

```sh
ttm login
ttm mcp install --client claude-code      # chạy: claude mcp add --scope user ttm -- ttm-mcp
ttm mcp install --client claude-desktop   # sửa claude_desktop_config.json
ttm mcp install --client cursor           # sửa ~/.cursor/mcp.json
ttm mcp install --client codex            # sửa ~/.codex/config.toml
```

File cấu hình được **sao lưu** (`<file>.bak-<thời điểm>`) trước khi sửa và được **gộp**: máy chủ MCP khác và thiết lập
khác giữ nguyên. File hỏng thì ttm không ghi gì. Xem trước bằng `--dry-run`.

## Cấu hình tay

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": [] }
  }
}
```

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.ttm]
command = "ttm-mcp"
args = []
```

## Tuỳ chọn của ttm-mcp

| Cờ | Việc |
| --- | --- |
| `--host <URL>` | Máy chủ (mặc định như `ttm`) |
| `--profile <tên>` | Profile trong `config.toml` |
| `--toolsets notee,factory` | Nhóm tool bật thêm (nhóm `workspace` luôn bật) |
| `-v`, `-vv` | Log ra stderr |

## Tool

| Tool | Việc |
| --- | --- |
| `workspace-get-self` | Người dùng và workspace hiện tại |
| `workspace-list` | Các workspace |
| `workspace-get-users` | Thành viên workspace (lọc theo tên/email) |

Tool Notee và Factory được thêm ở các bản sau.

## Claude Desktop không cần terminal

Tải file `.mcpb` cho hệ điều hành của bạn ở trang Releases, mở bằng Claude Desktop, dán token tạo ở `/me/ket-noi`.

## MCP từ xa

Không muốn cài gì: thêm URL MCP từ xa (hiện ở trang này trên web) vào Claude.ai hoặc ChatGPT, đăng nhập và cho phép.
