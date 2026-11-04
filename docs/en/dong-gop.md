---
title: Contributing
description: Building, testing, formatting code, editing the docs and the release process of the ttm repo.
order: 17
section: more
---

All contributions are welcome: bug reports, doc fixes, pull requests. Repo: <https://github.com/toantranmei/ttm> (MIT).

## Prerequisites

- Rust stable (`rust-toolchain.toml` with `rustfmt`, `clippy`), edition 2024, minimum 1.88.
- `git`. To try the runner: Claude Code (`claude`); to try PRs: `gh`.

```sh
git clone https://github.com/toantranmei/ttm && cd ttm
cargo build --workspace
target/debug/ttm --help
target/debug/ttm-mcp --help
```

Point the CLI at a local server: `target/debug/ttm --host http://localhost:3000 login`. To keep it fully separate from
your real configuration:

```sh
export TTM_CONFIG_DIR=/tmp/ttm-dev/config TTM_CACHE_DIR=/tmp/ttm-dev/cache TTM_HOME=/tmp/ttm-dev/home TTM_NO_KEYRING=1
```

## Layout

| Crate | Role |
| --- | --- |
| `crates/ttm-api` | Typed HTTP client for `/api/v1`: retries, User-Agent, `Deprecation`, 426, problem+json |
| `crates/ttm-core` | Configuration, profiles, host selection, discovery, token storage, project files, worktrees, 3-way merge |
| `crates/ttm-mcp-tools` | MCP tools on the `Backend` trait. New tools go here so the server's remote MCP can reuse them |
| `crates/ttm-runner` | Runner: connection, executor, harness, repo checks, merge queue, services |
| `crates/ttm-cli` | The `ttm` binary (commands defined with clap in `src/cli.rs`) |
| `crates/ttm-mcp` | The `ttm-mcp` binary |
| `docs/vi`, `docs/en`, `docs/zh` | Help center sources |
| `mcpb/` | Claude Desktop bundle manifest |

## Before you open a pull request

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- One logical change per pull request; commits follow Conventional Commits in English (`feat(cli): …`, `fix(api): …`).
- Tests live next to the code: unit tests in the module, HTTP behavior with `wiremock` in each crate's `tests/`.
- Never write tokens to logs; anywhere credentials are printed must be masked with `mask`.
- Adding or changing a command, flag or tool: update `docs/vi` (the primary language), and `docs/en`, `docs/zh`, or say
  clearly in the PR that the translations are pending.

CI (`.github/workflows/ci.yml`) runs on every push and pull request: `cargo fmt --check`, clippy with `-D warnings`,
tests on Linux, macOS and Windows (with `TTM_NO_KEYRING=1`, `TTM_NO_BROWSER=1`), and a check of the Markdown spec copy.
The `build-matrix.yml` workflow builds all 7 targets when a PR changes the build configuration, weekly, or when run by
hand.

## Editing the docs

Each Markdown file in `docs/<language>/` has front matter:

```yaml
---
title: Short title
description: A one-sentence description.
order: 4
section: guides
---
```

| Key | Meaning |
| --- | --- |
| `title` | The title, which the website uses as the H1 (don't write a `#` H1 in the body) |
| `description` | One sentence, used for the page description |
| `order` | Global order in the table of contents |
| `section` | `start`, `guides`, `reference` or `more` |

The file name is the slug (ASCII, Vietnamese without diacritics, joined with `-`); keep the same name across languages.
Write links between pages as `[Installation](cai-dat.md)`. In tables, write `\|` instead of `|`.

The Notee-flavored Markdown spec that `ttm-mcp` serves at `notee://spec/markdown` is embedded from
`crates/ttm-mcp-tools/spec/notee-markdown.md`, a copy of `docs/en/notee-markdown.md`; CI fails when the two files
differ (`cmp`). If you edit one, copy it to the other.

## Releasing

Maintainers release with [cargo-dist](https://opensource.axo.dev/cargo-dist/) 0.34 (`dist-workspace.toml`,
`.github/workflows/release.yml`).

1. Change `version` in `[workspace.package]` and the `ttm-*` entries in `[workspace.dependencies]` of `Cargo.toml`, run
   `cargo build` to update `Cargo.lock`, and update the [Changelog](thay-doi.md).
2. Commit `chore: release X.Y.Z`, then tag and push:

   ```sh
   git tag vX.Y.Z && git push origin main vX.Y.Z
   ```

3. The `Release` workflow (runs on version-shaped tags):
   - `plan`: dist plans the release;
   - `build-local-artifacts`: builds 7 targets with `TTM_DEFAULT_HOST=https://toantranmei.com`, creates archives,
     `.sha256`, MSI, and **attestations** (`actions/attest-build-provenance`);
   - `build-global-artifacts`: shell/PowerShell install scripts, Homebrew formulas, npm packages, `sha256.sum`;
   - `host`: creates the GitHub Release with all files;
   - `publish-homebrew-formula` (when the repo variable `PUBLISH_HOMEBREW=true`): pushes `ttm.rb`, `ttm-mcp.rb` to
     `toantranmei/homebrew-tap` with the write-only deploy key `HOMEBREW_TAP_DEPLOY_KEY`;
   - `publish-npm` (when `PUBLISH_NPM=true`): `npm publish` `@toantranmei/cli`, `@toantranmei/mcp` with **trusted
     publishing** (OIDC, `id-token: write`, npm ≥ 11.5.1): npm trusts this workflow of `toantranmei/ttm`, and no token is
     stored.
4. Verify: `ttm self-update --check`, `brew upgrade`, `npm view @toantranmei/cli version`,
   `gh attestation verify <file> --repo toantranmei/ttm`.

There is no Windows on ARM build: dist builds that target on Linux, where an MSI (WiX) can't be created.
