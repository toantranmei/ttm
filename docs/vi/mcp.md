---
title: Kết nối MCP
description: Dùng ttm-mcp với Claude Code, Claude Desktop, Cursor, VS Code, Codex, Windsurf, hoặc MCP từ xa cho Claude.ai và ChatGPT.
order: 6
section: guides
---

Có hai cách cho ứng dụng AI làm việc với Notee và Factory của bạn:

| | MCP cục bộ (`ttm-mcp`) | MCP từ xa |
| --- | --- | --- |
| Chạy ở đâu | Trên máy bạn, ứng dụng khởi động `ttm-mcp` qua stdio | Trên server, ứng dụng gọi `https://toantranmei.com/api/v1/mcp` |
| Cài đặt | `ttm`, `ttm-mcp` | Không cần |
| Đăng nhập | Token của `ttm login` (hoặc `TTM_TOKEN`) | OAuth trong ứng dụng, bấm **Cho phép** trên web |
| Hợp với | Claude Code, Claude Desktop, Cursor, VS Code, Codex, Windsurf, runner | Claude.ai, ChatGPT, ứng dụng hỗ trợ MCP qua URL |
| Tool | Như nhau (cùng mã nguồn) | Như nhau |

AI luôn hành động **thay mặt bạn**, trong phạm vi scope của token và quyền của bạn trên từng page.

## MCP cục bộ

### Chuẩn bị

```sh
ttm login          # ttm-mcp dùng lại token này
ttm doctor         # dòng ttm-mcp phải là [ok  ]
```

`ttm-mcp` đọc token giống `ttm`: `TTM_TOKEN`, rồi keyring, rồi `credentials.toml`, theo profile và host đang chọn. Token
hết hạn và có refresh token thì nó tự gia hạn. Chưa đăng nhập thì máy chủ vẫn khởi động nhưng mọi tool trả lỗi
`unauthorized` (log: ``not signed in to <host>; run `ttm login` (tools will answer unauthorized)``).

### Cài tự động

```sh
ttm mcp install --client claude-code
ttm mcp install --client claude-desktop
ttm mcp install --client cursor
ttm mcp install --client codex
```

| Client | `ttm mcp install` làm gì |
| --- | --- |
| `claude-code` | Chạy `claude mcp add --scope user <tên> -- <ttm-mcp> [args]`. Nếu tên đã có, gỡ (`claude mcp remove --scope user <tên>`) rồi thêm lại. Cần `claude` trên `PATH`. |
| `claude-desktop` | Ghi `mcpServers.<tên>` vào `claude_desktop_config.json` |
| `cursor` | Ghi `mcpServers.<tên>` vào `~/.cursor/mcp.json` |
| `codex` | Ghi `[mcp_servers.<tên>]` vào `$CODEX_HOME/config.toml` (mặc định `~/.codex/config.toml`) |

Với file cấu hình:

- **Gộp, không ghi đè**: máy chủ MCP khác, khoá khác, thứ tự khoá, `env` có sẵn của mục `ttm` và (TOML) chú thích giữ
  nguyên; chỉ `command` và `args` của mục `ttm` được đặt lại.
- **Sao lưu**: file cũ được chép sang `<file>.bak-<giây unix>` trước khi sửa.
- **File hỏng** (JSON/TOML sai) thì không ghi gì và báo lỗi; sửa file rồi chạy lại.
- Chạy lại khi không có gì đổi thì in ``<client> already uses <lệnh> for `<tên>`.`` và không tạo bản sao lưu.

| Cờ | Mặc định | Việc |
| --- | --- | --- |
| `--client <tên>` | (bắt buộc) | `claude-code`, `claude-desktop`, `cursor`, `codex` |
| `--name <tên>` | `ttm` | Tên máy chủ trong cấu hình client |
| `--command <đường dẫn>` | `ttm-mcp` cạnh `ttm`, rồi trên `PATH`, rồi chữ `ttm-mcp` | Lệnh client sẽ chạy. Đường dẫn tuyệt đối vì ứng dụng đồ hoạ không thấy `PATH` của shell. |
| `--toolsets <danh sách>` | (không truyền) | Thêm `--toolsets <danh sách>` vào `args` |
| `--dry-run` | | In nội dung sẽ ghi (hoặc lệnh `claude` sẽ chạy), không ghi gì |
| `--host`, `--profile` (cờ chung) | | Chỉ khi bạn truyền tường minh (hoặc đặt `TTM_HOST`) mới được ghim vào `args` |

