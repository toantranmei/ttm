---
title: Bắt đầu nhanh
description: Năm phút từ cài đặt tới đăng nhập, chọn workspace, đọc page Notee và thêm MCP vào Claude Code.
order: 3
section: start
---

Cần: một tài khoản trên [toantranmei.com](https://toantranmei.com) và một terminal. Các ví dụ dùng macOS/Linux; trên
Windows dùng PowerShell với cùng lệnh `ttm`.

## 1. Cài đặt

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
ttm --version
```

Kênh khác (Homebrew, npm, MSI): xem [Cài đặt](cai-dat.md).

## 2. Đăng nhập

```sh
ttm login
```

```text
Open this URL in your browser / Mở URL này trong trình duyệt:
  https://toantranmei.com/ket-noi/cli?response_type=code&client_id=ttm-cli&redirect_uri=http%3A%2F%2F127.0.0.1%3A53124%2Fcallback&…
Waiting for the browser (5 minutes)… On a machine without a browser use `ttm login --device`.
```

Trình duyệt mở trang **“ttm CLI muốn truy cập tài khoản của bạn”**, liệt kê quyền xin cấp. Nếu bạn ở nhiều workspace, có
ô **Giới hạn trong workspace** (để trống = mọi workspace). Bấm **Cho phép**; trang chuyển về `127.0.0.1` báo
“Đã đăng nhập, quay lại terminal.”

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

Đang SSH hoặc trong container? Dùng `ttm login --device` (xem [Đăng nhập](dang-nhap.md)).

## 3. Kiểm tra

```sh
ttm whoami
```

```text
An Nguyễn <an@example.com>
host: https://toantranmei.com  profile: default
```

## 4. Chọn workspace

```sh
ttm workspace list
```

```text
  Cá nhân  personal · owner · 0190f3a2-7c1e-7b55-9d0a-3c2b1e4f5a60
  Team Aurora [aurora]  team · member · 0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71
```

```sh
ttm workspace switch aurora
```

```text
Current workspace for profile `default`: Team Aurora (0190f3a2-8d2f-7c66-8e1b-4d3c2f5a6b71)
```

`switch` nhận id, slug, tên đầy đủ hoặc phần đầu tên (không phân biệt hoa thường) nếu chỉ khớp một workspace. Lựa chọn
được lưu vào profile, và `ttm notes search`, `ttm project init`, `ttm-mcp` dùng nó làm mặc định.

## 5. Tìm và đọc một page

```sh
ttm notes search "ke hoach"
```

```text
0190f3b0-1111-7aaa-8bbb-0123456789ab  Kế hoạch quý 4
0190f3b0-2222-7aaa-8bbb-0123456789ab  Họp tuần
    …chốt kế hoạch phát hành bản 0.2 trước ngày 15…
```

Tìm kiếm không phân biệt dấu ("ke hoach" thấy "Kế hoạch"); dòng thụt vào là đoạn trích khi chữ khớp nằm trong nội dung.

```sh
ttm notes get 0190f3b0-1111-7aaa-8bbb-0123456789ab
```

```markdown
# Mục tiêu
- Phát hành ttm 0.2
<callout icon="💡" color="yellow">
	Ưu tiên tài liệu tiếng Việt
</callout>
```

Bạn có thể dán cả URL của page thay cho id. Lưu ra file: `ttm notes get <id> -o ke-hoach.md`. Định dạng chi tiết:
[Notee-flavored Markdown](notee-markdown.md).

## 6. Thêm MCP vào Claude Code

```sh
ttm mcp install --client claude-code
```

```text
Claude Code now starts `/Users/an/.cargo/bin/ttm-mcp` as MCP server `ttm`.
Restart the client to load it. Sign in first with `ttm login` if you have not.
```

Lệnh chạy `claude mcp add --scope user ttm -- /Users/an/.cargo/bin/ttm-mcp`. Mở lại Claude Code, gõ `/mcp` để thấy máy chủ
`ttm`, rồi thử hỏi:

> Tìm page "Kế hoạch quý 4" trong Notee và tóm tắt ba ý chính.

Claude gọi `notee-search` rồi `notee-fetch` bằng token của bạn. Ứng dụng khác (Claude Desktop, Cursor, Codex, VS Code) và
MCP từ xa: xem [Kết nối MCP](mcp.md).

## Tiếp theo

- [Notee](notee.md): sửa page, đề xuất sửa, truy vấn database, comment.
- [Factory](factory.md): liên kết repo với dự án, đồng bộ kế hoạch, luật cho agent.
- [Runner](runner.md): để agent làm task trên máy bạn.
- Lỗi gì đó? `ttm doctor` rồi xem [Xử lý sự cố](xu-ly-su-co.md).
