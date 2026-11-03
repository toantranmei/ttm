---
title: Contributing
description: How to build, test and send pull requests to the ttm repository, including these docs.
order: 8
---

# Contributing

Repository: <https://github.com/toantranmei/ttm> (MIT). Bug reports, docs fixes and pull requests are welcome.

```sh
git clone https://github.com/toantranmei/ttm && cd ttm
cargo build --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Conventional Commits in English (`feat(cli): ...`, `fix(api): ...`).
- Tests next to the code; HTTP behaviour against a `wiremock` mock server.
- Point the CLI at a dev server: `--host http://localhost:3000`.
- New MCP tools go in `crates/ttm-mcp-tools` on the `Backend` trait so the remote MCP server can reuse them.

## Editing the docs

These docs live in `docs/vi`, `docs/en`, `docs/zh` (Markdown with `title`, `description`, `order` front-matter).
Vietnamese is the primary language; update the translations too, or say in the pull request that they are pending.
