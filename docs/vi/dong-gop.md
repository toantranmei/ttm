---
title: Đóng góp
description: Build, kiểm thử, định dạng mã, sửa tài liệu và quy trình phát hành của repo ttm.
order: 17
section: more
---

Mọi đóng góp đều được chào đón: báo lỗi, sửa tài liệu, pull request. Repo: <https://github.com/toantranmei/ttm> (MIT).

## Chuẩn bị

- Rust stable (`rust-toolchain.toml` kèm `rustfmt`, `clippy`), edition 2024, tối thiểu 1.88.
- `git`. Để thử runner: Claude Code (`claude`); để thử PR: `gh`.

```sh
git clone https://github.com/toantranmei/ttm && cd ttm
cargo build --workspace
target/debug/ttm --help
target/debug/ttm-mcp --help
```

Trỏ CLI vào server local: `target/debug/ttm --host http://localhost:3000 login`. Tách hẳn khỏi cấu hình thật của bạn:

```sh
export TTM_CONFIG_DIR=/tmp/ttm-dev/config TTM_CACHE_DIR=/tmp/ttm-dev/cache TTM_HOME=/tmp/ttm-dev/home TTM_NO_KEYRING=1
```

## Cấu trúc

| Crate | Vai trò |
| --- | --- |
| `crates/ttm-api` | Client HTTP có kiểu cho `/api/v1`: thử lại, User-Agent, `Deprecation`, 426, problem+json |
| `crates/ttm-core` | Cấu hình, profile, chọn host, discovery, lưu token, file dự án, worktree, merge 3 phía |
| `crates/ttm-mcp-tools` | Tool MCP trên trait `Backend`. Tool mới đặt ở đây để MCP từ xa của server dùng lại |
| `crates/ttm-runner` | Runner: kết nối, executor, harness, kiểm tra repo, merge queue, dịch vụ |
| `crates/ttm-cli` | Binary `ttm` (định nghĩa lệnh bằng clap trong `src/cli.rs`) |
| `crates/ttm-mcp` | Binary `ttm-mcp` |
| `docs/vi`, `docs/en`, `docs/zh` | Nguồn trung tâm trợ giúp |
| `mcpb/` | Manifest gói Claude Desktop |

## Trước khi mở pull request

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Mỗi pull request một thay đổi logic; commit theo Conventional Commits bằng tiếng Anh (`feat(cli): …`, `fix(api): …`).
- Test đặt cạnh mã: unit test trong module, hành vi HTTP bằng `wiremock` trong `tests/` của từng crate.
- Không bao giờ ghi token ra log; chỗ nào in thông tin xác thực phải che bằng `mask`.
- Thêm hay đổi lệnh, cờ, tool: cập nhật `docs/vi` (ngôn ngữ chính), và `docs/en`, `docs/zh` hoặc ghi rõ trong PR là bản
  dịch đang chờ.

CI (`.github/workflows/ci.yml`) chạy trên mọi push và pull request: `cargo fmt --check`, clippy với `-D warnings`, test
trên Linux, macOS và Windows (với `TTM_NO_KEYRING=1`, `TTM_NO_BROWSER=1`), và kiểm bản sao đặc tả Markdown. Workflow
`build-matrix.yml` build đủ 7 target khi PR đổi cấu hình build, hằng tuần, hoặc chạy tay.

## Sửa tài liệu

Mỗi file Markdown trong `docs/<ngôn ngữ>/` có front matter:

```yaml
---
title: Tiêu đề ngắn
description: Một câu mô tả.
order: 4
section: guides
---
```

| Khoá | Ý nghĩa |
| --- | --- |
| `title` | Tiêu đề, trang web dùng làm H1 (đừng viết `#` H1 trong thân) |
| `description` | Một câu, dùng cho mô tả trang |
| `order` | Thứ tự toàn cục trong mục lục |
| `section` | `start`, `guides`, `reference` hoặc `more` |

Tên file là slug (ASCII, tiếng Việt không dấu, nối bằng `-`); giữ nguyên tên giữa các ngôn ngữ. Liên kết giữa các trang
viết `[Cài đặt](cai-dat.md)`. Trong bảng, viết `\|` thay cho `|`.

Đặc tả Notee-flavored Markdown mà `ttm-mcp` phục vụ ở `notee://spec/markdown` được nhúng từ
`crates/ttm-mcp-tools/spec/notee-markdown.md`, bản sao của `docs/en/notee-markdown.md`; CI chặn khi hai file khác nhau
(`cmp`). Sửa một thì chép sang file kia.

## Phát hành

Phát hành do người bảo trì làm bằng [cargo-dist](https://opensource.axo.dev/cargo-dist/) 0.34 (`dist-workspace.toml`,
`.github/workflows/release.yml`).

1. Đổi `version` trong `[workspace.package]` và các mục `ttm-*` trong `[workspace.dependencies]` của `Cargo.toml`, chạy
   `cargo build` để cập nhật `Cargo.lock`, cập nhật [Thay đổi](thay-doi.md).
2. Commit `chore: release X.Y.Z`, rồi tag và đẩy:

   ```sh
   git tag vX.Y.Z && git push origin main vX.Y.Z
   ```

3. Workflow `Release` (chạy khi có tag dạng phiên bản):
   - `plan`: dist lập kế hoạch phát hành;
   - `build-local-artifacts`: build 7 target với `TTM_DEFAULT_HOST=https://toantranmei.com`, tạo archive, `.sha256`,
     MSI, và **attestation** (`actions/attest-build-provenance`);
   - `build-global-artifacts`: script cài shell/PowerShell, formula Homebrew, gói npm, `sha256.sum`;
   - `host`: tạo GitHub Release với mọi file;
   - `publish-homebrew-formula` (khi biến repo `PUBLISH_HOMEBREW=true`): đẩy `ttm.rb`, `ttm-mcp.rb` vào
     `toantranmei/homebrew-tap` bằng deploy key chỉ-ghi `HOMEBREW_TAP_DEPLOY_KEY`;
   - `publish-npm` (khi `PUBLISH_NPM=true`): `npm publish` `@toantranmei/cli`, `@toantranmei/mcp` bằng **trusted publishing**
     (OIDC, `id-token: write`, npm ≥ 11.5.1): npm tin workflow này của `toantranmei/ttm`, không lưu token nào.
4. Kiểm tra: `ttm self-update --check`, `brew upgrade`, `npm view @toantranmei/cli version`,
   `gh attestation verify <file> --repo toantranmei/ttm`.

Không có bản Windows on ARM: dist build target đó trên Linux, nơi không tạo được MSI (WiX).
