---
title: Đăng nhập
description: Đăng nhập ttm bằng trình duyệt, mã thiết bị hoặc token cá nhân; chọn máy chủ và profile.
order: 2
---

# Đăng nhập

```sh
ttm login
```

ttm mở trình duyệt tới trang đồng ý (`/ket-noi/cli`), bạn chọn workspace và quyền rồi bấm **Cho phép**. Trình duyệt quay
về một trang cục bộ (`http://127.0.0.1:<cổng>/callback`) báo "Đã đăng nhập, quay lại terminal". Luồng dùng PKCE, hết hạn
sau 5 phút.

## Máy không có trình duyệt (SSH, container)

```sh
ttm login --device
```

ttm in một mã 8 ký tự dạng `XXXX-XXXX`. Mở `<máy chủ>/activate` trên điện thoại hoặc máy khác, nhập mã và cho phép.

## Token có sẵn

Tạo token cá nhân ở **Tài khoản → Kết nối** (`/me/ket-noi`), rồi:

```sh
ttm login --token ttm_pat_...
echo "$TTM_PAT" | ttm login --token -
```

## Token được lưu ở đâu

| Thứ tự | Nguồn |
| --- | --- |
| 1 | Biến môi trường `TTM_TOKEN` (CI, máy không có keyring) |
| 2 | Keyring của hệ điều hành (dịch vụ `ttm`, tài khoản `<profile>@<máy chủ>`) |
| 3 | `~/.config/ttm/credentials.toml` (Windows: `%APPDATA%\ttm`), quyền 0600 |

Token đăng nhập sống 30 ngày. Khi hết hạn, ttm tự gia hạn bằng refresh token nếu có, không thì yêu cầu `ttm login` lại.

## Máy chủ và profile

Thứ tự chọn máy chủ: `--host` > `TTM_HOST` > profile > `https://toantranmei.com`.

```sh
ttm --profile dev login --host http://localhost:3000
ttm --profile dev whoami
```

Profile nằm trong `~/.config/ttm/config.toml`:

```toml
default_profile = "dev"

[profiles.dev]
host = "http://localhost:3000"
workspace = "<id workspace>"
```

## Đăng xuất

```sh
ttm logout
```

Thu hồi token trên máy chủ rồi xoá khỏi máy. Xem và thu hồi mọi token, app đã kết nối ở `/me/ket-noi`.
