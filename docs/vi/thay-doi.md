---
title: Thay đổi
description: Lịch sử phiên bản của ttm và ttm-mcp.
order: 18
section: more
---

`ttm` và `ttm-mcp` phát hành cùng phiên bản. Bản tải: <https://github.com/toantranmei/ttm/releases>.

## 0.1.3 (chưa phát hành)

### Lỗi và mã lỗi

- Lỗi của server giữ `rule`, `field`, `params`: `ttm` in `rule`/`field` trong thông báo lỗi; `--json` và lỗi của tool MCP
  (`structuredContent.error`) có thêm `status`, `rule`, `field`, `params` để rẽ nhánh theo `split-limit`, `rev-conflict`,
  `approval-required`, hoặc đọc `params.required` của `insufficient_scope`. Xem [Mã lỗi](ma-loi.md).
- Mô tả tool `factory-create-subtasks` ghi đúng quy tắc `split-limit` (`409 conflict`) thay vì `split_limit`.

### ttm

- `ttm task split` nhận khoá camelCase hoặc snake_case (`acceptance_criteria`, `depends_on`, như tool MCP) và cảnh báo
  khoá lạ.
- `ttm task approve --reject` in `rejected` (trước đây `rejectd`).
- `ttm hook pre-bash` bảo vệ các nhánh trong `hooks.pre_bash.protected_branches` của luật đã lưu, rồi nhánh chính trong
  `.ttm/project.toml`, rồi `main`/`master` (trước đây luôn là `main`/`master`).
- `ttm notes query --page-size` chỉ nhận 1–100, báo lỗi rõ trước khi gọi server.
- Đăng nhập CLI: token truy cập 7 ngày, tự gia hạn; vẫn đăng nhập chừng nào còn dùng `ttm` ít nhất mỗi 90 ngày (đi cùng
  thay đổi phía server sắp phát hành).

### Runner

- Dịch vụ (launchd, systemd) mang `PATH` của shell chạy `install-service` cộng `~/.local/bin`, `~/.cargo/bin`,
  `/opt/homebrew/bin`, `/usr/local/bin`…, nên tìm thấy `claude`, `gh`, `git`. Lệnh mới `ttm runner uninstall-service`.
- `worktree_hint` của job chỉ được là đường dẫn tương đối dưới `worktree_root`; đường dẫn tuyệt đối, `..` hay symlink ra
  ngoài bị từ chối (`invalid_worktree`).
- Long-poll job với `wait=25` (tối đa của server); yêu cầu đang chờ không bị huỷ khi tới lượt heartbeat.

### Phát hành

- Gói Claude Desktop `.mcpb` cho macOS (universal), Linux x64/arm64, Windows x64 trên GitHub Releases. Phiên bản của
  manifest theo `Cargo.toml` (CI kiểm).

## 0.1.2 (04/11/2026)

Bản phát hành đầu tiên.

### Kênh phát hành

- Script cài shell (macOS, Linux) và PowerShell (Windows), archive cho 7 target: macOS arm64/x64, Linux arm64/x64 (glibc
  và musl), Windows x64; MSI cho Windows x64.
- Homebrew: `brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp`.
- npm: `@toantranmei/cli` (lệnh `ttm`), `@toantranmei/mcp` (lệnh `ttm-mcp`), đăng bằng trusted publishing.
- SHA-256 cho mọi file và GitHub artifact attestation.
- Không có bản Windows on ARM (bản x64 chạy qua giả lập).

### ttm

- Đăng nhập: trình duyệt (loopback + PKCE), `--device` (RFC 8628), `--token` / stdin; `logout` thu hồi trên server;
  `whoami`, `status`, `doctor`.
- Profile và `--host` / `TTM_HOST`, discovery `/.well-known/ttm` có bộ đệm, token trong keyring hoặc `credentials.toml`
  0600, `TTM_TOKEN` cho CI.
- `workspace list/switch/current`; `token list/create/revoke` (mở trang web).
- Notee: `notes search/get/query/comments/create/edit/comment/rows upsert/rows delete/suggestions/resolve`, đề xuất sửa
  `--suggest`.
- Factory: `project init/link/open/status`, `sync` với change request, `--check` cho CI, `--resolve` giải xung đột bằng
  agent trong worktree riêng; `task list/show/claim/done/approve/split`; `rules apply` (khối quản lý trong
  `AGENTS.md`/`CLAUDE.md`, hook Claude Code); `hook pre-bash/stop`.
- Runner: `runner up/down/status/config/install-service` (launchd, systemd `--user`, Task Scheduler), harness Claude Code,
  worktree mỗi task, sự kiện trực tiếp, push và PR qua `gh`, merge queue tuỳ chọn; chỉ chạy job cho repo đã khai báo
  (`repo_not_configured`, `invalid_branch`), gia hạn token runner 12 giờ, dừng khi token liên tục bị từ chối.
- `agent run/logs/stop`.
- `mcp install --client claude-code|claude-desktop|cursor|codex` (gộp cấu hình, sao lưu, `--dry-run`).
- `self-update` (kiểm SHA-256 và attestation), `completion`.
- `TTM_HOME` thay thư mục nhà (Windows bỏ qua `HOME`).

### ttm-mcp

- Máy chủ MCP qua stdio, 26 tool: nhóm `workspace` (3), `notee` (15), `factory` (8); resource `notee://spec/markdown`;
  `--toolsets`.
- Cùng bộ tool với MCP từ xa `https://toantranmei.com/api/v1/mcp`.

## 0.1.1 (huỷ)

Đã gắn tag nhưng không phát hành: không build được MSI cho Windows on ARM trên máy build Linux của dist. Bản 0.1.2 bỏ
target đó.

## 0.1.0 (không phát hành)

Đã gắn tag trước khi có định nghĩa MSI nên quy trình phát hành không chạy được; không có gói nào được đăng.
