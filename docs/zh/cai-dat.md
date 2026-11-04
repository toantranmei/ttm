---
title: 安装
description: 在 macOS、Linux 和 Windows 上安装 ttm 与 ttm-mcp，校验下载文件，更新、卸载以及启用命令补全。
order: 2
section: start
---

`ttm`（命令行）和 `ttm-mcp`（MCP 服务器）是**两个独立的包**，版本号相同。请先安装 `ttm`；如果你在本机使用 AI 应用（Claude
Code、Claude Desktop、Cursor、Codex 等）或要运行 runner，再额外安装 `ttm-mcp`。

## 支持的平台

| 操作系统 | 架构（target） | 渠道 |
| --- | --- | --- |
| macOS | Apple Silicon（`aarch64-apple-darwin`）、Intel（`x86_64-apple-darwin`） | 脚本、Homebrew、npm |
| Linux | x64、arm64；glibc（`*-unknown-linux-gnu`）和 musl（`*-unknown-linux-musl`） | 脚本、Homebrew、npm |
| Windows | x64（`x86_64-pc-windows-msvc`） | PowerShell、MSI、npm |

> **注意：** 没有 Windows on ARM 版本。ARM 上的 Windows 11 通过系统自带的模拟层运行 x64 版本；按 Windows x64 的方式安装即可。

## 选择一种渠道

| 渠道 | `ttm` | `ttm-mcp` | 更新 |
| --- | --- | --- | --- |
| 脚本（macOS、Linux） | `ttm-cli-installer.sh` | `ttm-mcp-installer.sh` | `ttm self-update` |
| PowerShell（Windows） | `ttm-cli-installer.ps1` | `ttm-mcp-installer.ps1` | `ttm self-update` |
| Homebrew（macOS、Linux） | `toantranmei/tap/ttm` | `toantranmei/tap/ttm-mcp` | `brew upgrade` |
| npm（所有平台） | `@toantranmei/cli` | `@toantranmei/mcp` | `npm update -g` |
| MSI（Windows x64） | `ttm-cli-x86_64-pc-windows-msvc.msi` | `ttm-mcp-x86_64-pc-windows-msvc.msi` | 下载新的 MSI |
| 源码 | `cargo install --git … ttm-cli` | `cargo install --git … ttm-mcp` | 重新安装 |
| Claude Desktop（从 0.1.4 起） | | `ttm-mcp-<操作系统>.mcpb`（见 [MCP](mcp.md)） | 安装新包 |

### macOS、Linux：脚本

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
```

脚本会把二进制文件放到 `$CARGO_HOME/bin`（默认 `~/.cargo/bin`），并在 shell 的启动文件中把该目录加入 `PATH`。打开新的终端（或执行
`source ~/.cargo/env`）即可立即使用。安装固定版本：把 `latest/download` 换成 `download/v0.1.4`。

### Windows：PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.ps1 | iex"
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.ps1 | iex"
```

二进制文件位于 `%USERPROFILE%\.cargo\bin`。安装后请打开新的 PowerShell 窗口。

### Windows：MSI

