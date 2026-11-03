---
title: 安装 ttm
description: 通过脚本、Homebrew、npm 或 Cargo 在 macOS、Linux 和 Windows 上安装 ttm 与 ttm-mcp。
order: 1
---

# 安装

`ttm`(命令行)和 `ttm-mcp`(供 AI 使用的 MCP 服务器)以相同版本发布。任选一种方式:

## macOS、Linux:脚本

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
```

## Windows:PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.ps1 | iex"
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.ps1 | iex"
```

Releases 页面也提供 MSI 安装包;二进制文件未做代码签名,Windows SmartScreen 可能会提示警告,请选择 **更多信息 → 仍要运行**。

## Homebrew

```sh
brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp
```

## npm

```sh
npm install -g @toantranmei/cli
npx @toantranmei/mcp   # 无需安装即可运行 MCP 服务器
```

## Cargo

```sh
cargo install ttm-cli ttm-mcp
```

## 检查

```sh
ttm --version
ttm doctor
```

## 未签名的二进制文件

以上方式不会添加隔离标记,可直接运行。如果在 macOS 上用浏览器下载后被 Gatekeeper 拦截:

```sh
xattr -d com.apple.quarantine ./ttm ./ttm-mcp
```

每个版本都附带 SHA-256 校验和与 GitHub attestation。手动验证:

```sh
gh attestation verify ttm-cli-aarch64-apple-darwin.tar.xz --repo toantranmei/ttm
```

## 更新

```sh
ttm self-update          # 下载新版本,校验 SHA-256(安装了 gh 时还会验证 attestation)
ttm self-update --check  # 仅检查
```

通过 Homebrew 或 npm 安装的,请使用 `brew upgrade` / `npm update -g`。
