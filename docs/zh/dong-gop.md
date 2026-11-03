---
title: 参与贡献
description: 如何构建、测试并向 ttm 仓库提交 pull request,包括修改本文档。
order: 8
---

# 参与贡献

仓库:<https://github.com/toantranmei/ttm>(MIT)。欢迎报告问题、改进文档和提交 pull request。

```sh
git clone https://github.com/toantranmei/ttm && cd ttm
cargo build --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- 使用英文的 Conventional Commits(`feat(cli): ...`、`fix(api): ...`)。
- 测试与代码放在一起;HTTP 行为使用 `wiremock` 模拟服务器测试。
- 将 CLI 指向开发服务器:`--host http://localhost:3000`。
- 新的 MCP 工具写在 `crates/ttm-mcp-tools` 中,基于 `Backend` trait,以便远程 MCP 服务器复用。

## 修改文档

文档位于 `docs/vi`、`docs/en`、`docs/zh`(Markdown,front-matter 包含 `title`、`description`、`order`)。越南语为主版本;
修改时请同步更新译文,或在 pull request 中注明译文待补。
