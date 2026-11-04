---
title: Bảo mật
description: Token được lưu và bảo vệ thế nào, scope tối thiểu, thu hồi, server thấy và không thấy gì, cách ly runner và cách báo lỗ hổng.
order: 15
section: more
---

## Token

- **Tiền tố rõ ràng**: `ttm_pat_` (token cá nhân), `ttm_oat_` (token đăng nhập, ứng dụng OAuth), `ttm_ort_` (refresh),
  `ttm_job_` (runner và lần chạy agent). Đừng dán token vào chat, issue, commit hay log CI. Lộ token: thu hồi ngay ở
  `<host>/me/ket-noi`.
- **Server chỉ lưu bản băm** (SHA-256) của token; token gốc chỉ hiện một lần lúc tạo.
- **Lưu trên máy**: keyring của hệ điều hành (macOS Keychain, Windows Credential Manager, Secret Service), dịch vụ `ttm`.
  Không có keyring thì `credentials.toml` quyền **0600**, ghi qua file tạm đã đặt quyền trước khi đổi tên, nên không có lúc
  nào người khác đọc được.
- **Không bao giờ ra log**: mọi chỗ hiển thị token chỉ giữ tiền tố và tối đa 4 ký tự (`ttm_pat_Ab12…`), kể cả ở `-vv`.
- **`TTM_TOKEN`** thắng mọi nguồn khác: tiện cho CI, nhưng biến môi trường có thể bị tiến trình con đọc; chỉ đặt nó trong
  bước cần.
- Token gửi qua header `Authorization: Bearer`. Request có token **không dùng cookie**, nên không bị CSRF.

## Quyền tối thiểu

- Chọn scope vừa đủ: script chỉ đọc Notee cần `workspace:read notee:read`; CI kiểm kế hoạch cần `factory:read`.
- **Giới hạn token vào một workspace** khi tạo PAT hoặc ở ô *Giới hạn trong workspace* lúc cho phép đăng nhập.
- Muốn agent chỉ **đề xuất** sửa, không ghi thẳng: cấp `notee:comment` mà không cấp `notee:write`, hoặc đặt
  `agent_write_mode = suggest` cho workspace.
- Quyền thật luôn là giao của scope và quyền của bạn trên tài nguyên: token không bao giờ làm được nhiều hơn chính bạn.
- Những việc nhạy cảm **không làm được bằng token** dù scope nào: tạo/liệt kê token, chia sẻ, thanh toán, xoá page, tạo
  workspace, quản trị (`token_not_allowed`).

## Thời hạn và thu hồi

- Token đăng nhập CLI sống 7 ngày, tự gia hạn bằng refresh token 90 ngày (PAT 1–365 ngày, ứng dụng OAuth 1 giờ, runner
  12 giờ, job theo thời gian của job).
- Refresh token **xoay vòng** mỗi lần dùng; dùng lại một refresh token cũ (dấu hiệu bị đánh cắp) thu hồi cả họ token.
- `ttm logout` thu hồi trên server rồi mới xoá ở máy.
- Trang `/me/ket-noi` cho thấy lần dùng cuối của từng token; ứng dụng đã kết nối có nút **Ngắt kết nối** thu hồi mọi token
  của ứng dụng đó; tab **Runner** tắt runner từ xa.

## Đăng nhập

- Luồng trình duyệt dùng **PKCE S256** và `state` ngẫu nhiên; trang callback chỉ nghe trên `127.0.0.1`, bỏ qua mọi request
  sai `state`, và chỉ sống tới khi nhận mã (tối đa 5 phút).
- Device flow: mã 8 ký tự sống 10 phút, dùng một lần; trang `/activate` cho bạn xem tên ứng dụng và quyền trước khi cho
  phép.
- Trang cho phép của ứng dụng bên ngoài (MCP từ xa) ghi rõ *Ứng dụng bên ngoài*, hiện nơi trình duyệt sẽ quay về, và
  **cảnh báo khi tên ứng dụng giống ứng dụng chính thức**. Chỉ bấm **Cho phép** khi chính bạn vừa kết nối ứng dụng đó.
- Token OAuth gắn với đúng tài nguyên `https://toantranmei.com/api/v1/mcp`; server từ chối cấp token cho tài nguyên khác.

## Server thấy gì

| Server thấy | Server không thấy |
| --- | --- |
| Mọi request API bạn (hoặc agent của bạn) gửi: page đọc, sửa, truy vấn, comment | Key API hay phiên đăng nhập Claude của bạn |
| Lần dùng cuối của token (thời điểm, IP, User-Agent) | File trên máy không được agent đọc hay đẩy lên |
| Sự kiện của run: suy nghĩ, hành động, lệnh và đường dẫn file agent dùng, kết quả test, số token | Nội dung `config.toml`, keyring, `credentials.toml` |
| Kết quả luật (`rule_check`) từ hook | Repo của bạn, trừ nhánh agent đẩy lên remote của bạn |
| Tên máy, hệ điều hành, phiên bản, harness của runner | |

Prompt của mỗi bước do server ghép và gửi xuống máy bạn để chạy Claude Code; phần logic tạo prompt nằm ở server. Claude Code
nói chuyện trực tiếp với Anthropic bằng tài khoản của bạn, không qua toantranmei.com.

## Runner

- Chỉ **kết nối đi ra** (heartbeat và long-poll), không mở cổng.
- Chỉ làm việc trong **clone bạn khai báo** (`ttm runner config --repo`), kiểm `origin` của clone trùng repo của job;
  không bao giờ clone URL server gửi. Tên nhánh được kiểm trước khi dùng.
- **Mỗi task một worktree, một nhánh**; worktree của bạn không bị đụng. Hook `pre-bash` chặn push thẳng vào nhánh được
  bảo vệ (mặc định nhánh chính của project), `--no-verify` và commit sai quy ước.
- Worktree luôn nằm dưới `worktree_root` của runner: `worktree_hint` của job chỉ là đường dẫn tương đối; đường dẫn tuyệt
  đối, `..` hay symlink dẫn ra ngoài bị từ chối (`invalid_worktree`).
- Token của job chỉ dùng được cho run của nó, nằm trong file cấu hình MCP quyền 0600 và bị xoá khi run xong.
- Agent chạy với **quyền người dùng hệ điều hành của bạn**, giới hạn bởi danh sách tool mà job cho phép. Đây không phải
  sandbox: chỉ chạy runner cho project bạn tin, xem lại pull request trước khi merge, và cân nhắc chạy runner bằng một tài
  khoản hệ điều hành riêng hoặc trong máy ảo.
- Kill switch: nút dừng trên web hoặc `ttm agent stop <run>`; runner nhận lệnh trong ≤ 20 giây.

## Cập nhật an toàn

`ttm self-update` từ chối cài bản không có checksum, kiểm SHA-256 của từng archive, và kiểm **GitHub artifact attestation**
(Sigstore) khi có `gh`; attestation sai thì dừng. Tự kiểm bằng tay: [Cài đặt](cai-dat.md).

Binary chưa được Apple hay Microsoft ký số; các kênh script, Homebrew, npm không gắn cờ quarantine. Mã nguồn mở (MIT) tại
<https://github.com/toantranmei/ttm>.

## Báo lỗ hổng

Gửi email riêng tới **toantran.mei@gmail.com** với mô tả, cách tái hiện và phiên bản (`ttm --version`). **Đừng mở issue
công khai** cho lỗ hổng. Chúng tôi phản hồi trong 72 giờ, báo tiến độ sửa và ghi công bạn trong ghi chú phát hành nếu bạn
muốn.
