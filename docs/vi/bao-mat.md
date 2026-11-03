---
title: Bảo mật
description: Token, quyền, nơi lưu trữ và cách báo lỗ hổng.
order: 7
---

# Bảo mật

- **Token có tiền tố** `ttm_pat_` (tạo tay), `ttm_oat_` (đăng nhập), `ttm_ort_` (refresh), `ttm_job_` (runner). Đừng dán
  token vào chat, issue hay commit. Lỡ lộ: thu hồi ngay ở `/me/ket-noi`.
- **Quyền (scope)**: `workspace:read`, `notee:read`, `notee:write`, `notee:comment`, `factory:read`, `factory:write`,
  `factory:run`, `offline_access`. Quyền thật là giao của scope và quyền của bạn trên tài liệu. Token có thể giới hạn
  trong một workspace.
- **Hạn**: mặc định 30 ngày. Refresh token xoay vòng; dùng lại refresh cũ sẽ thu hồi cả họ token.
- **Lưu trữ**: keyring của hệ điều hành; nếu không có, file `credentials.toml` quyền 0600. ttm không bao giờ ghi token ra
  log; `ttm status` chỉ hiện vài ký tự đầu.
- **Đăng nhập**: PKCE S256 và `state` ngẫu nhiên; trang callback chỉ nghe ở `127.0.0.1`.
- **Cập nhật**: `ttm self-update` kiểm SHA-256, và attestation GitHub khi có `gh`.
- **Mã nguồn mở** (MIT): logic điều phối nằm ở máy chủ; `ttm` chỉ xác thực, gọi API và chạy việc trên máy bạn.

## Báo lỗ hổng

Gửi email riêng tới `toantran.mei@gmail.com` (đừng mở issue công khai). Chúng tôi phản hồi trong 72 giờ.
