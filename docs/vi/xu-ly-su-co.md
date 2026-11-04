---
title: Xử lý sự cố và hỏi đáp
description: Cách chẩn đoán, các lỗi thường gặp theo triệu chứng khi cài, đăng nhập, dùng MCP, Notee, Factory, runner, và câu hỏi thường gặp.
order: 16
section: more
---

## Chẩn đoán trước

```sh
ttm doctor            # host, discovery, phiên bản, token, keyring, git, claude, ttm-mcp
ttm status --json     # host, profile, nguồn token, workspace đang dùng
ttm -vv whoami        # log debug của request (token được che)
```

Khi báo lỗi trên GitHub, đính kèm kết quả `ttm doctor`, `ttm --version`, hệ điều hành và kênh cài.

## Cài đặt

**`brew install` báo Xcode quá cũ.**
Homebrew từ chối khi Xcode cũ. Cập nhật Xcode trong App Store, hoặc gỡ Xcode và dùng Command Line Tools:
`sudo rm -rf /Applications/Xcode.app && xcode-select --install`.

**Cài bằng npm xong, `ttm` báo không tìm thấy binary.**
npm đã bỏ qua script `postinstall` (npm 11 cảnh báo về script của gói cài toàn cục). Cài lại và cho phép đúng hai gói:
`npm install -g --allow-scripts=@toantranmei/cli,@toantranmei/mcp @toantranmei/cli @toantranmei/mcp`.

**`command not found: ttm` sau khi chạy script cài.**
Mở terminal mới, hoặc `source ~/.cargo/env`. Kiểm `~/.cargo/bin` có trong `PATH`.

**Windows trên ARM.**
Không có bản ARM; cài bản x64 (PowerShell, MSI hoặc npm), Windows chạy nó qua giả lập.

**macOS chặn binary tải bằng trình duyệt.**
`xattr -d com.apple.quarantine ./ttm ./ttm-mcp`, hoặc cài bằng script/Homebrew.

**Windows SmartScreen cảnh báo MSI.**
Binary chưa ký số. Kiểm SHA-256 và attestation ([Cài đặt](cai-dat.md)) rồi chọn **More info → Run anyway**.

**`ttm self-update` báo `cannot write into …`.**
Bạn không có quyền ghi vào thư mục cài (thường do cài bằng Homebrew, npm, MSI). Cập nhật bằng trình quản lý gói tương ứng.

**`self-update` báo `GitHub answered 403`.**
Giới hạn API ẩn danh của GitHub. Thử lại sau, hoặc cài lại bằng script.

## Đăng nhập

**`ttm login` mở trình duyệt nhưng không bao giờ xong.**
Bạn đang SSH vào máy khác (trang callback ở `127.0.0.1` của máy kia) hoặc tường lửa chặn cổng cục bộ. Dùng
`ttm login --device`.

**`timed out after 5 minutes waiting for the browser`.**
Chạy lại và bấm **Cho phép** trong 5 phút. Dùng `--no-browser` để tự mở URL.

**`the code expired before it was approved`.**
Mã thiết bị sống 10 phút. Chạy lại `ttm login --device`.

**`` not signed in to …; run `ttm login` `` dù đã đăng nhập.**
Token gắn với (profile, host). Có thể bạn đang dùng profile hoặc host khác: xem `ttm status`. `TTM_HOST`, `TTM_PROFILE`,
`--host` đều đổi cặp đó.

**`invalid_token` sau một thời gian.**
Token hết hạn (với `ttm login`: không dùng `ttm` quá 90 ngày, refresh token đã hết hạn) hoặc đã bị thu hồi. `ttm login` lại. Nếu `ttm status` báo nguồn
`TTM_TOKEN`, biến đó đang giữ token cũ: bỏ hoặc thay nó.

**`insufficient_scope … [requires scope X]` / `{"required": "X"}`.**
Token thiếu scope X. PAT: tạo token mới có X. `ttm login --scope` hẹp: chạy lại `ttm login` (mặc định xin mọi scope).

**`keyring unavailable …; saving the token to credentials.toml`.**
Máy không có keyring (Linux không có Secret Service, container). Token vẫn lưu trong file 0600. Đặt `TTM_NO_KEYRING=1` để
tắt cảnh báo.

**`ttm doctor` báo `token … valid but lacks workspace:read`.**
PAT không có `workspace:read`: lệnh Notee/Factory vẫn chạy, nhưng `whoami`, `workspace list` thì không.

## MCP

**Ứng dụng không thấy máy chủ `ttm`.**
Khởi động lại hẳn ứng dụng sau `ttm mcp install`. Kiểm `command` trong cấu hình là đường dẫn tuyệt đối tồn tại (ứng dụng
đồ hoạ không thấy `PATH` của shell). Claude Code: `claude mcp list`.

