---
title: Đăng nhập và token
description: Đăng nhập bằng trình duyệt, mã thiết bị hoặc token cá nhân; scope, thời hạn, nơi lưu token, CI, đăng xuất và nhiều tài khoản.
order: 4
section: guides
---

`ttm` và `ttm-mcp` gọi API bằng một **token Bearer** (`Authorization: Bearer ttm_…`). Có ba cách lấy token:

| Cách | Lệnh | Hợp với |
| --- | --- | --- |
| Trình duyệt trên cùng máy (mặc định) | `ttm login` | Máy cá nhân có trình duyệt |
| Mã thiết bị | `ttm login --device` | SSH, container, máy chủ không có trình duyệt |
| Token cá nhân có sẵn | `ttm login --token ttm_pat_…` hoặc biến `TTM_TOKEN` | CI, script, token giới hạn quyền |

## Đăng nhập bằng trình duyệt

```sh
ttm login
```

1. `ttm` mở một cổng ngẫu nhiên trên `127.0.0.1`, tạo cặp PKCE (S256) và một `state` ngẫu nhiên.
2. Trình duyệt mở trang cho phép `<host>/ket-noi/cli` (in cả URL ra stderr để bạn tự mở nếu cần).
3. Trang hiện tên ứng dụng (**ttm CLI**, “Ứng dụng chính thức của toantranmei”), danh sách quyền, và ô **Giới hạn trong
   workspace** khi bạn ở nhiều workspace (để trống = mọi workspace). Bấm **Cho phép**.
4. Trình duyệt quay về `http://127.0.0.1:<cổng>/callback`; `ttm` kiểm `state`, đổi mã lấy token rồi gọi `/me` để kiểm tra.

```text
Signed in to https://toantranmei.com as An Nguyễn <an@example.com>.
Profile `default`, token stored in keyring.
```

Bạn có **5 phút** để hoàn tất. Bấm **Từ chối** thì lệnh dừng với `sign-in was refused in the browser (access_denied)`.

| Cờ | Việc |
| --- | --- |
| `--no-browser` | Không tự mở trình duyệt, chỉ in URL (cũng bật bằng biến `TTM_NO_BROWSER`) |
| `--scope "<danh sách>"` | Xin scope khác mặc định, cách nhau bằng dấu cách |

> **Lưu ý:** trang callback chỉ nghe trên `127.0.0.1` của máy chạy `ttm`. Khi SSH vào máy khác, trình duyệt trên laptop
> không với tới được; dùng `ttm login --device`.

## Đăng nhập bằng mã thiết bị

```sh
ttm login --device
```

```text

On any device, open https://toantranmei.com/activate and enter the code:

    BCDF-GHJK

```

Mở `https://toantranmei.com/activate` trên điện thoại hoặc máy khác, đăng nhập web nếu cần, nhập mã (không phân biệt hoa
thường, có hay không có gạch nối đều được), xem quyền rồi bấm **Cho phép**. `ttm` hỏi server mỗi 5 giây (tăng thêm 5 giây
nếu server báo `slow_down`) và lưu token khi bạn cho phép.

- Mã gồm 8 chữ cái phụ âm (`BCDFGHJKLMNPQRSTVWXZ`), dạng `XXXX-XXXX`, sống **10 phút**. Hết hạn:
  `the code expired before it was approved; run ttm login --device again`.
- Bấm **Từ chối** trên web: `device login failed: OAuth error access_denied`.
- `--device` không dùng chung được với `--token`.

## Token cá nhân (PAT)

Tạo token ở **Tài khoản → Kết nối** (`<host>/me/ket-noi`, tab **Token**): đặt tên, chọn quyền, chọn workspace (hoặc *Mọi
workspace*) và thời hạn **7, 30, 90 hoặc 365 ngày** (mặc định 30). Token `ttm_pat_…` chỉ hiện **một lần**; sao chép ngay.

Lưu token vào máy:

```sh
ttm login --token ttm_pat_AbCd...
printf %s "$MY_PAT" | ttm login --token -     # đọc từ stdin, không lộ trong lịch sử shell
```

`ttm` gọi `/me` để kiểm tra trước khi lưu:

- token sai, hết hạn hoặc đã thu hồi: `the server rejected this token (… invalid_token, HTTP 401)`, không lưu gì;
- token hợp lệ nhưng thiếu `workspace:read`: vẫn lưu, in `Token saved for <host>.`;
- chuỗi không bắt đầu bằng `ttm_`: cảnh báo nhưng vẫn thử.

Các lệnh `ttm token` chỉ **mở trang web** (API không cho token tạo hay liệt kê token, để một token bị lộ không tự sinh
thêm token):

```sh
ttm token list                                            # mở <host>/me/ket-noi
ttm token create --name "CI" --scope notee:read,workspace:read --expires 90d
ttm token revoke                                          # mở trang để thu hồi
```

`create` gửi kèm tên, scope và thời hạn (`--expires` nhận `1`–`365` ngày, viết `30` hoặc `30d`) làm tham số của URL; kiểm
tra lại các ô trên trang trước khi bấm **Tạo token**.

