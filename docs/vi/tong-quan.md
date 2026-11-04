---
title: Tổng quan
description: ttm và ttm-mcp là gì, các khái niệm chính và cách các thành phần nối với nhau.
order: 1
section: start
---

**ttm** là bộ công cụ mã nguồn mở (MIT) của [toantranmei.com](https://toantranmei.com), gồm hai chương trình phát hành
cùng phiên bản:

| Chương trình | Là gì | Dùng khi |
| --- | --- | --- |
| `ttm` | Dòng lệnh (CLI) | Đăng nhập, đổi workspace, đọc và sửa page Notee, liên kết repo với dự án Factory, đồng bộ kế hoạch, ép luật cho agent, chạy agent trên máy bạn, cài MCP vào các ứng dụng AI |
| `ttm-mcp` | Máy chủ [MCP](https://modelcontextprotocol.io) chạy trên máy bạn qua stdio | Cho Claude Code, Claude Desktop, Cursor, Codex, VS Code… đọc và ghi Notee, làm việc trong task Factory thay mặt bạn |

Mã nguồn: <https://github.com/toantranmei/ttm>. Phiên bản hiện tại: **0.1.6**.

Nếu bạn không dùng terminal, bạn không cần cài gì: dán **URL MCP từ xa** `https://toantranmei.com/api/v1/mcp` vào
Claude.ai hoặc ChatGPT (xem [Kết nối MCP](mcp.md)).

## Khái niệm

| Khái niệm | Ý nghĩa |
| --- | --- |
| **Host** | Máy chủ mà `ttm` nói chuyện cùng. Mặc định `https://toantranmei.com`; đổi bằng `--host`, `TTM_HOST` hoặc profile (ví dụ `http://localhost:3000` khi phát triển). |
| **Discovery** | Tài liệu `GET <host>/.well-known/ttm` cho biết địa chỉ API, OAuth, MCP, phiên bản API, `min_cli` và tính năng (`notee`, `factory`). ttm không viết cứng URL nào ngoài host. Bản lưu đệm sống 1 giờ. |
| **Profile** | Một bộ (host, workspace hiện tại) có tên trong `config.toml`, ví dụ `default`, `work`. Mỗi profile có token riêng. |
| **Workspace** | Không gian làm việc (`personal` hoặc `team`) chứa page Notee và dự án Factory. Bạn có vai trò `owner`, `admin`, `member` hoặc `guest`. Workspace hiện tại lưu theo profile. |
| **Token** | Chuỗi bí mật bắt đầu bằng `ttm_`: `ttm_pat_` (token cá nhân tạo trên web), `ttm_oat_` (token truy cập OAuth, do `ttm login` hoặc ứng dụng MCP từ xa nhận), `ttm_ort_` (refresh token), `ttm_job_` (token của runner và của từng lần chạy agent). |
| **Scope** | Quyền của token: `workspace:read`, `notee:read`, `notee:write`, `notee:comment`, `factory:read`, `factory:write`, `factory:run`, `offline_access`. Quyền thật = giao của scope và quyền của bạn trên từng tài nguyên. Xem [Đăng nhập](dang-nhap.md). |
| **Notee** | Ứng dụng ghi chú: page, database, comment, đề xuất sửa. Agent đọc và ghi page dưới dạng [Notee-flavored Markdown](notee-markdown.md). |
| **Factory** | Ứng dụng quản lý dự án cho người và agent: project, task, workflow, cổng duyệt, change request, merge queue. Web là nguồn sự thật; repo giữ bản sao kế hoạch trong `.ttm/plan`. |
| **Runner** | `ttm runner up`: tiến trình trên máy dev nhận *job* từ server, tạo git worktree, chạy Claude Code bằng tài khoản Claude của bạn, gửi sự kiện về Execution Map, đẩy nhánh và mở pull request. |
| **Harness** | Bộ chuyển đổi để chạy một coding agent. Hiện có `claude-code`. |
| **MCP cục bộ và MCP từ xa** | MCP cục bộ là `ttm-mcp` trên máy bạn (stdio, dùng token của `ttm login`). MCP từ xa là endpoint `https://toantranmei.com/api/v1/mcp` (Streamable HTTP, OAuth 2.1), dùng cùng bộ tool. |

## Kiến trúc

```text
                      ┌──────────────── toantranmei.com (server) ─────────────────┐
 Claude.ai, ChatGPT ─▶│ /api/v1/mcp        MCP từ xa (Streamable HTTP, OAuth 2.1)  │
                      │ /.well-known/ttm   discovery                               │
 ttm, ttm-mcp ───────▶│ /api/v1            API (Bearer token, scope, ACL)          │
   (Bearer)           │ /ket-noi/cli       trang cho phép đăng nhập                │
                      │ /activate          nhập mã thiết bị                        │
                      │ /me/ket-noi        quản lý token, ứng dụng, runner         │
                      └────────────────────────────────────────────────────────────┘
                              ▲ long-poll + heartbeat (chỉ kết nối đi ra)
                              │
 máy dev:  ttm runner ── git worktree mỗi task ── claude -p (tài khoản của bạn)
                                                      │ stdio
                                                      ▼
                                                 ttm-mcp (token của lần chạy)
```

- **Phần "não" nằm ở server**: chia task, chọn model, ghép context và prompt. `ttm` chỉ xác thực, tạo worktree, chạy
  harness theo job, gửi sự kiện, ép luật cục bộ.
- **Agent chỉ chạy trên máy bạn**, bằng gói Claude hoặc API key của bạn. Server không giữ key Claude.
- **Cùng một bộ tool** cho MCP cục bộ và MCP từ xa: crate `ttm-mcp-tools` được server dùng lại, nên tên tool, tham số và
  kết quả giống nhau.

## Repo có gì

| Crate | Vai trò |
| --- | --- |
| `ttm-api` | Client HTTP cho `/api/v1`: thử lại, User-Agent, cảnh báo `Deprecation`, lỗi 426, problem+json |
| `ttm-core` | Cấu hình, profile, chọn host, discovery, lưu token, file dự án, worktree |
| `ttm-mcp-tools` | Tool MCP trên trait `Backend` (dùng chung với MCP từ xa) |
| `ttm-runner` | Runner: nhận job, worktree, harness Claude Code, luồng sự kiện, merge queue |
| `ttm-cli` | Binary `ttm` |
| `ttm-mcp` | Binary `ttm-mcp` |

## Đọc tiếp

- [Cài đặt](cai-dat.md) rồi [Bắt đầu nhanh](bat-dau-nhanh.md) (5 phút).
- [Kết nối MCP](mcp.md) để dùng với Claude, Cursor, Codex.
- [Tham chiếu lệnh](lenh.md) và [Tool MCP](mcp-cong-cu.md) khi cần tra cứu.
