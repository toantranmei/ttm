---
title: Cài đặt
description: Cài ttm và ttm-mcp trên macOS, Linux và Windows, kiểm tra file tải về, cập nhật, gỡ cài đặt và bật gợi ý lệnh.
order: 2
section: start
---

`ttm` (dòng lệnh) và `ttm-mcp` (máy chủ MCP) là **hai gói riêng**, cùng phiên bản. Cài `ttm` trước; cài thêm `ttm-mcp` nếu
bạn dùng ứng dụng AI trên máy (Claude Code, Claude Desktop, Cursor, Codex…) hoặc chạy runner.

## Nền tảng hỗ trợ

| Hệ điều hành | Kiến trúc (target) | Kênh |
| --- | --- | --- |
| macOS | Apple Silicon (`aarch64-apple-darwin`), Intel (`x86_64-apple-darwin`) | script, Homebrew, npm |
| Linux | x64, arm64; glibc (`*-unknown-linux-gnu`) và musl (`*-unknown-linux-musl`) | script, Homebrew, npm |
| Windows | x64 (`x86_64-pc-windows-msvc`) | PowerShell, MSI, npm |

> **Lưu ý:** không có bản Windows on ARM. Windows 11 trên ARM chạy bản x64 qua giả lập có sẵn của hệ điều hành; cài như
> Windows x64.

## Chọn một kênh

| Kênh | `ttm` | `ttm-mcp` | Cập nhật |
| --- | --- | --- | --- |
| Script (macOS, Linux) | `ttm-cli-installer.sh` | `ttm-mcp-installer.sh` | `ttm self-update` |
| PowerShell (Windows) | `ttm-cli-installer.ps1` | `ttm-mcp-installer.ps1` | `ttm self-update` |
| Homebrew (macOS, Linux) | `toantranmei/tap/ttm` | `toantranmei/tap/ttm-mcp` | `brew upgrade` |
| npm (mọi nền tảng) | `@toantranmei/cli` | `@toantranmei/mcp` | `npm update -g` |
| MSI (Windows x64) | `ttm-cli-x86_64-pc-windows-msvc.msi` | `ttm-mcp-x86_64-pc-windows-msvc.msi` | tải MSI mới |
| Mã nguồn | `cargo install --git … ttm-cli` | `cargo install --git … ttm-mcp` | cài lại |

### macOS, Linux: script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
```

Script đặt binary vào `$CARGO_HOME/bin` (mặc định `~/.cargo/bin`) và thêm thư mục đó vào `PATH` trong file khởi động của
shell. Mở terminal mới (hoặc `source ~/.cargo/env`) để dùng ngay. Cài một phiên bản cố định: thay `latest/download` bằng
`download/v0.1.2`.

### Windows: PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.ps1 | iex"
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.ps1 | iex"
```

Binary vào `%USERPROFILE%\.cargo\bin`. Mở cửa sổ PowerShell mới sau khi cài.

### Windows: MSI

