//! `ttm runner config claude-token`: piped, the token is read from stdin and never echoed back.

mod common;

use common::*;

#[tokio::test]
async fn claude_token_from_stdin_is_saved_and_not_printed() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let out = ttm_env(
        root.path(),
        root.path(),
        &server.uri(),
        &["runner", "config", "claude-token"],
        Some("sk-ant-oat01-secret\n"),
        &[],
    )
    .await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!all.contains("sk-ant-oat01-secret"), "{all}");
    // Empty input is refused.
    let out = ttm_env(
        root.path(),
        root.path(),
        &server.uri(),
        &["runner", "config", "claude-token"],
        Some("\n"),
        &[],
    )
    .await;
    assert!(!out.status.success());
}
