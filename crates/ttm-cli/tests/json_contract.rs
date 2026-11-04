//! `--json` stays a contract: every key that ttm 0.1.6 printed is still printed (new keys may be added). Fixtures in
//! `tests/fixtures/json/<command>.json` hold the 0.1.6 shapes.

mod common;

use common::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn account_and_workspace_commands_keep_their_keys() {
    let server = server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/revoke"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();
    let run = |args: &'static [&'static str], stdin: Option<&'static str>| {
        let root = root.path().to_path_buf();
        let host = host.clone();
        async move { ttm_env(&root, &root, &host, args, stdin, &[]).await }
    };

    let out = run(&["login", "--token", "-", "--json"], Some("ttm_pat_good\n")).await;
    assert_json_contract("login", &json(&out));
    let out = run(&["workspace", "switch", "aurora", "--json"], None).await;
    assert_json_contract("workspace_switch", &json(&out));
    let out = run(&["workspace", "list", "--json"], None).await;
    assert_json_contract("workspace_list", &json(&out));
    let out = run(&["workspace", "current", "--json"], None).await;
    assert_json_contract("workspace_current", &json(&out));
    let out = run(&["whoami", "--json"], None).await;
    assert_json_contract("whoami", &json(&out));
    let out = run(&["status", "--json"], None).await;
    assert_json_contract("status", &json(&out));
    let out = run(&["doctor", "--json"], None).await;
    assert_json_contract("doctor", &json(&out));
    let out = run(&["token", "list", "--json"], None).await;
    assert_json_contract("token", &json(&out));
    let out = run(
        &["mcp", "install", "--client", "cursor", "--dry-run", "--json"],
        None,
    )
    .await;
    assert_json_contract("mcp_install", &json(&out));
    let out = run(&["logout", "--json"], None).await;
    assert_json_contract("logout", &json(&out));
}

#[tokio::test]
async fn json_mode_prints_only_json_on_stdout() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();
    let out = ttm_env(
        root.path(),
        root.path(),
        &host,
        &["token", "create", "--json"],
        None,
        &[],
    )
    .await;
    // The URL hint goes to stderr only for people; with --json stdout parses as one JSON value.
    let v = json(&out);
    assert!(v["url"].as_str().unwrap().contains("/me/ket-noi"));
    let out = ttm_env(root.path(), root.path(), &host, &["whoami", "--json"], None, &[]).await;
    assert!(!out.status.success());
    let v = json(&out);
    assert_eq!(v["error"]["code"], "error", "{v}");
}

#[tokio::test]
async fn notes_commands_keep_their_keys() {
    use serde_json::json;
    let server = server().await;
    let ok = |body: serde_json::Value| ResponseTemplate::new(200).set_body_json(body);
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ok(
            json!([{"id": "pg1", "workspace_id": "w1", "title": "Plan", "icon": "📝", "snippet": "x"}]),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/databases/db1/query"))
        .respond_with(ok(json!({"columns": [{"name": "Name"}], "rows": [{"id": "r1", "values": {"Name": "A"}}], "next_cursor": "c2"})))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();
    ttm(root.path(), &host, &["login", "--token", "ttm_pat_good"]).await;
    let out = ttm(root.path(), &host, &["notes", "search", "plan", "--json"]).await;
    assert_json_contract("notes_search", &json(&out));
    let out = ttm(
        root.path(),
        &host,
        &["notes", "query", "db1", "--sql", "SELECT 1", "--json"],
    )
    .await;
    assert_json_contract("notes_query", &json(&out));
}