Tải `ttm-cli-x86_64-pc-windows-msvc.msi` và `ttm-mcp-x86_64-pc-windows-msvc.msi` ở
[trang Releases](https://github.com/toantranmei/ttm/releases/latest) rồi chạy. Binary chưa ký số nên Windows SmartScreen có
thể cảnh báo: kiểm tra file (mục **Kiểm tra file tải về** bên dưới) rồi chọn **More info → Run anyway**.

### Homebrew

```sh
brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp
```

> **Lưu ý:** Homebrew từ chối cài khi Xcode trên máy đã cũ (thông báo *Your Xcode is too outdated*). Cập nhật Xcode trong
> App Store; nếu không cần Xcode, gỡ nó và chỉ dùng Command Line Tools
> (`sudo rm -rf /Applications/Xcode.app && xcode-select --install`), rồi chạy lại `brew install`.

### npm

```sh
npm install -g @toantranmei/cli @toantranmei/mcp
```

Hai gói npm là lớp bọc: script `postinstall` tải đúng binary cho máy bạn từ GitHub Releases. `@toantranmei/cli` cung cấp
lệnh `ttm`, `@toantranmei/mcp` cung cấp lệnh `ttm-mcp`. Cần Node.js ≥ 14.18.

> **Lưu ý:** npm 11 cảnh báo về script cài đặt của gói cài toàn cục và có thể bỏ qua chúng tuỳ cấu hình. Thiếu script đó
> thì gói không có binary và lệnh `ttm` báo lỗi khi chạy. Cho phép đúng hai gói này:
>
> ```sh
> npm install -g --allow-scripts=@toantranmei/cli,@toantranmei/mcp @toantranmei/cli @toantranmei/mcp
> ```

Chạy máy chủ MCP không cần cài toàn cục:

```sh
npx -y @toantranmei/mcp --help
```

### Từ mã nguồn

Các crate chưa đăng trên crates.io; cài thẳng từ GitHub (cần Rust stable ≥ 1.88, có `git`):

```sh
cargo install --locked --git https://github.com/toantranmei/ttm --tag v0.1.2 ttm-cli
cargo install --locked --git https://github.com/toantranmei/ttm --tag v0.1.2 ttm-mcp
```

Bản tự build trỏ tới `https://toantranmei.com` như bản phát hành. Đổi host mặc định lúc build:
`TTM_DEFAULT_HOST=https://example.com cargo install …`.

## Kiểm tra sau khi cài

```sh
ttm --version
ttm-mcp --version
ttm doctor
```

```text
ttm 0.1.2
ttm-mcp 0.1.2
[ok  ] host       https://toantranmei.com (profile default)
[ok  ] discovery  api_base https://toantranmei.com/api/v1
[ok  ] version    ttm 0.1.2, server API v1
[warn] token      not signed in (ttm login)
[ok  ] keyring    OS keyring available
[ok  ] git        git version 2.50.1
[ok  ] claude     2.1.318 (Claude Code)
[ok  ] ttm-mcp    /Users/an/.cargo/bin/ttm-mcp
```

`[warn] token` là bình thường trước khi [đăng nhập](dang-nhap.md). `git` cần cho `ttm sync` và runner; `claude` (Claude Code)
chỉ cần cho runner và `ttm sync --resolve`.

## Kiểm tra file tải về

Mỗi bản phát hành có:

- file `<archive>.sha256` cạnh mỗi archive và MSI, cộng file `sha256.sum` gom tất cả;
- **GitHub artifact attestation** (Sigstore), chứng minh file được build bởi workflow phát hành của repo
  `toantranmei/ttm`.

SHA-256:

```sh
V=v0.1.2; F=ttm-cli-aarch64-apple-darwin.tar.xz
curl -LO https://github.com/toantranmei/ttm/releases/download/$V/$F
curl -LO https://github.com/toantranmei/ttm/releases/download/$V/$F.sha256
shasum -a 256 "$F"; cat "$F.sha256"   # hai giá trị phải trùng
```

Trên Linux dùng `sha256sum`, trên Windows `Get-FileHash -Algorithm SHA256 <file>`.

Attestation, bằng [GitHub CLI](https://cli.github.com):

```sh
gh attestation verify ttm-cli-aarch64-apple-darwin.tar.xz --repo toantranmei/ttm
```

`ttm self-update` tự làm cả hai bước (attestation chỉ khi máy có `gh`).

## Binary chưa ký số

Binary không được ký số bởi Apple hay Microsoft. Các kênh script, Homebrew, npm, cargo không gắn cờ quarantine nên macOS
chạy bình thường. Nếu bạn tải archive bằng trình duyệt và Gatekeeper chặn:

```sh
xattr -d com.apple.quarantine ./ttm ./ttm-mcp
```

## Cập nhật

| Kênh cài | Lệnh |
| --- | --- |
| Script, PowerShell, tải tay | `ttm self-update` |
| Homebrew | `brew upgrade ttm ttm-mcp` |
| npm | `npm update -g @toantranmei/cli @toantranmei/mcp` |
| MSI | tải và chạy MSI mới |

`ttm self-update` lấy bản phát hành từ GitHub (`api.github.com/repos/toantranmei/ttm/releases`), tải archive đúng target
của binary đang chạy, kiểm SHA-256 (từ chối cài nếu bản phát hành không có checksum), kiểm attestation khi có `gh` (từ chối
nếu kiểm thất bại), rồi thay `ttm` và cả `ttm-mcp` nếu nó nằm cùng thư mục với `ttm`.

```sh
ttm self-update --check           # chỉ báo có bản mới hay không
ttm self-update                   # cài bản mới nhất
ttm self-update --version 0.1.2   # cài đúng phiên bản này (dùng được để hạ cấp)
```

```text
ttm 0.1.2 is up to date.
```

```text
Updated to ttm 0.1.3: /Users/an/.cargo/bin/ttm, /Users/an/.cargo/bin/ttm-mcp
SHA-256 verified; attestation verified.
```

> **Lưu ý:** cài bằng Homebrew hoặc npm thì cập nhật bằng chính trình quản lý gói đó. `ttm self-update` sẽ ghi đè binary
> mà trình quản lý gói đang theo dõi, làm hai bên lệch phiên bản.

Khi server cần phiên bản mới hơn (`min_cli` trong discovery), mọi lệnh in cảnh báo
`the server needs ttm >= X (this is Y); run ttm self-update` và API có thể trả `426 upgrade_required`.

## Gỡ cài đặt

1. Đăng xuất để thu hồi token trên server và xoá khỏi keyring: `ttm logout` (lặp lại với từng `--profile` bạn dùng).
2. Nếu đã cài dịch vụ runner, gỡ nó trước (xem [Runner](runner.md), mục **Gỡ dịch vụ**).
3. Gỡ chương trình:

   | Kênh | Lệnh |
   | --- | --- |
   | Script | `rm ~/.cargo/bin/ttm ~/.cargo/bin/ttm-mcp` |
   | PowerShell | `Remove-Item "$env:USERPROFILE\.cargo\bin\ttm.exe", "$env:USERPROFILE\.cargo\bin\ttm-mcp.exe"` |
   | Homebrew | `brew uninstall ttm ttm-mcp` |
   | npm | `npm uninstall -g @toantranmei/cli @toantranmei/mcp` |
   | MSI | Settings → Apps → gỡ `ttm-cli` và `ttm-mcp` |
   | cargo | `cargo uninstall ttm-cli ttm-mcp` |

4. Xoá cấu hình và bộ đệm nếu muốn: `~/.config/ttm`, `~/.cache/ttm` (Windows: `%APPDATA%\ttm`, `%LOCALAPPDATA%\ttm`).
   Worktree của runner nằm ở `~/.ttm/worktrees` hoặc thư mục bạn đã cấu hình.

## PATH

`ttm mcp install` ghi **đường dẫn tuyệt đối** của `ttm-mcp` (file nằm cạnh `ttm`, nếu không có thì tìm trên `PATH`) vào
cấu hình của ứng dụng AI, vì ứng dụng đồ hoạ thường không thấy `PATH` của shell. Nếu `ttm doctor` báo
`ttm-mcp not found on PATH`, thêm thư mục cài vào `PATH`:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc   # hoặc ~/.bashrc
```

```powershell
[Environment]::SetEnvironmentVariable("Path", "$env:USERPROFILE\.cargo\bin;" + [Environment]::GetEnvironmentVariable("Path", "User"), "User")
```

## Gợi ý lệnh (shell completion)

`ttm completion <shell>` in script gợi ý cho `bash`, `zsh`, `fish`, `powershell`, `elvish`. Lệnh này không đọc cấu hình nên
chạy được cả khi `config.toml` hỏng.

```sh
# zsh: thêm `fpath=(~/.zfunc $fpath)` vào ~/.zshrc trước `compinit`
mkdir -p ~/.zfunc && ttm completion zsh > ~/.zfunc/_ttm

# bash
mkdir -p ~/.local/share/bash-completion/completions
ttm completion bash > ~/.local/share/bash-completion/completions/ttm

# fish
ttm completion fish > ~/.config/fish/completions/ttm.fish
```

```powershell
# thêm dòng này vào $PROFILE để giữ lâu dài
ttm completion powershell | Out-String | Invoke-Expression
```