**Mọi tool trả `unauthorized`.**
`ttm-mcp` không tìm thấy token. Chạy `ttm login` với cùng profile/host mà cấu hình MCP dùng (mặc định là `default_profile`);
nếu cấu hình có `--profile`/`--host`, đăng nhập với đúng giá trị đó. Máy không có keyring: thêm `TTM_TOKEN` vào `env` của
cấu hình MCP.

**`ttm mcp install` báo `the existing config is not valid JSON; fix it first`.**
File cấu hình của ứng dụng đang hỏng; ttm không ghi gì. Sửa file (bản sao lưu cũ nằm ở `<file>.bak-<số>` nếu từng cài)
rồi chạy lại.

**``Claude Code (`claude`) is not on PATH``.**
Cài Claude Code, hoặc chạy lệnh in kèm thông báo bằng tay (`claude mcp add --scope user ttm -- …`).

**Tool `factory-*` trả `no_run`.**
Các tool đó dành cho agent trong một lần chạy của runner. Ngoài run, chỉ `factory-get-task`, `factory-get-context` (truyền
`task`) và `factory-get-rules` (truyền `project`) dùng được.

**Model chọn sai tool hoặc tốn nhiều context.**
Giảm số tool: `ttm-mcp --toolsets notee` (hoặc `ttm mcp install --toolsets notee`).

**Kết nối MCP từ xa trong Claude.ai thất bại.**
Kiểm URL đúng `https://toantranmei.com/api/v1/mcp`. Đăng nhập web trước rồi bấm **Connect** lại. Đã từ chối nhầm: kết nối
lại. Xoá kết nối cũ ở `/me/ket-noi` → **Ứng dụng đã kết nối**.

## Notee

**`ttm notes edit` in `Page updated.` nhưng không đổi gì như mong đợi, kèm `warning: … old_str not found; skipped`.**
`--old` phải khớp chính xác Markdown server trả, phân biệt hoa thường, kể cả ký tự thoát (`\*`, `\_`) và tab thụt lề. Đọc
lại bằng `ttm notes get` và chép đúng đoạn đó.

**`old_str matches N places`.**
Thêm chữ xung quanh để chỉ khớp một chỗ, hoặc dùng `--replace-all`.

**Sửa page lại thành đề xuất (`Suggestion … created`) dù không có `--suggest`.**
Workspace đặt `agent_write_mode = suggest`, hoặc token chỉ có `notee:comment`. Người có quyền sửa duyệt bằng
`ttm notes resolve`.

**`would_delete_children` khi `--replace`.**
Nội dung mới thiếu thẻ `<page>`/`<database>` của page con. Giữ các thẻ đó, hoặc thêm `--allow-deleting-content` nếu thật
sự muốn xoá.

**`too_many_blocks`.**
Lần ghi đổi hơn 1.000 block. Chia thành nhiều lần `--old/--new` nhỏ.

**`rate_limited` khi agent sửa liên tục.**
Mỗi token được 60 lần ghi Notee mỗi giờ. Gộp thay đổi vào ít lần ghi hơn.

**`ambiguous_anchor` khi comment.**
Đoạn trích xuất hiện nhiều lần; thêm `--prefix`/`--suffix` (MCP: `prefix`/`suffix`).

**SQL báo `sql_table: Table 'Rows' is not allowed`.**
Tên bảng là `rows` viết thường, không nháy. Cột mới cần nháy kép: `"Trạng thái"`.

**`query_timeout`.**
Truy vấn quá 3 giây: thêm `WHERE`, `LIMIT`, bỏ hàm nặng.

**`ttm notes get` của một dòng database không thấy giá trị cột.**
Giá trị cột chỉ có trong `--json` (`properties`).

## Factory

**`this directory is not linked to a project`.**
Chạy `ttm project link <KEY>` ở gốc repo, rồi commit `.ttm/project.toml` và `.ttm/plan`.

**`ttm sync --check` thoát 1 trong CI.**
Kế hoạch trong repo lệch với server. Chạy `ttm sync` ở máy dev và commit `.ttm/plan`, hoặc duyệt change request đang chờ.

**`Conflicts with the server`.**
Cùng trường bị đổi ở cả hai phía. Sửa `.ttm/plan` rồi `ttm sync` lại, hoặc `ttm sync --resolve` (cần `claude`).

**Sửa local không lên server.**
Một change request đang chờ duyệt (`ttm project status` cho thấy id); `ttm sync` không gửi lại cho tới khi nó được quyết.
Duyệt trên web (người tạo không tự duyệt được).