```sh
ttm mcp install --client cursor --dry-run
```

```text
--- /Users/an/.cursor/mcp.json (dry run)
{
  "mcpServers": {
    "ttm": {
      "command": "/Users/an/.cargo/bin/ttm-mcp",
      "args": []
    }
  }
}

Dry run: nothing written.
```

```sh
ttm --profile dev --host http://localhost:3000 mcp install --client claude-code --name ttm-dev
```

```text
Claude Code now starts `/Users/an/.cargo/bin/ttm-mcp` as MCP server `ttm-dev`.
Restart the client to load it. Sign in first with `ttm login` if you have not.
```

Khởi động lại ứng dụng sau khi cài.

### Claude Code

Cài tay (phạm vi người dùng, mọi project):

```sh
claude mcp add --scope user ttm -- ttm-mcp
claude mcp add --scope user ttm-notee -- ttm-mcp --toolsets notee
```

Chỉ cho một project, đặt file `.mcp.json` ở gốc repo:

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": ["--toolsets", "notee,factory"] }
  }
}
```

Kiểm tra: gõ `/mcp` trong Claude Code, hoặc `claude mcp list`.

### Claude Desktop

File cấu hình:

| Hệ điều hành | Đường dẫn |
| --- | --- |
| macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| Windows | `%APPDATA%\Claude\claude_desktop_config.json` |
| Linux | `$XDG_CONFIG_HOME/Claude/claude_desktop_config.json` (mặc định `~/.config/Claude/…`) |

```json
{
  "mcpServers": {
    "ttm": {
      "command": "/Users/an/.cargo/bin/ttm-mcp",
      "args": []
    }
  }
}
```

Trên Windows dùng đường dẫn có `\\`: `"C:\\Users\\an\\.cargo\\bin\\ttm-mcp.exe"`. Thoát hẳn Claude Desktop rồi mở lại.

#### Gói `.mcpb` (cài một chạm)

Từ 0.1.3, mỗi bản phát hành trên [GitHub Releases](https://github.com/toantranmei/ttm/releases) có gói Claude Desktop
cho từng hệ điều hành, kèm file `.sha256`:

| Gói | Máy |
| --- | --- |
| `ttm-mcp-darwin.mcpb` | macOS (Apple Silicon và Intel, binary universal) |
| `ttm-mcp-win32-x64.mcpb` | Windows x64 (Windows on ARM chạy qua giả lập) |
| `ttm-mcp-linux-x64.mcpb`, `ttm-mcp-linux-arm64.mcpb` | Linux (binary musl tĩnh) |

Mở file bằng Claude Desktop (bấm đúp, hoặc **Settings → Extensions → Install Extension…**). Gói chạy `ttm-mcp --toolsets
notee` (nhóm `workspace` và `notee`, không có `factory`) và hỏi hai giá trị: **Access token** (PAT `ttm_pat_…` tạo ở
`https://toantranmei.com/me/ket-noi`; để trống thì dùng token đã lưu bởi `ttm login`) và **Server** (mặc định
`https://toantranmei.com`). Không cần cài `ttm-mcp` riêng; cập nhật bằng cách cài gói của bản mới.

### Cursor

`ttm mcp install --client cursor` ghi `~/.cursor/mcp.json` (mọi project). Chỉ cho một project: `.cursor/mcp.json` trong
repo, cùng định dạng:

```json
{
  "mcpServers": {
    "ttm": { "command": "ttm-mcp", "args": [] }
  }
}
```

### VS Code (GitHub Copilot, chế độ agent)

Chưa có `--client vscode`; thêm tay vào `.vscode/mcp.json` của workspace (hoặc lệnh **MCP: Open User Configuration**):

```json
{
  "servers": {
    "ttm": {
      "type": "stdio",
      "command": "ttm-mcp",
      "args": ["--toolsets", "notee"]
    }
  }
}
```

### Codex

`~/.codex/config.toml` (hoặc `$CODEX_HOME/config.toml`):

```toml
[mcp_servers.ttm]
command = "/Users/an/.cargo/bin/ttm-mcp"
args = ["--toolsets", "notee"]
```

