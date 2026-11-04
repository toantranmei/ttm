---
title: 参与贡献
description: ttm 仓库的构建、测试、代码格式化、文档修改和发布流程。
order: 17
section: more
---

欢迎任何形式的贡献：报告 bug、修改文档、提交 pull request。仓库：<https://github.com/toantranmei/ttm>（MIT）。

## 准备

- Rust stable（`rust-toolchain.toml` 包含 `rustfmt`、`clippy`），edition 2024，最低 1.88。
- `git`。测试 runner 需要 Claude Code（`claude`）；测试 PR 需要 `gh`。

```sh
git clone https://github.com/toantranmei/ttm && cd ttm
cargo build --workspace
target/debug/ttm --help
target/debug/ttm-mcp --help
```

将 CLI 指向本地服务器：`target/debug/ttm --host http://localhost:3000 login`。与你的真实配置完全隔离：

```sh
export TTM_CONFIG_DIR=/tmp/ttm-dev/config TTM_CACHE_DIR=/tmp/ttm-dev/cache TTM_HOME=/tmp/ttm-dev/home TTM_NO_KEYRING=1
```

## 结构

| Crate | 作用 |
| --- | --- |
| `crates/ttm-api` | `/api/v1` 的强类型 HTTP 客户端：重试、User-Agent、`Deprecation`、426、problem+json |
| `crates/ttm-core` | 配置、profile、host 选择、discovery、token 存储、项目文件、worktree、三方合并 |
| `crates/ttm-mcp-tools` | 基于 `Backend` trait 的 MCP 工具。新工具放在这里，以便服务器的远程 MCP 复用 |
| `crates/ttm-runner` | Runner：连接、executor、harness、仓库检查、合并队列、服务 |
| `crates/ttm-cli` | `ttm` 二进制（命令用 clap 定义在 `src/cli.rs` 中） |
| `crates/ttm-mcp` | `ttm-mcp` 二进制 |
| `docs/vi`、`docs/en`、`docs/zh` | 帮助中心的源文件 |
| `mcpb/` | Claude Desktop 包的 manifest |

## 提交 pull request 之前

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- 每个 pull request 只包含一项逻辑变更；commit 遵循 Conventional Commits 并使用英文（`feat(cli): …`、`fix(api): …`）。
- 测试与代码放在一起：单元测试写在模块内，HTTP 行为用 `wiremock` 写在各 crate 的 `tests/` 中。
- 绝不把 token 写入日志；凡是打印认证信息的地方都必须用 `mask` 遮盖。
- 新增或修改命令、选项、工具时：更新 `docs/vi`（主语言），以及 `docs/en`、`docs/zh`，或者在 PR 中注明翻译
  尚待完成。

CI（`.github/workflows/ci.yml`）在每次 push 和 pull request 时运行：`cargo fmt --check`、带 `-D warnings` 的 clippy、在
Linux、macOS 和 Windows 上运行测试（使用 `TTM_NO_KEYRING=1`、`TTM_NO_BROWSER=1`），并检查 Markdown 规范的副本。工作流
`build-matrix.yml` 在 PR 修改构建配置时、每周一次或手动触发时构建全部 7 个 target。

## 修改文档

`docs/<语言>/` 中的每个 Markdown 文件都有 front matter：

```yaml
---
title: Tiêu đề ngắn
description: Một câu mô tả.
order: 4
section: guides
---
```

| 键 | 含义 |
| --- | --- |
| `title` | 标题，网站将其用作 H1（正文中不要写 `#` H1） |
| `description` | 一句话，用作页面描述 |
| `order` | 在目录中的全局顺序 |
| `section` | `start`、`guides`、`reference` 或 `more` |

文件名即 slug（ASCII，不带声调的越南语，用 `-` 连接）；各语言之间保持文件名一致。页面之间的链接
写作 `[Cài đặt](cai-dat.md)`。在表格中，用 `\|` 代替 `|`。

`ttm-mcp` 在 `notee://spec/markdown` 提供的 Notee-flavored Markdown 规范嵌入自
`crates/ttm-mcp-tools/spec/notee-markdown.md`，它是 `docs/en/notee-markdown.md` 的副本；两个文件不一致时 CI 会失败
（`cmp`）。修改其中一个后，请复制到另一个。

## 发布

发布由维护者使用 [cargo-dist](https://opensource.axo.dev/cargo-dist/) 0.34 完成（`dist-workspace.toml`、
`.github/workflows/release.yml`）。

1. 修改 `Cargo.toml` 中 `[workspace.package]` 的 `version` 以及 `[workspace.dependencies]` 中的各个 `ttm-*` 条目，运行
   `cargo build` 以更新 `Cargo.lock`，并更新 [更新日志](thay-doi.md)。
2. 提交 `chore: release X.Y.Z`，然后打 tag 并推送：

   ```sh
   git tag vX.Y.Z && git push origin main vX.Y.Z
   ```

3. `Release` 工作流（在推送版本号形式的 tag 时运行）：
   - `plan`：dist 制定发布计划；
   - `build-local-artifacts`：使用 `TTM_DEFAULT_HOST=https://toantranmei.com` 构建 7 个 target，生成压缩包、`.sha256`、
     MSI 以及 **attestation**（`actions/attest-build-provenance`）；
   - `build-global-artifacts`：shell/PowerShell 安装脚本、Homebrew formula、npm 包、`sha256.sum`；
   - `host`：创建包含所有文件的 GitHub Release；
   - `publish-homebrew-formula`（仓库变量 `PUBLISH_HOMEBREW=true` 时）：使用只写的 deploy key `HOMEBREW_TAP_DEPLOY_KEY`
     将 `ttm.rb`、`ttm-mcp.rb` 推送到 `toantranmei/homebrew-tap`；
   - `publish-npm`（`PUBLISH_NPM=true` 时）：通过 **trusted publishing**
     （OIDC、`id-token: write`、npm ≥ 11.5.1）`npm publish` `@toantranmei/cli`、`@toantranmei/mcp`：npm 信任 `toantranmei/ttm` 的这个工作流，不保存任何 token。
4. 检查：`ttm self-update --check`、`brew upgrade`、`npm view @toantranmei/cli version`、
   `gh attestation verify <file> --repo toantranmei/ttm`。

没有 Windows on ARM 版本：dist 在 Linux 上构建该 target，而 Linux 上无法生成 MSI（WiX）。