**`the ttm markers are broken`.**
`AGENTS.md` hoặc `CLAUDE.md` chỉ còn một trong hai dấu `<!-- ttm:begin -->` / `<!-- ttm:end -->`. Sửa tay rồi
`ttm rules apply`.

**Hook chặn commit: `commit message must follow Conventional Commits`.**
Dùng dạng `type(scope): tóm tắt`, ví dụ `fix(auth): retry on 503`. Project tắt được luật này trên web.

**`ttm task done` báo `conflict` với `approval-required`, `tests-required`…**
Workflow chặn chuyển trạng thái. Xem bảng ở [Mã lỗi](ma-loi.md).

## Runner

**``no agent harness found; install Claude Code (`claude`) first``.**
Cài Claude Code. Chạy dưới dịch vụ mà vẫn báo: dịch vụ dùng `PATH` ghi lúc cài (bản trước 0.1.4 không ghi `PATH`). Chạy
lại `ttm runner install-service` từ shell mà `which claude` tìm thấy (xem [Runner](runner.md)).

**Job bị từ chối: `refused AUR-12: no local clone is configured for project AUR`.**
`ttm runner config --repo AUR=~/code/aurora` rồi khởi động lại runner.

**`the job's repository … is not the origin of …`.**
`origin` của clone không phải repo của project. Sửa remote (`git remote set-url origin …`) hoặc trỏ `--repo` tới đúng clone.

**Job bị từ chối: `` refused AUR-12: the job's worktree_hint `…` is an absolute path; worktrees stay under the runner's worktree_root … ``.**
Server gửi một `worktree_hint` không nằm dưới `worktree_root` (tuyệt đối, có `..` hoặc qua symlink ra ngoài). Runner từ
chối (`invalid_worktree`) và không tạo gì; báo lỗi cho người quản trị server.

**Runner tự chạy lại sau `ttm runner down`.**
Dịch vụ (launchd `KeepAlive`, systemd, Task Scheduler) đang giữ nó. Tắt hẳn bằng `ttm runner uninstall-service`.

**Run kết thúc `paused`.**
Claude báo hết hạn mức; runner tạm ngừng nhận job tới giờ reset, server xếp lại việc.

**Không có pull request sau run thành công.**
Cần `gh` đã đăng nhập (`gh auth status`) và repo có remote. Nhánh vẫn được đẩy lên; mở PR bằng tay.

**Hết dung lượng vì worktree.**
Giảm `--keep-days`, hoặc xoá thư mục trong `~/.ttm/worktrees` rồi `git worktree prune` trong repo.

## Hỏi đáp

**ttm có miễn phí và mã nguồn mở không?**
Có, MIT, tại <https://github.com/toantranmei/ttm>. Giới hạn sử dụng (số agent, song song…) theo gói của workspace trên
toantranmei.com.

**Khác gì giữa `ttm-mcp` và MCP từ xa?**
Cùng tool. `ttm-mcp` chạy trên máy bạn với token của `ttm login`, hợp với ứng dụng trên máy và runner. MCP từ xa không cần
cài, dùng OAuth trong Claude.ai, ChatGPT.

**Agent dùng token Claude của ai?**
Của bạn: runner gọi Claude Code đã đăng nhập trên máy bạn. Server không bao giờ nhận key Claude, và agent chạy trên máy
không trừ credit của toantranmei.

**Dùng với server tự host hay localhost được không?**
Có: `--host http://localhost:3000` hoặc một profile riêng. ttm đọc mọi địa chỉ từ discovery `/.well-known/ttm`.

**Nhiều tài khoản trên một máy?**
Mỗi tài khoản một profile: `ttm --profile work login`, `ttm --profile personal login`. Chọn profile mặc định bằng
`default_profile`.

**Dùng trong CI thế nào?**
PAT tối thiểu + `TTM_TOKEN`, ví dụ `ttm sync --check` để chặn PR làm lệch kế hoạch. Xem [Đăng nhập](dang-nhap.md).

**Có Cargo/crates.io không?**
Chưa đăng crates.io; cài từ mã nguồn bằng `cargo install --git https://github.com/toantranmei/ttm ttm-cli`.

**Có gói `.mcpb` cho Claude Desktop không?**
Có, từ 0.1.4: tải `ttm-mcp-<hệ điều hành>.mcpb` ở GitHub Releases (xem [MCP](mcp.md)). Bản cũ hơn: dùng
`ttm mcp install --client claude-desktop` hoặc MCP từ xa.

**Dữ liệu của tôi có được dùng để huấn luyện model không?**
ttm không gửi dữ liệu cho bên thứ ba. Nội dung bạn đưa cho Claude, ChatGPT… theo chính sách của nhà cung cấp đó.