### Windsurf và ứng dụng khác

Windsurf: `~/.codeium/windsurf/mcp_config.json`. Hầu hết ứng dụng dùng cùng khối `mcpServers`:

```json
{
  "mcpServers": {
    "ttm": {
      "command": "ttm-mcp",
      "args": [],
      "env": { "TTM_PROFILE": "work" }
    }
  }
}
```

Truyền cấu hình qua `env` khi cần: `TTM_PROFILE`, `TTM_HOST`, `TTM_TOOLSETS`, hoặc `TTM_TOKEN` (máy không có keyring).
Chạy qua npm, không cần cài: `"command": "npx", "args": ["-y", "@toantranmei/mcp"]`.

### Tuỳ chọn của ttm-mcp

```text
Usage: ttm-mcp [OPTIONS]
```

| Cờ | Biến | Việc |
| --- | --- | --- |
| `--host <URL>` | `TTM_HOST` | Máy chủ |
| `--profile <NAME>` | `TTM_PROFILE` | Profile trong `config.toml` |
| `--toolsets <LIST>` | `TTM_TOOLSETS` | Nhóm tool bật, cách nhau bằng dấu phẩy: `notee`, `factory`. `workspace` luôn bật. Không đặt = tất cả. |
| `-v`, `-vv` | `TTM_LOG` | Log ra stderr |
| `-V`, `--version` | | In phiên bản |

`ttm-mcp` không có `--json` hay lệnh con: nó chỉ nói giao thức MCP trên stdin/stdout. Discovery không với tới được thì
nó dùng endpoint mặc định `<host>/api/v1`.

## Nhóm tool (toolset)

| Nhóm | Tool | Scope cần |
| --- | --- | --- |
| `workspace` (luôn bật) | `workspace-get-self`, `workspace-list`, `workspace-get-users` | `workspace:read` |
| `notee` (alias `notes`) | `notee-search`, `notee-fetch`, `notee-query-database`, `notee-get-comments`, `notee-create-pages`, `notee-update-page`, `notee-move-pages`, `notee-duplicate-page`, `notee-create-database`, `notee-update-database`, `notee-upsert-rows`, `notee-delete-rows`, `notee-create-comment`, `notee-get-suggestions`, `notee-resolve-suggestion`; resource `notee://spec/markdown` | `notee:read` / `notee:comment` / `notee:write` |
| `factory` | `factory-get-task`, `factory-get-context`, `factory-report-progress`, `factory-request-approval`, `factory-create-subtasks`, `factory-attach-artifact`, `factory-get-rules`, `factory-renew-lease` | `factory:read` / `factory:run` |

```sh
ttm-mcp --toolsets notee            # workspace + notee
ttm-mcp --toolsets ""               # chỉ workspace
```

Tham số, kết quả và lỗi của từng tool: [Tool MCP](mcp-cong-cu.md). Ít tool hơn giúp model chọn đúng tool và tốn ít context
hơn; bật `factory` chủ yếu cho agent chạy trong runner.

Máy chủ tự giới thiệu với client là `ttm` kèm hướng dẫn: *Start with workspace-get-self to learn who the user is and which
workspace is current.* Tên ứng dụng client (ví dụ `claude-code`) được ghi vào lịch sử page khi tool sửa page: “An (via
agent claude-code)”.

## Biến của lần chạy agent

Khi runner chạy một task, nó khởi động `ttm-mcp` với:

| Biến | Giá trị |
| --- | --- |
| `TTM_TOKEN` | Token của job (`ttm_job_…`), chỉ dùng được cho run đó |
| `TTM_HOST` | Host của runner |
| `TTM_NO_KEYRING` | `1` |
| `TTM_RUN_ID` | Id của run |
| `TTM_TASK` | Key task, ví dụ `AUR-12` |
| `TTM_PROJECT` | Id (hoặc key) của project |

Nhờ đó các tool `factory-*` biết “task của run này” mà agent không cần truyền key. Ngoài một run, các tool cần run
(`factory-report-progress`, `factory-request-approval`, `factory-attach-artifact`, `factory-renew-lease`) trả lỗi
`no_run`.

## MCP từ xa

URL: **`https://toantranmei.com/api/v1/mcp`** (MCP Streamable HTTP, không giữ phiên).

### Claude.ai

