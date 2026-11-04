---
title: Giao diện dòng lệnh
description: Quy ước đầu ra của ttm 0.2: dòng kết quả, Next, bảng glyph trạng thái, huy hiệu, màu, chế độ plain và JSON, bề ngang.
order: 19
section: reference
---

Từ 0.2.0, mọi lệnh của `ttm` in qua một thư viện chung (`ttm-ui`), nên đầu ra giống nhau ở mọi nơi. Lệnh và đầu ra luôn
bằng tiếng Anh; nội dung của bạn (tên task, tên page…) giữ nguyên.

## Mỗi lệnh in theo một mẫu

1. **Dòng đầu là kết quả**: `✓ Signed in as An <an@example.com>`, `! Plan out of sync…`, `2 workspace(s), current: Aurora`.
2. **Chi tiết**: bảng, cặp khoá–giá trị, diff.
3. **Dòng cuối là lệnh gợi ý**: `Next: ttm project init`.

Lỗi in ra stderr theo một mẫu:

```text
✕ updating AUR-12: AUR-12 changed since rev 4.
  rule: rev-conflict  field: rev
  Fix: someone changed it meanwhile: fetch it again (ttm sync) and redo the change
  Help: https://toantranmei.com/help/ttm/ma-loi#rev-conflict
```

`Help:` trỏ tới mục của rule trong [Mã lỗi](ma-loi.md).

## Glyph trạng thái

| Glyph | `--plain` | Nghĩa | Category |
| --- | --- | --- | --- |
| `○` | `todo` | Chưa làm | `inbox`, `refinement`, `ready` |
| `●` | `doing` | Đang làm | `ux-design`, `in-progress` |
| `◐` | `review` | Chờ review / chờ duyệt | `design-review`, `qc`, `approval` (vàng = chờ người) |
| `✓` | `done` | Xong | `done` |
| `✕` | `blocked` | Lỗi / bị chặn | `blocked` |
| `⏸` | `paused` | Tạm dừng | `paused` |

Kết quả kiểm tra (`ttm doctor`, `ttm status`): `✓` ok, `!` cảnh báo, `✕` lỗi (`--plain`: `ok`, `warn`, `FAIL`).
Callout: `✓` thành công, `!` cảnh báo, `✕` lỗi, `›` ghi chú (`--plain`: `OK:`, `WARNING:`, `ERROR:`, `NOTE:`).

## Huy hiệu

| Huy hiệu | `--plain` | Nghĩa |
| --- | --- | --- |
| `🤖` | `[agent]` | Agent |
| `👤` | `[person]` | Người |
| `· Supporter` | `[Supporter]` | Thành viên Supporter |
| `✦ Mei-er` | `[Mei-er]` | Thành viên Mei-er |
| `✦✦ Sponsor` | `[Sponsor]` | Thành viên Sponsor |

Huy hiệu thành viên luôn màu vàng gold (màu này chỉ dành cho Mei-er). Agent thuộc workspace có chủ là thành viên mang huy
hiệu của chủ.

## Màu

| Token | Màu | Dùng cho |
| --- | --- | --- |
| teal | `#5bb7a8` | Thành công, đang chạy |
| yellow | `#f2b84b` | Chờ, cảnh báo |
| coral | `#e97962` | Lỗi, bị chặn |
| purple | `#b597d6` | Review, thiết kế |
| blue | `#55a4cf` | Thông tin, link, lệnh gợi ý |
| gold | `#d4a72c` | Chỉ Mei-er |
| ink | đậm, màu chữ của terminal | Chữ nhấn |
| muted | mờ | Chữ phụ |

Terminal hỗ trợ truecolor (`COLORTERM=truecolor|24bit`, Windows Terminal) dùng đúng mã màu; `TERM=*256color*` dùng bảng
256 màu gần nhất; còn lại dùng 16 màu cơ bản.

## Chế độ

| Khi | Đầu ra |
| --- | --- |
| stdout là terminal | Màu, khung, glyph, emoji; spinner trên stderr |
| Pipe, `NO_COLOR` (khác rỗng), `TERM=dumb` | Không màu, vẫn có khung và glyph |
| `CLICOLOR_FORCE=1` | Ép màu kể cả khi pipe (thua `NO_COLOR`) |
| `--plain` hoặc `TTM_PLAIN=1` | Không màu, không khung, không glyph, không emoji: chữ thay glyph (cho log) |
| `--json` | Chỉ JSON trên stdout (hình dạng 0.1.6, chỉ thêm khoá mới); lỗi vẫn ra stderr |

Bảng không bao giờ xuống dòng: ô dài bị cắt bằng `…` theo bề rộng hiển thị (chữ Việt có dấu, chữ Hán và emoji tính đúng).
Đường dẫn và URL trong dòng chi tiết không bị cắt, để copy được.

## Bề ngang

`TTM_COLUMNS` > `COLUMNS` > kích thước terminal > 80, kẹp trong 40–200 cột.

## Wizard và `--yes`

Lệnh thiết lập (`ttm project init`, `ttm agent create`, `ttm mcp-server add`…) hỏi từng câu trong terminal; **mọi câu
hỏi có cờ tương ứng**. Cờ đã có thì không hỏi. `-y/--yes` lấy giá trị mặc định và tự xác nhận; thiếu cờ bắt buộc thì lỗi
`missing_flag` (mã thoát 2). Không phải terminal mà không có `--yes` → `missing_flag` ("pass --workflow or run in a
terminal") hoặc `confirmation_required`. Sau wizard, `ttm` in ra stderr lệnh tương đương đủ cờ để dán vào script:

```text
Equivalent: ttm project init --name 'Web todo' --key WEB --workflow balanced --agents create --runner this --yes
```

Secret không bao giờ là giá trị cờ: dùng `--secret-env TEN_BIEN` (đọc biến môi trường) hoặc nhập ẩn trong terminal.

## Màn hình "sống"

Chỉ `ttm inbox --watch` và `ttm agent logs -f` in liên tục (mỗi sự kiện một dòng, không TUI toàn màn hình). Với
`--json`, hai lệnh này in NDJSON (mỗi dòng một object).
