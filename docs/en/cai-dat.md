---
title: Installation
description: Install ttm and ttm-mcp on macOS, Linux, and Windows, verify downloads, update, uninstall, and enable shell completion.
order: 2
section: start
---

`ttm` (the CLI) and `ttm-mcp` (the MCP server) are **two separate packages** with the same version. Install `ttm` first;
also install `ttm-mcp` if you use AI apps on your machine (Claude Code, Claude Desktop, Cursor, Codex…) or run the runner.

## Supported platforms

| Operating system | Architecture (target) | Channels |
| --- | --- | --- |
| macOS | Apple Silicon (`aarch64-apple-darwin`), Intel (`x86_64-apple-darwin`) | script, Homebrew, npm |
| Linux | x64, arm64; glibc (`*-unknown-linux-gnu`) and musl (`*-unknown-linux-musl`) | script, Homebrew, npm |
| Windows | x64 (`x86_64-pc-windows-msvc`) | PowerShell, MSI, npm |

> **Note:** there is no Windows on ARM build. Windows 11 on ARM runs the x64 build through the operating system's built-in
> emulation; install it as you would on Windows x64.

## Choose a channel

| Channel | `ttm` | `ttm-mcp` | Update |
| --- | --- | --- | --- |
| Script (macOS, Linux) | `ttm-cli-installer.sh` | `ttm-mcp-installer.sh` | `ttm self-update` |
| PowerShell (Windows) | `ttm-cli-installer.ps1` | `ttm-mcp-installer.ps1` | `ttm self-update` |
| Homebrew (macOS, Linux) | `toantranmei/tap/ttm` | `toantranmei/tap/ttm-mcp` | `brew upgrade` |
| npm (all platforms) | `@toantranmei/cli` | `@toantranmei/mcp` | `npm update -g` |
| MSI (Windows x64) | `ttm-cli-x86_64-pc-windows-msvc.msi` | `ttm-mcp-x86_64-pc-windows-msvc.msi` | download the new MSI |
| Source | `cargo install --git … ttm-cli` | `cargo install --git … ttm-mcp` | reinstall |
| Claude Desktop (since 0.1.4) | | `ttm-mcp-<os>.mcpb` (see [MCP](mcp.md)) | install the new bundle |

### macOS, Linux: script

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.sh | sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.sh | sh
```

The script puts the binaries in `$CARGO_HOME/bin` (default `~/.cargo/bin`) and adds that directory to `PATH` in your
shell's startup file. Open a new terminal (or run `source ~/.cargo/env`) to use them right away. To install a specific
version, replace `latest/download` with `download/v0.1.4`.

### Windows: PowerShell

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-cli-installer.ps1 | iex"
powershell -ExecutionPolicy Bypass -c "irm https://github.com/toantranmei/ttm/releases/latest/download/ttm-mcp-installer.ps1 | iex"
```

The binaries go to `%USERPROFILE%\.cargo\bin`. Open a new PowerShell window after installing.

### Windows: MSI

