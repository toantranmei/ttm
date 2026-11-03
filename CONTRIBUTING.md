# Contributing to ttm

Thanks for helping. Bug reports, docs fixes and pull requests are all welcome.

## Setup

- Rust stable (see `rust-toolchain.toml`), edition 2024.
- `cargo build --workspace`, then run `target/debug/ttm --help` or `target/debug/ttm-mcp --help`.
- Point the CLI at a local server with `--host http://localhost:3000` or `TTM_HOST`.

## Before opening a pull request

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- One logical change per pull request; conventional commit messages in English (`feat(cli): …`, `fix(api): …`).
- Add tests next to the code: unit tests in the module, HTTP behaviour with `wiremock` in `tests/`.
- Never log tokens. Anything that prints a credential must mask it.
- User-facing strings and docs: Vietnamese is the primary language of the help center (`docs/vi`); update `docs/en`
  and `docs/zh` as well, or say in the pull request that a translation is pending.

## Layout

- `crates/ttm-api`: typed HTTP client. Types follow `openapi/v1.json` from the server.
- `crates/ttm-core`: config, profiles, discovery, credentials.
- `crates/ttm-mcp-tools`: MCP tools on top of the `Backend` trait. New tools go here so the hosted MCP server can reuse
  them.
- `crates/ttm-cli`, `crates/ttm-mcp`: the two binaries.
- `docs/{vi,en,zh}`: help center sources (Markdown with `title`, `description`, `order` front-matter).

## Releases

Releases are cut by maintainers with [cargo-dist](https://opensource.axo.dev/cargo-dist/) from a `v*` tag; see
`dist-workspace.toml` and `.github/workflows/release.yml`.
