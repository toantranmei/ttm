# Claude Desktop bundle (`.mcpb`)

`manifest.json` describes the bundle. The release pipeline copies the `ttm-mcp` binary of each platform into
`server/` and packs one `ttm-<target>.mcpb` per platform with `npx @anthropic-ai/mcpb pack mcpb ttm-<target>.mcpb`.
