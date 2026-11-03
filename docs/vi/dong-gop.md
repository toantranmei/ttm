---
title: Đóng góp
description: Cách build, kiểm thử và gửi pull request cho repo ttm, kể cả sửa tài liệu này.
order: 8
---

# Đóng góp

Repo: <https://github.com/toantranmei/ttm> (MIT). Chào đón báo lỗi, sửa tài liệu và pull request.

```sh
git clone https://github.com/toantranmei/ttm && cd ttm
cargo build --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Commit theo Conventional Commits, tiếng Anh (`feat(cli): ...`, `fix(api): ...`).
- Viết test cạnh code; hành vi HTTP dùng máy chủ giả `wiremock`.
- Trỏ CLI vào máy chủ dev: `--host http://localhost:3000`.
- Tool MCP mới viết trong `crates/ttm-mcp-tools` trên trait `Backend` để máy chủ MCP từ xa dùng lại.

## Sửa tài liệu

Tài liệu này nằm ở `docs/vi`, `docs/en`, `docs/zh` (Markdown, front-matter `title`, `description`, `order`). Tiếng Việt
là bản chính; khi sửa, cập nhật cả bản dịch hoặc ghi rõ trong pull request là bản dịch còn thiếu.
