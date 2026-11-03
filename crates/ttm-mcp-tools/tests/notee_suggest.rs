//! Suggestion tools against a mock API.

use serde_json::json;
use ttm_api::Client;
use ttm_mcp_tools::{HttpBackend, ToolSet, Toolsets};
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn args(v: serde_json::Value) -> Option<serde_json::Map<String, serde_json::Value>> {
    v.as_object().cloned()
}

#[tokio::test]
async fn suggest_list_and_resolve() {
    let server = MockServer::start().await;
    let client = Client::new(format!("{}/api/v1", server.uri()), Some("t".into())).unwrap();
    let set = ToolSet::new(HttpBackend::new(client, None), Toolsets::parse("notee").unwrap());

    Mock::given(method("PATCH"))
        .and(path("/api/v1/pages/p1/markdown"))
        .and(body_json(json!({"replace_content": "# Mới", "mode": "suggest"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"applied": false, "warnings": [], "suggestion_id": "s1"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pages/p1/suggestions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id": "s1", "status": "pending", "via_agent": "claude-code", "author": {"id": "u1", "name": "An"},
             "changes": [{"id": "c1", "kind": "replace", "anchor": {"quote": {"exact": "Cũ"}}, "before_nfm": "# Cũ", "after_nfm": "# Mới"}]},
            {"id": "s0", "status": "accepted", "changes": []}
        ])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/suggestions/s1/accept"))
        .and(body_json(json!({"change_ids": ["c1"]})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"id": "s1", "status": "accepted", "stale_change_ids": []})),
        )
        .mount(&server)
        .await;

    let created = set
        .call(
            "notee-update-page",
            args(json!({"page": "p1", "replace_content": "# Mới", "mode": "suggest"})),
        )
        .await
        .unwrap();
    assert!(created.content[0].as_text().unwrap().text.contains("s1"));

    let list = set
        .call("notee-get-suggestions", args(json!({"page": "p1"})))
        .await
        .unwrap();
    assert_eq!(
        list.structured_content.unwrap()["suggestions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let resolved = set
        .call(
            "notee-resolve-suggestion",
            args(json!({"suggestion": "s1", "decision": "accept", "change_ids": ["c1"]})),
        )
        .await
        .unwrap();
    assert_eq!(resolved.structured_content.unwrap()["status"], "accepted");
    assert!(
        set.call(
            "notee-resolve-suggestion",
            args(json!({"suggestion": "s1", "decision": "reject", "change_ids": ["c1"]}))
        )
        .await
        .is_err()
    );
}
