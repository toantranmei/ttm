//! `ttm task approve` is two-step: preview + confirmation; without a terminal it needs `--yes`.

mod common;

use common::*;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn approve_needs_a_confirmation_or_yes() {
    let server = server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/tasks/AUR-14"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "AUR-14", "key": "AUR-14", "title": "Release", "state": "Approval", "category": "approval"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/tasks/AUR-14/approve"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "AUR-14", "key": "AUR-14", "title": "Release", "state": "Done", "category": "done"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();
    ttm(root.path(), &host, &["login", "--token", "ttm_pat_good"]).await;

    // Not a terminal, no --yes: nothing is sent, exit 2.
    let out = ttm(root.path(), &host, &["task", "approve", "AUR-14"]).await;
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("task  AUR-14 · Release"), "{err}");
    assert!(err.contains("confirmation_required"), "{err}");

    // --yes: approved once.
    let out = ttm(root.path(), &host, &["task", "approve", "AUR-14", "--yes"]).await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("✓ AUR-14 approved"));
}
