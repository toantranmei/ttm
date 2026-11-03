//! Notee read tools through `HttpBackend` against a mock API.

use serde_json::json;
use ttm_api::Client;
use ttm_mcp_tools::{HttpBackend, ToolSet, Toolsets};
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PAGE: &str = "3f0c2a1e-1b2c-4d5e-8f90-123456789abc";
const DB: &str = "aaaaaaaa-1b2c-4d5e-8f90-123456789abc";

async fn setup() -> (MockServer, ToolSet<HttpBackend>) {
    let server = MockServer::start().await;
    let client = Client::new(format!("{}/api/v1", server.uri()), Some("t".into())).unwrap();
    let set = ToolSet::new(
        HttpBackend::new(client, Some("w1".into())),
        Toolsets::parse("notee").unwrap(),
    );
    (server, set)
}

fn args(v: serde_json::Value) -> Option<serde_json::Map<String, serde_json::Value>> {
    v.as_object().cloned()
}

#[tokio::test]
async fn search_uses_current_workspace_and_limits() {
    let (server, set) = setup().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .and(query_param("q", "ke hoach"))
        .and(query_param("workspace_id", "w1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id": "p1", "workspace_id": "w1", "title": "Kế hoạch", "icon": null, "snippet": "…kế hoạch quý…"},
            {"id": "p2", "workspace_id": "w1", "title": "Kế hoạch 2", "icon": null, "snippet": ""}
        ])))
        .mount(&server)
        .await;
    let out = set
        .call("notee-search", args(json!({"query": "ke hoach", "limit": 1})))
        .await
        .unwrap();
    assert_eq!(
        out.structured_content.unwrap()["results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn fetch_page_by_url_and_database_schema() {
    let (server, set) = setup().await;
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/resources/{PAGE}/summary")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"id": PAGE, "kind": "page", "title": "Kế hoạch"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/pages/{PAGE}/markdown")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": PAGE, "title": "Kế hoạch", "markdown": "# Kế hoạch\n<unknown url=\"x\" alt=\"sketch\"/>",
            "truncated": true, "unknown_block_ids": ["b7"]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/resources/{DB}/summary")))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"id": DB, "kind": "database", "title": "Tasks"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/databases/{DB}/schema")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "title": "Tasks", "properties": [{"name": "Status", "type": "select"}], "views": []
        })))
        .mount(&server)
        .await;

    let page = set
        .call(
            "notee-fetch",
            args(json!({"id": format!("https://toantranmei.com/notes/ke-hoach-{PAGE}")})),
        )
        .await
        .unwrap();
    let text = page.content[0].as_text().unwrap().text.clone();
    assert!(
        text.contains("<unknown url=\"x\" alt=\"sketch\"/>") && text.contains("b7"),
        "{text}"
    );
    assert_eq!(page.structured_content.unwrap()["kind"], "page");

    let db = set.call("notee-fetch", args(json!({"id": DB}))).await.unwrap();
    let db = db.structured_content.unwrap();
    assert_eq!(
        (db["kind"].as_str(), db["properties"][0]["type"].as_str()),
        (Some("database"), Some("select"))
    );
}

#[tokio::test]
async fn query_requires_exactly_one_of_sql_or_filter() {
    let (server, set) = setup().await;
    Mock::given(method("POST"))
        .and(path(format!("/api/v1/databases/{DB}/query")))
        .and(body_json(json!({"sql": "SELECT * FROM rows", "page_size": 100})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "columns": ["Name"], "rows": [{"id": "r1", "values": {"Name": "A"}}], "next_cursor": null
        })))
        .mount(&server)
        .await;
    assert!(
        set.call("notee-query-database", args(json!({"database": DB})))
            .await
            .is_err()
    );
    let out = set
        .call(
            "notee-query-database",
            args(json!({"database": DB, "sql": "SELECT * FROM rows", "page_size": 500})),
        )
        .await
        .unwrap();
    assert_eq!(out.structured_content.unwrap()["rows"][0]["values"]["Name"], "A");
}

#[tokio::test]
async fn comments_hide_resolved_threads() {
    let (server, set) = setup().await;
    Mock::given(method("GET"))
        .and(path(format!("/api/v1/resources/{PAGE}/comments")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id": "t1", "anchor": {"kind": "text", "block_id": "b1", "quote": "quý 4"}, "resolved_at": null,
             "comments": [{"id": "c1", "author": {"id": "u1", "name": "An"},
                "body": {"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Sửa số liệu"}]}]},
                "created_at": "2026-11-04T00:00:00Z"}]},
            {"id": "t2", "anchor": {"kind": "page"}, "resolved_at": "2026-11-04T00:00:00Z", "comments": []}
        ])))
        .mount(&server)
        .await;
    let out = set
        .call("notee-get-comments", args(json!({"page": PAGE})))
        .await
        .unwrap();
    let v = out.structured_content.unwrap();
    assert_eq!(v["threads"].as_array().unwrap().len(), 1);
    assert_eq!(v["threads"][0]["comments"][0]["text"], "Sửa số liệu");
}

#[test]
fn spec_resource_is_listed_with_notee() {
    let client = Client::new("http://localhost/api/v1", None).unwrap();
    let set = ToolSet::new(
        HttpBackend::new(client.clone(), None),
        Toolsets::parse("notee").unwrap(),
    );
    assert_eq!(set.resources().len(), 1);
    assert!(set.read("notee://spec/markdown").unwrap().contains("<unknown"));
    let without = ToolSet::new(HttpBackend::new(client, None), Toolsets::parse("").unwrap());
    assert!(without.resources().is_empty());
}
