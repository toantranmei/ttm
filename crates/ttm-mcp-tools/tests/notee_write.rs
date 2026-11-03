//! Notee write tools through `HttpBackend` against a mock API: request bodies follow the P3 contract.

use serde_json::json;
use ttm_api::Client;
use ttm_mcp_tools::{CallContext, HttpBackend, ToolSet, Toolsets};
use wiremock::matchers::{body_json, body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup() -> (MockServer, ToolSet<HttpBackend>) {
    let server = MockServer::start().await;
    let client = Client::new(format!("{}/api/v1", server.uri()), Some("t".into())).unwrap();
    (
        server,
        ToolSet::new(HttpBackend::new(client, None), Toolsets::parse("notee").unwrap()),
    )
}

fn args(v: serde_json::Value) -> Option<serde_json::Map<String, serde_json::Value>> {
    v.as_object().cloned()
}

#[tokio::test]
async fn create_pages_batch() {
    let (server, set) = setup().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/pages/batch"))
        .and(body_json(json!({
            "parent": {"page_id": "p0"},
            "pages": [{"title": "A", "markdown": "# A"}, {"title": "B"}]
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({"pages": [{"id": "a"}, {"id": "b"}]})))
        .expect(1)
        .mount(&server)
        .await;
    let out = set
        .call("notee-create-pages", args(json!({"parent": {"page_id": "p0"}, "pages": [{"title": "A", "content": "# A"}, {"title": "B"}]})))
        .await
        .unwrap();
    assert_eq!(out.structured_content.unwrap()["pages"][1]["id"], "b");
    let too_many: Vec<_> = (0..21).map(|i| json!({"title": i.to_string()})).collect();
    assert!(
        set.call(
            "notee-create-pages",
            args(json!({"parent": {"page_id": "p0"}, "pages": too_many}))
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn update_page_records_the_agent_and_reports_warnings() {
    let (server, set) = setup().await;
    Mock::given(method("PATCH"))
        .and(path("/api/v1/pages/p1/markdown"))
        .and(body_json(json!({
            "update_content": [{"old_str": "quý 3", "new_str": "quý 4"}],
            "agent": "cursor"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "applied": true, "warnings": ["`old` not found"], "seq": 12, "snapshot_seq": 11
        })))
        .mount(&server)
        .await;
    let ctx = CallContext {
        agent: Some("cursor".into()),
    };
    let out = set
        .call_with(
            "notee-update-page",
            args(json!({"page": "p1", "update_content": [{"old_str": "quý 3", "new_str": "quý 4"}]})),
            &ctx,
        )
        .await
        .unwrap();
    assert!(out.content[0].as_text().unwrap().text.contains("not found"));
}

#[tokio::test]
async fn too_many_blocks_is_a_tool_error() {
    let (server, set) = setup().await;
    Mock::given(method("PATCH"))
        .respond_with(
            ResponseTemplate::new(413)
                .insert_header("content-type", "application/problem+json")
                .set_body_string(r#"{"code":"too_many_blocks","title":"Too many blocks","status":413}"#),
        )
        .mount(&server)
        .await;
    let out = set
        .call(
            "notee-update-page",
            args(json!({"page": "p1", "replace_content": "x"})),
        )
        .await
        .unwrap();
    assert_eq!(out.is_error, Some(true));
    assert_eq!(
        out.structured_content.unwrap()["error"]["code"],
        "too_many_blocks"
    );
}

#[tokio::test]
async fn comment_anchored_to_quote_and_rows() {
    let (server, set) = setup().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/resources/p1/comments"))
        .and(body_partial_json(
            json!({"anchor": {"kind": "quote", "quote": {"exact": "quý 4", "prefix": "kế hoạch "}}}),
        ))
        .respond_with(
            ResponseTemplate::new(201)
                .set_body_json(json!({"id": "t1", "anchor": {"kind": "text"}, "comments": []})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/databases/db/rows/upsert"))
        .and(body_json(
            json!({"rows": [{"values": {"Name": "A"}}, {"id": "r2", "values": {"Name": "B"}}]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ids": ["r1", "r2"]})))
        .mount(&server)
        .await;
    let out = set
        .call(
            "notee-create-comment",
            args(
                json!({"page": "p1", "text": "Kiểm tra", "quote": {"exact": "quý 4", "prefix": "kế hoạch "}}),
            ),
        )
        .await
        .unwrap();
    assert_eq!(out.structured_content.unwrap()["id"], "t1");
    let rows = set
        .call("notee-upsert-rows", args(json!({"database": "db", "rows": [{"values": {"Name": "A"}}, {"id": "r2", "values": {"Name": "B"}}]})))
        .await
        .unwrap();
    assert_eq!(rows.structured_content.unwrap()["ids"][0], "r1");
}
