//! `mcpb/manifest.json` (the Claude Desktop bundle) stays in sync with the workspace: same version as the crates, and
//! the tools it lists are the tools of the toolsets it starts (`workspace` + `notee`).

use std::path::Path;

fn manifest() -> Option<serde_json::Value> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mcpb/manifest.json");
    // Absent from a published crate: nothing to check there.
    let text = std::fs::read_to_string(file).ok()?;
    Some(serde_json::from_str(&text).expect("mcpb/manifest.json is valid JSON"))
}

#[test]
fn version_matches_the_workspace() {
    let Some(manifest) = manifest() else { return };
    assert_eq!(
        manifest["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "set mcpb/manifest.json `version` to the workspace version in Cargo.toml"
    );
}

#[test]
fn tools_match_the_bundled_toolsets() {
    let Some(manifest) = manifest() else { return };
    let args: Vec<&str> = manifest["server"]["mcp_config"]["args"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a.as_str())
        .collect();
    assert_eq!(
        args,
        ["--toolsets", "notee"],
        "the tool list below assumes the notee toolset"
    );
    let mut listed: Vec<String> = manifest["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    let mut served: Vec<String> = ttm_mcp_tools::workspace::tools()
        .into_iter()
        .chain(ttm_mcp_tools::notee::tools())
        .chain(ttm_mcp_tools::notee_write::tools())
        .chain(ttm_mcp_tools::notee_suggest::tools())
        .map(|t| t.name.to_string())
        .collect();
    listed.sort();
    served.sort();
    served.dedup();
    assert_eq!(listed, served, "update the `tools` list of mcpb/manifest.json");
}
