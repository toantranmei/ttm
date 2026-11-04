---
title: Cấu hình dạng code
description: Thư mục .ttm với workflow, luật Tự động hoá, agent và skill trong repo; kiểm cục bộ, change request theo loại, xung đột và ttm cr.
order: 20
section: guides
---

Từ 0.2.0, ngoài kế hoạch (`plan/`), `ttm sync` còn đồng bộ **cấu hình** của dự án dưới dạng file trong repo: workflow,
luật Tự động hoá, agent và skill. Web vẫn là nguồn gốc: sửa ở repo chỉ thành **change request** (CR) chờ duyệt, không bao
giờ ghi đè. Tính năng này chỉ bật khi server báo `ascode_v2` trong `discovery.features`; server cũ vẫn chỉ đồng bộ kế
hoạch như 0.1.

## Cấu trúc `.ttm/`

```text
.ttm/
  project.toml   commit: dự án liên kết, rev đã đồng bộ
  plan/          commit: kế hoạch dạng Markdown
  workflow.yaml  commit: workflow của dự án
  rules/         commit: luật Tự động hoá, mỗi luật một file <slug>.yaml
  agents/        commit: agent, mỗi agent một file <handle>.yaml (không bao giờ có secret)
  skills/        commit: skill, mỗi skill một thư mục <slug>/ có SKILL.md
  base/          bỏ qua: kế hoạch ở lần đồng bộ trước (tổ tiên chung khi so ba chiều)
  base-config/   bỏ qua: cấu hình ở lần đồng bộ trước
  cache/         bỏ qua: luật cho hook, bản sao bị thay, config_base.json
```

`ttm sync` tải bundle với `?include=config,skills`, ghi các thư mục trên và lưu `config_base` (phiên bản cấu hình của
server) để gửi lại cùng CR.

## Kiểm cục bộ trước khi gửi

Mọi file cấu hình **đã sửa** được kiểm trước; có lỗi thì `ttm sync` dừng, không tạo CR, mã thoát 1:

| Kiểm | Lỗi |
| --- | --- |
| `workflow.yaml`, `rules/*.yaml`, `agents/*.yaml` phải là YAML hợp lệ | `✕ yaml <file>:<dòng> …` |
| `agents/*.yaml` không có khoá giống secret ở bất kỳ cấp nào: `token`, `*_token`, `*secret*`, `*password*`, `api_key`, `*_api_key`, `apikey` | `secret_in_file` |
| `skills/<slug>/` qua cùng luật với `ttm skill push`: có `SKILL.md` + front matter `name`/`description`, ≤ 200 file, ≤ 5 MB, đường dẫn an toàn | `skill_md_missing`, `front_matter`, `path_unsafe`, `too_large`, `too_many_files` |

Secret (token MCP, API key) không bao giờ nằm trong repo: chúng ở MCP gateway của server (xem [Bảo mật](bao-mat.md)) hoặc
biến môi trường của runner (`env:VAR` trong `runner.toml`).

## Change request theo loại

Server tạo một CR cho kế hoạch và một **config request** cho mỗi đối tượng cấu hình đổi:

```text
✓ 2 configuration request(s) created; they apply once approved
KIND      TARGET  ID
────────────────────────
agent     dev     cfg_8f2…
workflow  Aurora  cfg_91a…
Next: ttm cr show cfg_8f2…
```

| Kind | Đối tượng | Duyệt ở đâu |
| --- | --- | --- |
| `plan` | Task, kế hoạch | Web hoặc `ttm cr approve` |
| `workflow` | `workflow.yaml` | Như trên; áp dụng = bản nháp mới của workflow |
| `rule` | `rules/<slug>.yaml` | Như trên |
| `agent` | `agents/<handle>.yaml` | Như trên (cần owner/admin) |
| `skill` | `skills/<slug>/` | Duyệt CR skill = duyệt phiên bản skill đó |

Lệnh: `ttm cr list [--kind … --status …]`, `ttm cr show <id>` (diff từng file), `ttm cr approve <id>`, `ttm cr reject <id>
--note "…"`. Xem [Tham chiếu lệnh](lenh.md).

## Xung đột

Cùng một trường đổi ở cả repo và web → `409` với danh sách `{kind, target, path, base, local, server}`; `ttm sync` in bảng
xung đột, không gửi gì, mã thoát 1. Giải như xung đột kế hoạch: sửa file rồi `ttm sync` lại, hoặc `ttm sync --resolve` để
agent cục bộ gộp ba phiên bản (file YAML dùng cùng cơ chế so ba chiều).

## "Rules" của AGENTS.md khác `.ttm/rules/`

| | `ttm rules apply` | `.ttm/rules/` |
| --- | --- | --- |
| Là gì | Luật cho **agent cục bộ** (khối trong `AGENTS.md`/`CLAUDE.md`, hook `ttm hook pre-bash`/`stop`) | Luật **Tự động hoá** của dự án ("khi … nếu … thì …") chạy trên server |
| Ai sửa | Server sinh, `ttm rules apply` ghi | Bạn sửa file, gửi qua `ttm sync` thành CR `rule` |
| Xem | `ttm rules apply` | `ttm automation list`, `show`, `test` |