1. **Settings → Connectors → Add custom connector**.
2. Tên tuỳ ý (ví dụ *toantranmei*), URL `https://toantranmei.com/api/v1/mcp`, bấm **Add**, rồi **Connect**.
3. Trình duyệt mở trang cho phép của toantranmei.com; đăng nhập nếu cần và bấm **Cho phép**.

### ChatGPT

Trong **Settings → Connectors** (cần bật chế độ developer cho connector tuỳ chỉnh), tạo connector mới với URL trên, chọn
xác thực OAuth và hoàn tất bước cho phép.

### Trang cho phép hiện gì

- Tiêu đề **“<tên ứng dụng> muốn truy cập tài khoản của bạn”**, và dòng *Ứng dụng bên ngoài. Chỉ cho phép nếu bạn tin ứng
  dụng này.*
- Cảnh báo đỏ nếu tên ứng dụng **giống** tên ứng dụng của toantranmei mà không phải ứng dụng chính thức.
- Nơi trình duyệt quay về sau khi cho phép (*Sau khi cho phép, trình duyệt quay về <host>.*).
- Danh sách quyền xin cấp. Ứng dụng không xin scope cụ thể thì nhận mọi scope trừ `offline_access`.
- Ô **Giới hạn trong workspace** khi bạn ở nhiều workspace.
- Nút **Cho phép** / **Từ chối**.

Token của ứng dụng từ xa sống **1 giờ**; ứng dụng chỉ có refresh token (90 ngày, xoay vòng mỗi lần dùng) khi xin
`offline_access`. Dùng lại một refresh token cũ sẽ thu hồi cả họ token. Ngắt kết nối ở `<host>/me/ket-noi`, tab **Ứng dụng
đã kết nối**.

### Chi tiết cho người viết client

| Mục | Giá trị |
| --- | --- |
| Không có token | `401` với `WWW-Authenticate: Bearer resource_metadata="https://toantranmei.com/.well-known/oauth-protected-resource/api/v1/mcp"` |
| Protected Resource Metadata (RFC 9728) | `/.well-known/oauth-protected-resource` (và hậu tố `/api/v1/mcp`) |
| Authorization Server Metadata (RFC 8414) | `/.well-known/oauth-authorization-server` |
| Trang cho phép | `/ket-noi/cli` |
| Token, thu hồi, thiết bị | `/api/v1/oauth/token`, `/api/v1/oauth/revoke`, `/api/v1/oauth/device_authorization` |
| Đăng ký client | Client ID Metadata Document (`client_id` là URL), hoặc Dynamic Client Registration `/api/v1/oauth/register` |
| PKCE | Bắt buộc, `S256` |
| `resource` | Phải là `https://toantranmei.com/api/v1/mcp`, khác thì `invalid_target` |

Mỗi lần gọi tool trên MCP từ xa đi qua cùng kiểm tra token, scope và quyền như một request API. MCP từ xa bật mọi nhóm
tool; các tool Factory cần run trả `no_run` ở đây.

## Giới hạn

| Giới hạn | Giá trị |
| --- | --- |
| Lần ghi Notee của agent | 60 mỗi giờ mỗi token (tạo page, tạo dòng bằng page, sửa page); vượt thì `429 rate_limited` |
| Một lần ghi | 1.000 block (tính cả block lồng), 2 MB Markdown |
| `notee-create-pages` | 1–20 page mỗi lần |
| `notee-upsert-rows` | 1–100 dòng mỗi lần |
| `notee-search` | Tối đa 20 kết quả; câu tìm ≤ 8 từ, ≤ 200 ký tự |
| `notee-query-database` | 1–100 dòng mỗi trang (mặc định 100), 3 giây, 4 MB mỗi trang, 2 truy vấn cùng lúc mỗi người |
| Page dài | Cắt sau 20.000 node (`truncated`) |
| Đoạn trích của comment | 1–1.000 ký tự |
| `factory-create-subtasks` | 10 task con mỗi run, 30 dưới một task gốc, sâu tối đa 2 cấp |

## Thử máy chủ bằng MCP Inspector

```sh
npx @modelcontextprotocol/inspector ttm-mcp --toolsets notee
```

Inspector mở giao diện web để liệt kê tool, đọc resource `notee://spec/markdown` và gọi thử từng tool.