## Scope

| Scope | Cho phép | Bao gồm |
| --- | --- | --- |
| `workspace:read` | Xem hồ sơ (`/me`), workspace, thành viên, agent, thông báo | |
| `notee:read` | Đọc cây page, tìm kiếm, đọc page dạng Markdown, phiên bản, đề xuất sửa, schema và truy vấn database, đọc comment, tải file, xuất | |
| `notee:comment` | Tạo, trả lời, sửa, xoá, đóng comment; **đề xuất sửa** page (`mode: suggest`) | `notee:read` |
| `notee:write` | Tạo, sửa, di chuyển, nhân bản page; khôi phục phiên bản; chấp nhận/từ chối đề xuất; tạo và sửa database, ghi và xoá dòng; tải file lên, nhập | `notee:read`, `notee:comment` |
| `factory:read` | Xem project, task, kế hoạch (export), change request, luật, sự kiện của run; gửi kết quả luật (`rule_check`) | |
| `factory:write` | Tạo project, sửa task, chuyển trạng thái, duyệt, gửi change request, tạo task con | `factory:read` |
| `factory:run` | Đăng ký và chạy runner, nhận job, gửi sự kiện, kết thúc run, `ttm agent run`, tạo task con của chính run | `factory:read` |
| `factory:admin` | Quản trị cấu hình Factory qua token: workflow, Tự động hoá, agent, skill, MCP server, cài mẫu; luôn qua **xác nhận hai bước** khi đi từ MCP. Token của job (agent đang chạy) **không bao giờ** có scope này | `factory:write` |
| `offline_access` | Cấp refresh token (giữ đăng nhập) | |

`ttm login` mặc định xin **tất cả**: `workspace:read notee:read notee:write notee:comment factory:read factory:write
factory:run factory:admin offline_access`. Thu hẹp bằng `--scope`, ví dụ chỉ đọc Notee:

```sh
ttm login --scope "workspace:read notee:read"
```

Quyền thật = **giao** của scope và quyền của bạn trên từng tài nguyên: token có `notee:write` vẫn không sửa được page bạn
chỉ có quyền xem. Token giới hạn trong một workspace chỉ thấy workspace đó (tìm kiếm ở workspace khác trả rỗng).

Một số việc **không bao giờ** làm được bằng token, chỉ làm trên web: quản lý token, chia sẻ (cấp quyền, link), thanh toán,
xoá page, tạo hay sửa workspace, thêm agent. Gọi chúng bằng token trả `403 token_not_allowed`. Thiếu scope trả
`403 insufficient_scope` kèm scope cần (xem [Mã lỗi](ma-loi.md)).

## Thời hạn

| Token | Tiền tố | Sống |
| --- | --- | --- |
| Đăng nhập CLI (`ttm login`, `--device`) | `ttm_oat_` | 7 ngày, tự gia hạn |
| Refresh token của CLI | `ttm_ort_` | 90 ngày, xoay vòng: mỗi lần gia hạn cấp refresh token mới với 90 ngày mới |
| Token cá nhân | `ttm_pat_` | 1–365 ngày do bạn chọn (mặc định 30), không gia hạn |
| Ứng dụng MCP từ xa (Claude.ai, ChatGPT…) | `ttm_oat_` | 1 giờ; refresh token 90 ngày (xoay vòng) chỉ khi ứng dụng xin `offline_access` |
| Runner | `ttm_job_` | 12 giờ, runner tự đăng ký lại trước khi hết hạn |
| Một lần chạy agent | `ttm_job_` | Theo thời gian của job |

Khi token đã lưu hết hạn và có refresh token, `ttm` và `ttm-mcp` tự xin token mới rồi lưu lại. Không gia hạn được thì lệnh
báo `the saved token has expired; run ttm login`. Token lấy từ `TTM_TOKEN` không bao giờ được gia hạn tự động.

> **Lưu ý:** token truy cập của CLI sống 7 ngày và được gia hạn tự động; bạn vẫn đăng nhập chừng nào còn dùng `ttm` (hoặc
> `ttm-mcp`) ít nhất mỗi 90 ngày. Bỏ không quá 90 ngày thì refresh token hết hạn và cần `ttm login` lại. (Thời hạn này
> đi cùng thay đổi phía server sắp phát hành; trước đó cả hai token sống 30 ngày.)

## Token được lưu ở đâu

Khi **đọc**, `ttm` và `ttm-mcp` dùng nguồn đầu tiên có token:

| Thứ tự | Nguồn | Ghi chú |
| --- | --- | --- |
| 1 | Biến `TTM_TOKEN` | Thắng mọi thứ, kể cả `--profile`. Không gia hạn. |
| 2 | Keyring của hệ điều hành | Dịch vụ `ttm`, tài khoản `<profile>@<host>` (ví dụ `default@https://toantranmei.com`). macOS Keychain, Windows Credential Manager, Secret Service trên Linux. |
| 3 | `credentials.toml` | `~/.config/ttm/credentials.toml` (Windows `%APPDATA%\ttm\credentials.toml`), quyền 0600 |

