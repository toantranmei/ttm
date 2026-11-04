#!/usr/bin/env bash
# Packs the Claude Desktop bundles (`.mcpb`) from the `ttm-mcp` archives that dist built.
#
#   mcpb/pack.sh <version> <distrib dir> <out dir>
#
# One bundle per OS, named like the archives:
#   ttm-mcp-darwin.mcpb      universal binary (lipo of aarch64 + x86_64; macOS only, where lipo exists)
#   ttm-mcp-linux-x64.mcpb   x86_64-unknown-linux-musl (static)
#   ttm-mcp-linux-arm64.mcpb aarch64-unknown-linux-musl (static)
#   ttm-mcp-win32-x64.mcpb   x86_64-pc-windows-msvc
# each with `<bundle>.sha256`. The manifest version is set to <version>, `compatibility.platforms` to the bundle's OS
# and `server.entry_point` to its binary.
# Needs bash, tar, unzip, jq, node (npx) and, for the macOS bundle, lipo.
set -euo pipefail

version="${1:?version}"
distrib="${2:?distrib dir}"
out="${3:?out dir}"
here="$(cd "$(dirname "$0")" && pwd)"
mcpb_cli="@anthropic-ai/mcpb@2.1.2"

mkdir -p "$out"
out="$(cd "$out" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# Extracts the ttm-mcp binary of <target> into <dir>; prints its path.
extract() {
  local target="$1" dir="$2"
  mkdir -p "$dir"
  if [ -f "$distrib/ttm-mcp-$target.tar.xz" ]; then
    tar -xJf "$distrib/ttm-mcp-$target.tar.xz" -C "$dir"
    echo "$dir/ttm-mcp-$target/ttm-mcp"
  elif [ -f "$distrib/ttm-mcp-$target.zip" ]; then
    unzip -q -o "$distrib/ttm-mcp-$target.zip" -d "$dir"
    echo "$dir/ttm-mcp.exe"
  else
    echo "missing archive ttm-mcp-$target in $distrib" >&2
    return 1
  fi
}

# pack <name> <platform> <binary file name> <binary path>
pack() {
  local name="$1" platform="$2" exe="$3" binary="$4"
  local dir="$work/bundle-$name"
  mkdir -p "$dir/server"
  cp "$binary" "$dir/server/$exe"
  chmod 755 "$dir/server/$exe"
  jq --arg v "$version" --arg p "$platform" --arg e "server/$exe" \
    '.version = $v | .compatibility.platforms = [$p] | .server.entry_point = $e' \
    "$here/manifest.json" > "$dir/manifest.json"
  npx --yes "$mcpb_cli" pack "$dir" "$out/ttm-mcp-$name.mcpb" < /dev/null
  (cd "$out" && shasum -a 256 "ttm-mcp-$name.mcpb" > "ttm-mcp-$name.mcpb.sha256")
}

arm="$(extract aarch64-apple-darwin "$work/darwin-arm64")"
x64="$(extract x86_64-apple-darwin "$work/darwin-x64")"
lipo -create -output "$work/ttm-mcp-darwin" "$arm" "$x64"
pack darwin darwin ttm-mcp "$work/ttm-mcp-darwin"
linux_x64="$(extract x86_64-unknown-linux-musl "$work/linux-x64")"
pack linux-x64 linux ttm-mcp "$linux_x64"
linux_arm64="$(extract aarch64-unknown-linux-musl "$work/linux-arm64")"
pack linux-arm64 linux ttm-mcp "$linux_arm64"
win="$(extract x86_64-pc-windows-msvc "$work/win32-x64")"
pack win32-x64 win32 ttm-mcp.exe "$win"

ls -l "$out"
