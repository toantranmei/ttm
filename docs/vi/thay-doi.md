---
title: Thay đổi
description: Lịch sử phiên bản của ttm và ttm-mcp.
order: 18
section: more
---

`ttm` và `ttm-mcp` phát hành cùng phiên bản. Bản tải: <https://github.com/toantranmei/ttm/releases>.

## 0.2.0 (chưa phát hành)

### Thay đổi phá vỡ

- **MCP:** tool trả danh sách giờ trả `structuredContent` là object `{items: […], next_cursor?}` thay vì mảng (MCP yêu cầu
  object khi tool có `outputSchema`). Tool vốn trả object giữ nguyên khoá cũ.
- **Lỗi trên stderr:** `error: <message>` thành mẫu `✕ <tóm tắt>` / `rule:` / `Fix:` / `Help:` ([Mã lỗi](ma-loi.md)).
  `--json` không đổi (chỉ thêm khoá). Mã thoát 2 thêm cho `missing_flag`, `confirmation_required`, `invalid_flag`,
  `unsupported_feature`.

### ttm

- Đầu ra thống nhất qua `ttm-ui`: màu brand (truecolor → 256 → 16), bảng cắt `…`, glyph trạng thái, huy hiệu, `Next:`;
  cờ toàn cục `--plain` (`TTM_PLAIN`) và `-y/--yes`; `TTM_COLUMNS` ([Giao diện dòng lệnh](giao-dien.md)).
- `ttm project init` thành wizard (mọi câu có cờ); lệnh mới `workflow`, `automation`, `agent create|edit|pause|resume|
  archive|skills|mcp`, `skill`, `mcp-server`, `cr`, `inbox [--watch]`, `template`, `workspace members`.
- `ttm sync` đồng bộ `.ttm/workflow.yaml`, `rules/`, `agents/`, `skills/` thành change request theo loại, kiểm YAML,
  chặn secret, kiểm skill ([Cấu hình dạng code](as-code.md)).
- Huy hiệu Mei-er ở `whoami`/`login` (+ `meta.membership` trong `--json`), `workspace members`, `task show`, `agent list`.
- `ttm login` xin thêm scope `factory:admin`.

### ttm-mcp

- Mọi tool có `outputSchema` và Markdown một mẫu; huy hiệu thành viên.
- Tool đọc mới (workflow, Tự động hoá, agent, skill, mẫu, hộp thư) và toolset `admin` với xác nhận hai bước
  (`factory-confirm`, `factory-confirm-cancel`; elicitation ở stdio). Agent đang chạy job không bao giờ thấy `admin`
  ([Tool MCP](mcp-cong-cu.md)).

### ttm runner

- Danh tính git của agent, hook `commit-msg` xoá mọi dòng Claude/Anthropic ở mọi job; skill đã duyệt (kiểm SHA-256); MCP
  từ xa qua gateway và MCP local `[mcp.local.*]`; cô lập job của đồng đội; báo `features` lên server
  ([Runner](runner.md)).

## 0.1.6 (04/11/2026)

### ttm runner

- Runner chạy Claude Code với `--strict-mcp-config`: job chỉ thấy các MCP server của ttm trong cấu hình của job, không
  bao giờ thấy MCP server khác đã cài trên máy, kể cả khi job do thành viên khác giao.

## 0.1.5 (04/11/2026)

Tag 0.1.3 và 0.1.4 đã tạo nhưng chưa từng phát hành (commit của tag chặn CI nên workflow phát hành không chạy); 0.1.5 là bản đầu tiên có các thay đổi dưới đây.

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