Khi **ghi**, `ttm` thử keyring trước (và xoá bản cũ trong file nếu có). Keyring không dùng được (Linux không có Secret
Service, container) hoặc bị tắt bằng `TTM_NO_KEYRING=1` thì token vào `credentials.toml`:

```toml
[credentials."default@https://toantranmei.com"]
access_token = "ttm_oat_…"
refresh_token = "ttm_ort_…"
expires_at = "2026-12-04T05:00:00Z"
scope = "workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run factory:admin offline_access"
```

`ttm status` cho biết token đang lấy từ đâu và chỉ hiện vài ký tự đầu:

```text
token:     ttm_oat_Ab12… from keyring, expires 2026-12-04 5:00:00.0 +00:00:00
```

Token của runner được lưu riêng, tài khoản `<profile>#runner@<host>`, và không bao giờ đọc từ `TTM_TOKEN`.

## CI và máy không có keyring

Tạo PAT với scope tối thiểu, lưu vào secret của CI rồi đặt `TTM_TOKEN`; không cần `ttm login`:

```yaml
# GitHub Actions
- name: Kế hoạch khớp với server
  env:
    TTM_TOKEN: ${{ secrets.TTM_TOKEN }}   # PAT có factory:read
  run: ttm sync --check
```

```sh
export TTM_TOKEN=ttm_pat_…
ttm whoami --json
```

Khi `TTM_TOKEN` đang đặt, `ttm login` cảnh báo `TTM_TOKEN is set and will keep overriding the token saved by this login`.

## Đăng xuất và thu hồi

```sh
ttm logout
```

```text
Signed out of https://toantranmei.com (profile `default`).
```

`ttm logout` thu hồi token truy cập và refresh token trên server (RFC 7009, `/api/v1/oauth/revoke`), rồi xoá token khỏi
keyring và `credentials.toml`. Server không xác nhận được (mất mạng) thì token vẫn bị xoá khỏi máy và lệnh cảnh báo; khi
đó thu hồi thêm trên web. Token lấy từ `TTM_TOKEN` thì phải tự bỏ biến đó.

Trên `<host>/me/ket-noi`:

- tab **Token**: mọi token còn hạn (PAT, đăng nhập CLI, job runner), lần dùng cuối, nút **Thu hồi**;
- tab **Ứng dụng đã kết nối**: ứng dụng OAuth (Claude.ai, ChatGPT…), nút **Ngắt kết nối** thu hồi cả họ token;
- tab **Runner**: máy đang chạy `ttm runner`.

Thu hồi có hiệu lực ngay: mọi request sau đó trả `401 invalid_token`.

## Token cũ và `factory:admin`

Từ 0.2.0, `ttm login` xin thêm `factory:admin` (lệnh `ttm agent`, `ttm skill review`, `ttm workflow publish`…). Token tạo
trước đó thiếu scope này: lệnh quản trị trả `insufficient_scope` với `Fix:` gợi ý đăng nhập lại. Chạy `ttm login` một lần
là đủ.

## Huy hiệu Mei-er

Khi bạn là thành viên Mei-er, `ttm login` và `ttm whoami` in thêm một dòng màu vàng gold:

```text
✦ Mei-er since 2026-11  Thank you for being a Mei-er. You keep this going.
```

| Hạng | Dòng cảm ơn |
| --- | --- |
| Supporter (`· Supporter`) | `Thank you for supporting toantranmei.` |
| Mei-er (`✦ Mei-er`) | `Thank you for being a Mei-er. You keep this going.` |
| Sponsor (`✦✦ Sponsor`) | `Thank you, Sponsor. This work exists because of you.` |

Đúng ngày kỷ niệm tham gia (cùng tháng–ngày, từ năm thứ nhất) có thêm `Happy <n>-year Mei-er anniversary!`. Với `--plain`,
huy hiệu là `[Mei-er]`. Với `--json`, `whoami` và `login` có thêm khoá `meta.membership` (`{tier, status, since, until,
badge}` hoặc `null`). Người chưa là thành viên chỉ thấy, **duy nhất** ở `ttm whoami`, một dòng mờ `Support toantranmei →
https://toantranmei.com/mei-er`; không lệnh nào khác nhắc.

## Nhiều tài khoản, nhiều máy chủ

Mỗi **profile** có host, workspace và token riêng:

```sh
ttm --profile work login                                  # profile "work" trên production
ttm --profile dev login --host http://localhost:3000      # profile "dev" trỏ server local
ttm --profile dev whoami
TTM_PROFILE=work ttm notes search "OKR"
```

`--host` (hoặc `TTM_HOST`) khi đăng nhập được ghi vào profile; lần đầu đăng nhập một profile chưa có host cũng ghi host
đang dùng. Token gắn với cặp (profile, host): cùng profile nhưng đổi `--host` là một token khác. Chọn profile mặc định
trong `config.toml` bằng `default_profile` (xem [Cấu hình](cau-hinh.md)).
