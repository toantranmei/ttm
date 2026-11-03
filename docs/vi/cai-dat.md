---
title: Cài đặt ttm
description: Cài ttm và ttm-mcp trên macOS, Linux và Windows qua script, Homebrew, npm hoặc Cargo.
order: 1
---

# Cài đặt

`ttm` (dòng lệnh) và `ttm-mcp` (máy chủ MCP cho AI) phát hành cùng phiên bản. Chọn một kênh:

## macOS, Linux: script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
```

## Windows: PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.ps1 | iex"
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.ps1 | iex"
```

Bộ cài MSI cũng có trên trang Releases; Windows SmartScreen có thể cảnh báo vì binary chưa ký số, chọn **More info → Run anyway**.

## Homebrew

```sh
brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp
```

## npm

```sh
npm install -g @toantranmei/cli
npx @toantranmei/mcp   # chạy máy chủ MCP không cần cài
```

## Cargo

```sh
cargo install ttm-cli ttm-mcp
```

## Kiểm tra

```sh
ttm --version
ttm doctor
```

## Binary chưa ký số

Các kênh trên không gắn cờ quarantine nên chạy bình thường. Nếu tải file bằng trình duyệt trên macOS và bị Gatekeeper
chặn:

```sh
xattr -d com.apple.quarantine ./ttm ./ttm-mcp
```

Mỗi bản phát hành có SHA-256 và attestation của GitHub. Kiểm tra bằng tay:

```sh
gh attestation verify ttm-cli-aarch64-apple-darwin.tar.xz --repo toantranmei/ttm
```

## Cập nhật

```sh
ttm self-update          # tải bản mới, kiểm SHA-256 (và attestation nếu có gh)
ttm self-update --check  # chỉ kiểm tra
```

Cài qua Homebrew hoặc npm thì cập nhật bằng `brew upgrade` / `npm update -g`.
