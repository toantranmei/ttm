# Claude Desktop bundle (`.mcpb`)

`manifest.json` describes the bundle; its `version` is the workspace version of `Cargo.toml` and its `tools` are the
tools of the `notee` toolset (checked by `crates/ttm-mcp-tools/tests/mcpb_manifest.rs`; CI also runs
`mcpb validate`).

On a release tag, the `build-mcpb` job of `.github/workflows/release.yml` runs `pack.sh`, which takes the `ttm-mcp`
archives dist built, copies the binary into `server/` and packs one bundle per OS with `@anthropic-ai/mcpb`:

| Bundle | Binary |
| --- | --- |
| `ttm-mcp-darwin.mcpb` | universal (`aarch64` + `x86_64`, joined with `lipo`) |
| `ttm-mcp-linux-x64.mcpb` | `x86_64-unknown-linux-musl` |
| `ttm-mcp-linux-arm64.mcpb` | `aarch64-unknown-linux-musl` |
| `ttm-mcp-win32-x64.mcpb` | `x86_64-pc-windows-msvc` |

Each comes with a `.sha256` and is uploaded to the GitHub Release. Locally (macOS, with `jq` and Node):

```sh
gh release download v0.1.5 -R toantranmei/ttm -p 'ttm-mcp-*.tar.xz' -p 'ttm-mcp-*.zip' -D /tmp/dist
mcpb/pack.sh 0.1.5 /tmp/dist /tmp/mcpb
```
