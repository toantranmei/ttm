//! A build without the `sim-harness` feature (every release: cargo-dist builds default features) has no `ttm-sim`.

#![cfg(not(feature = "sim-harness"))]

use ttm_runner::harness;

#[test]
fn release_builds_have_no_simulated_harness() {
    assert!(harness::all().iter().all(|h| h.name() != "ttm-sim"));
    assert!(harness::by_name("ttm-sim").is_none());
    assert!(!harness::installed().iter().any(|n| n == "ttm-sim"));
}

#[test]
fn the_feature_is_never_on_by_default_or_in_dist() {
    let manifest: toml::Value = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    let default = manifest
        .get("features")
        .and_then(|f| f.get("default"))
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !default.iter().any(|f| f.as_str() == Some("sim-harness")),
        "sim-harness must not be a default feature"
    );
    for (name, text) in [
        (
            "dist-workspace.toml",
            include_str!("../../../dist-workspace.toml"),
        ),
        ("Cargo.toml", include_str!("../../../Cargo.toml")),
        (
            "crates/ttm-cli/Cargo.toml",
            include_str!("../../ttm-cli/Cargo.toml"),
        ),
        (
            "crates/ttm-mcp/Cargo.toml",
            include_str!("../../ttm-mcp/Cargo.toml"),
        ),
    ] {
        assert!(
            !text.contains("sim-harness"),
            "{name} must not enable sim-harness"
        );
    }
}