在 [Releases 页面](https://github.com/toantranmei/ttm/releases/latest)下载 `ttm-cli-x86_64-pc-windows-msvc.msi` 和
`ttm-mcp-x86_64-pc-windows-msvc.msi` 后运行。二进制文件未做数字签名，Windows SmartScreen 可能会发出警告：先校验文件（见下文
**校验下载文件**），再选择 **More info → Run anyway**。

### Homebrew

```sh
brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp
```

> **注意：** 如果本机的 Xcode 过旧，Homebrew 会拒绝安装（提示 *Your Xcode is too outdated*）。请在 App Store 中更新 Xcode；如果不需要
> Xcode，可以卸载它，只使用 Command Line Tools（`sudo rm -rf /Applications/Xcode.app && xcode-select --install`），然后重新执行
> `brew install`。

### npm

```sh
npm install -g @toantranmei/cli @toantranmei/mcp
```

这两个 npm 包只是一层封装：`postinstall` 脚本会从 GitHub Releases 下载与你的机器匹配的二进制文件。`@toantranmei/cli` 提供 `ttm`
命令，`@toantranmei/mcp` 提供 `ttm-mcp` 命令。需要 Node.js ≥ 14.18。

> **注意：** npm 11 会对全局安装包的安装脚本发出警告，并可能根据配置跳过它们。缺少该脚本时，包里没有二进制文件，运行 `ttm`
> 会报错。只允许这两个包运行脚本：
>
> ```sh
> npm install -g --allow-scripts=@toantranmei/cli,@toantranmei/mcp @toantranmei/cli @toantranmei/mcp
> ```

无需全局安装即可运行 MCP 服务器：

```sh
npx -y @toantranmei/mcp --help
```

### 从源码安装

这些 crate 尚未发布到 crates.io；直接从 GitHub 安装（需要 Rust stable ≥ 1.88 和 `git`）：

```sh
cargo install --locked --git https://github.com/toantranmei/ttm --tag v0.1.4 ttm-cli
cargo install --locked --git https://github.com/toantranmei/ttm --tag v0.1.4 ttm-mcp
```

自行构建的版本与正式发布版一样指向 `https://toantranmei.com`。在构建时修改默认 host：
`TTM_DEFAULT_HOST=https://example.com cargo install …`。

## 安装后检查

```sh
ttm --version
ttm-mcp --version
ttm doctor
```

```text
ttm 0.1.4
ttm-mcp 0.1.4
[ok  ] host       https://toantranmei.com (profile default)
[ok  ] discovery  api_base https://toantranmei.com/api/v1
[ok  ] version    ttm 0.1.4, server API v1
[warn] token      not signed in (ttm login)
[ok  ] keyring    OS keyring available
[ok  ] git        git version 2.50.1
[ok  ] claude     2.1.318 (Claude Code)
[ok  ] ttm-mcp    /Users/an/.cargo/bin/ttm-mcp
```

在[登录](dang-nhap.md)之前出现 `[warn] token` 是正常的。`ttm sync` 和 runner 需要 `git`；`claude`（Claude Code）只有 runner 和
`ttm sync --resolve` 才需要。

## 校验下载文件

每个发布版本都提供：

- 每个 archive 和 MSI 旁边的 `<archive>.sha256` 文件，以及汇总全部校验值的 `sha256.sum` 文件（`.mcpb` 包只有
  自己的 `.sha256`）；
- **GitHub artifact attestation**（Sigstore），证明文件由 `toantranmei/ttm` 仓库的发布工作流构建。

SHA-256：

```sh
V=v0.1.4; F=ttm-cli-aarch64-apple-darwin.tar.xz
curl -LO https://github.com/toantranmei/ttm/releases/download/$V/$F
curl -LO https://github.com/toantranmei/ttm/releases/download/$V/$F.sha256
shasum -a 256 "$F"; cat "$F.sha256"   # 两个值必须一致
```

Linux 上使用 `sha256sum`，Windows 上使用 `Get-FileHash -Algorithm SHA256 <file>`。

使用 [GitHub CLI](https://cli.github.com) 校验 attestation：

```sh
gh attestation verify ttm-cli-aarch64-apple-darwin.tar.xz --repo toantranmei/ttm
```

`ttm self-update` 会自动完成这两步（仅当本机装有 `gh` 时才校验 attestation）。

## 未签名的二进制文件

二进制文件没有经过 Apple 或 Microsoft 的数字签名。通过脚本、Homebrew、npm、cargo 安装时不会带 quarantine 标记，因此 macOS
可以正常运行。如果你用浏览器下载 archive 后被 Gatekeeper 拦截：

```sh
xattr -d com.apple.quarantine ./ttm ./ttm-mcp
```

## 更新

| 安装渠道 | 命令 |
| --- | --- |
| 脚本、PowerShell、手动下载 | `ttm self-update` |
| Homebrew | `brew upgrade ttm ttm-mcp` |
| npm | `npm update -g @toantranmei/cli @toantranmei/mcp` |
| MSI | 下载并运行新的 MSI |

`ttm self-update` 从 GitHub（`api.github.com/repos/toantranmei/ttm/releases`）获取发布版本，下载与当前运行的二进制文件 target
相同的 archive，校验 SHA-256（发布版本没有校验值时拒绝安装），装有 `gh` 时校验 attestation（校验失败则拒绝），然后替换 `ttm`；如果
`ttm-mcp` 与 `ttm` 在同一目录，也一并替换。

```sh
ttm self-update --check           # 只报告是否有新版本
ttm self-update                   # 安装最新版本
ttm self-update --version 0.1.4   # 安装指定版本（可用于降级）
```

```text
ttm 0.1.4 is up to date.
```

```text
Updated to ttm 0.1.4: /Users/an/.cargo/bin/ttm, /Users/an/.cargo/bin/ttm-mcp
SHA-256 verified; attestation verified.
```

> **注意：** 通过 Homebrew 或 npm 安装的，请用对应的包管理器更新。`ttm self-update` 会覆盖包管理器正在跟踪的二进制文件，导致两边版本不一致。

当服务器要求更新的版本（discovery 中的 `min_cli`）时，每条命令都会打印警告
`` the server needs ttm >= X (this is Y); run `ttm self-update` ``，API 也可能返回 `426 upgrade_required`。

## 卸载

1. 退出登录，以在服务器上撤销 token 并将其从 keyring 中删除：`ttm logout`（对你用过的每个 `--profile` 重复执行）。
2. 如果安装过 runner 服务，先用 `ttm runner uninstall-service` 卸载它（见 [Runner](runner.md)）。
3. 卸载程序：

   | 渠道 | 命令 |
   | --- | --- |
   | 脚本 | `rm ~/.cargo/bin/ttm ~/.cargo/bin/ttm-mcp` |
   | PowerShell | `Remove-Item "$env:USERPROFILE\.cargo\bin\ttm.exe", "$env:USERPROFILE\.cargo\bin\ttm-mcp.exe"` |
   | Homebrew | `brew uninstall ttm ttm-mcp` |
   | npm | `npm uninstall -g @toantranmei/cli @toantranmei/mcp` |
   | MSI | 设置 → 应用 → 卸载 `ttm-cli` 和 `ttm-mcp` |
   | cargo | `cargo uninstall ttm-cli ttm-mcp` |

4. 如有需要，删除配置和缓存：`~/.config/ttm`、`~/.cache/ttm`（Windows：`%APPDATA%\ttm`、`%LOCALAPPDATA%\ttm`）。runner 的
   worktree 位于 `~/.ttm/worktrees` 或你配置的目录。

## PATH

`ttm mcp install` 会把 `ttm-mcp` 的**绝对路径**（与 `ttm` 同目录的文件，找不到时在 `PATH` 中查找）写入 AI 应用的配置，因为图形界面应用通常看不到
shell 的 `PATH`。如果 `ttm doctor` 报告 `ttm-mcp not found on PATH`，请把安装目录加入 `PATH`：

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc   # 或 ~/.bashrc
```

```powershell
[Environment]::SetEnvironmentVariable("Path", "$env:USERPROFILE\.cargo\bin;" + [Environment]::GetEnvironmentVariable("Path", "User"), "User")
```

## 命令补全（shell completion）

`ttm completion <shell>` 为 `bash`、`zsh`、`fish`、`powershell`、`elvish` 输出补全脚本。该命令不读取配置，因此即使 `config.toml`
损坏也能运行。

```sh
# zsh：在 ~/.zshrc 中 `compinit` 之前加入 `fpath=(~/.zfunc $fpath)`
mkdir -p ~/.zfunc && ttm completion zsh > ~/.zfunc/_ttm

# bash
mkdir -p ~/.local/share/bash-completion/completions
ttm completion bash > ~/.local/share/bash-completion/completions/ttm

# fish
ttm completion fish > ~/.config/fish/completions/ttm.fish
```

```powershell
# 把这一行加入 $PROFILE 以长期生效
ttm completion powershell | Out-String | Invoke-Expression
```
