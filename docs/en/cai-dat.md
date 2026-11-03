---
title: Install ttm
description: Install ttm and ttm-mcp on macOS, Linux and Windows with a script, Homebrew, npm or Cargo.
order: 1
---

# Install

`ttm` (command line) and `ttm-mcp` (MCP server for AI) ship with the same version. Pick one channel:

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

MSI installers are on the Releases page too; Windows SmartScreen may warn because the binaries are not code-signed:
choose **More info → Run anyway**.

## Homebrew

```sh
brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp
```

## npm

```sh
npm install -g @toantranmei/cli
npx @toantranmei/mcp   # run the MCP server without installing
```

## Cargo

```sh
cargo install ttm-cli ttm-mcp
```

## Check

```sh
ttm --version
ttm doctor
```

## Unsigned binaries

The channels above do not set the quarantine flag, so the binaries just run. If you downloaded a file with a browser
on macOS and Gatekeeper blocks it:

```sh
xattr -d com.apple.quarantine ./ttm ./ttm-mcp
```

Every release has SHA-256 checksums and a GitHub attestation. To verify by hand:

```sh
gh attestation verify ttm-cli-aarch64-apple-darwin.tar.xz --repo toantranmei/ttm
```

## Update

```sh
ttm self-update          # download, check SHA-256 (and the attestation when gh is installed)
ttm self-update --check  # only check
```

With Homebrew or npm, use `brew upgrade` / `npm update -g`.
