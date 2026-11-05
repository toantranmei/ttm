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

/// A registered runner keeps its token across `ttm runner up` (no new registration): changing "team jobs" or the
/// capacity in `runner.toml` tells the server at once, as the owner.
#[tokio::test]
async fn team_jobs_of_a_registered_runner_reach_the_server() {
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, ResponseTemplate};
    let server = server().await;
    Mock::given(method("PATCH"))
        .and(path("/api/v1/factory/runners/r-1"))
        .and(wiremock::matchers::header("authorization", "Bearer ttm_pat_good"))
        .and(body_partial_json(
            serde_json::json!({ "accept_team_jobs": true, "max_parallel": 3 }),
        ))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(
        config.join("runner.toml"),
        "runner_id = \"r-1\"\nname = \"box\"\n",
    )
    .unwrap();
    let args = [
        "runner",
        "config",
        "--accept-team-jobs",
        "true",
        "--max-parallel",
        "3",
    ];
    let out = ttm_env(
        root.path(),
        root.path(),
        &server.uri(),
        &args,
        None,
        &[("TTM_TOKEN", "ttm_pat_good")],
    )
    .await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains("still has the old"), "{text}");
    // An unregistered runner only writes runner.toml (nothing to tell yet).
    let fresh = tempfile::tempdir().unwrap();
    let args = ["runner", "config", "--accept-team-jobs", "false"];
    let out = ttm_env(
        fresh.path(),
        fresh.path(),
        &server.uri(),
        &args,
        None,
        &[("TTM_TOKEN", "ttm_pat_good")],
    )
    .await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