Download `ttm-cli-x86_64-pc-windows-msvc.msi` and `ttm-mcp-x86_64-pc-windows-msvc.msi` from the
[Releases page](https://github.com/toantranmei/ttm/releases/latest) and run them. The binaries are not code-signed, so
Windows SmartScreen may warn you: verify the files (see **Verify downloads** below), then choose **More info → Run anyway**.

### Homebrew

```sh
brew install toantranmei/tap/ttm toantranmei/tap/ttm-mcp
```

> **Note:** Homebrew refuses to install when the Xcode on your machine is outdated (the message *Your Xcode is too
> outdated*). Update Xcode from the App Store; if you don't need Xcode, remove it and use only the Command Line Tools
> (`sudo rm -rf /Applications/Xcode.app && xcode-select --install`), then run `brew install` again.

### npm

```sh
npm install -g @toantranmei/cli @toantranmei/mcp
```

The two npm packages are wrappers: a `postinstall` script downloads the right binary for your machine from GitHub
Releases. `@toantranmei/cli` provides the `ttm` command, `@toantranmei/mcp` provides the `ttm-mcp` command. Requires
Node.js ≥ 14.18.

> **Note:** npm 11 warns about install scripts in globally installed packages and may skip them depending on your
> configuration. Without that script the package has no binary and the `ttm` command fails when run. Allow just these two
> packages:
>
> ```sh
> npm install -g --allow-scripts=@toantranmei/cli,@toantranmei/mcp @toantranmei/cli @toantranmei/mcp
> ```

Run the MCP server without a global install:

```sh
npx -y @toantranmei/mcp --help
```

### From source

The crates are not published on crates.io; install directly from GitHub (requires Rust stable ≥ 1.88 and `git`):

```sh
cargo install --locked --git https://github.com/toantranmei/ttm --tag v0.1.4 ttm-cli
cargo install --locked --git https://github.com/toantranmei/ttm --tag v0.1.4 ttm-mcp
```

Self-built binaries point to `https://toantranmei.com`, like the release builds. To change the default host at build time:
`TTM_DEFAULT_HOST=https://example.com cargo install …`.

## Verify the installation

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

`[warn] token` is expected before you [sign in](dang-nhap.md). `git` is required for `ttm sync` and the runner; `claude`
(Claude Code) is required only for the runner and `ttm sync --resolve`.

## Verify downloads

Each release includes:

- a `<archive>.sha256` file next to each archive and MSI, plus a `sha256.sum` file covering all of them (a `.mcpb`
  bundle only has its own `.sha256`);
- a **GitHub artifact attestation** (Sigstore), proving the file was built by the release workflow of the
  `toantranmei/ttm` repo.

SHA-256:

```sh
V=v0.1.4; F=ttm-cli-aarch64-apple-darwin.tar.xz
curl -LO https://github.com/toantranmei/ttm/releases/download/$V/$F
curl -LO https://github.com/toantranmei/ttm/releases/download/$V/$F.sha256
shasum -a 256 "$F"; cat "$F.sha256"   # the two values must match
```

On Linux use `sha256sum`; on Windows, `Get-FileHash -Algorithm SHA256 <file>`.

Attestation, with the [GitHub CLI](https://cli.github.com):

```sh
gh attestation verify ttm-cli-aarch64-apple-darwin.tar.xz --repo toantranmei/ttm
```

`ttm self-update` does both steps for you (attestation only when `gh` is installed).

## Unsigned binaries

The binaries are not code-signed by Apple or Microsoft. The script, Homebrew, npm, and cargo channels don't set the
quarantine flag, so macOS runs them normally. If you downloaded an archive with a browser and Gatekeeper blocks it:

```sh
xattr -d com.apple.quarantine ./ttm ./ttm-mcp
```

## Update

| Install channel | Command |
| --- | --- |
| Script, PowerShell, manual download | `ttm self-update` |
| Homebrew | `brew upgrade ttm ttm-mcp` |
| npm | `npm update -g @toantranmei/cli @toantranmei/mcp` |
| MSI | download and run the new MSI |

`ttm self-update` fetches releases from GitHub (`api.github.com/repos/toantranmei/ttm/releases`), downloads the archive
for the running binary's target, verifies SHA-256 (refusing to install if the release has no checksum), verifies the
attestation when `gh` is available (refusing if verification fails), then replaces `ttm`, and also `ttm-mcp` if it is in
the same directory as `ttm`.

```sh
ttm self-update --check           # only report whether a new version exists
ttm self-update                   # install the latest version
ttm self-update --version 0.1.4   # install exactly this version (works for downgrades)
```

```text
ttm 0.1.4 is up to date.
```

```text
Updated to ttm 0.1.4: /Users/an/.cargo/bin/ttm, /Users/an/.cargo/bin/ttm-mcp
SHA-256 verified; attestation verified.
```

> **Note:** if you installed with Homebrew or npm, update with that package manager. `ttm self-update` would overwrite the
> binary the package manager tracks, leaving the two out of sync.

When the server requires a newer version (`min_cli` in discovery), every command prints the warning
`` the server needs ttm >= X (this is Y); run `ttm self-update` `` and the API may return `426 upgrade_required`.

## Uninstall

1. Sign out to revoke the token on the server and remove it from the keyring: `ttm logout` (repeat for each `--profile`
   you use).
2. If you installed the runner service, remove it first with `ttm runner uninstall-service` (see [Runner](runner.md)).
3. Remove the programs:

   | Channel | Command |
   | --- | --- |
   | Script | `rm ~/.cargo/bin/ttm ~/.cargo/bin/ttm-mcp` |
   | PowerShell | `Remove-Item "$env:USERPROFILE\.cargo\bin\ttm.exe", "$env:USERPROFILE\.cargo\bin\ttm-mcp.exe"` |
   | Homebrew | `brew uninstall ttm ttm-mcp` |
   | npm | `npm uninstall -g @toantranmei/cli @toantranmei/mcp` |
   | MSI | Settings → Apps → uninstall `ttm-cli` and `ttm-mcp` |
   | cargo | `cargo uninstall ttm-cli ttm-mcp` |

4. Delete the configuration and cache if you want: `~/.config/ttm`, `~/.cache/ttm` (Windows: `%APPDATA%\ttm`,
   `%LOCALAPPDATA%\ttm`). Runner worktrees live in `~/.ttm/worktrees` or the directory you configured.

## PATH

`ttm mcp install` writes the **absolute path** of `ttm-mcp` (the file next to `ttm`, or else the one found on `PATH`) into
the AI app's configuration, because GUI apps usually don't see your shell's `PATH`. If `ttm doctor` reports
`ttm-mcp not found on PATH`, add the install directory to `PATH`:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc   # or ~/.bashrc
```

```powershell
[Environment]::SetEnvironmentVariable("Path", "$env:USERPROFILE\.cargo\bin;" + [Environment]::GetEnvironmentVariable("Path", "User"), "User")
```

## Shell completion

`ttm completion <shell>` prints a completion script for `bash`, `zsh`, `fish`, `powershell`, `elvish`. This command doesn't
read the configuration, so it works even when `config.toml` is broken.

```sh
# zsh: add `fpath=(~/.zfunc $fpath)` to ~/.zshrc before `compinit`
mkdir -p ~/.zfunc && ttm completion zsh > ~/.zfunc/_ttm

# bash
mkdir -p ~/.local/share/bash-completion/completions
ttm completion bash > ~/.local/share/bash-completion/completions/ttm

# fish
ttm completion fish > ~/.config/fish/completions/ttm.fish
```

```powershell
# add this line to $PROFILE to make it permanent
ttm completion powershell | Out-String | Invoke-Expression
```
