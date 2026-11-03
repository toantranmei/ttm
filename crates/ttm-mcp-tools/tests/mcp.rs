//! The tool set over a real MCP session (in-memory pipe) with `HttpBackend` against a mock `/api/v1`.

use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use ttm_api::Client;
use ttm_mcp_tools::{HttpBackend, ToolSet, Toolsets};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn api() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .and(header("authorization", "Bearer ttm_pat_good"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user": {"id": "u1", "email": "an@example.com", "name": "An"},
            "workspaces": [
                {"id": "w1", "name": "Cá nhân", "kind": "personal", "role": "owner"},
                {"id": "w2", "name": "Team", "kind": "team", "role": "guest"}
            ]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/w1/members"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"user": {"id": "u1", "name": "An", "email": "an@example.com"}, "role": "owner"},
            {"user": {"id": "u2", "name": "Bình", "email": "binh@example.com"}, "role": "member"}
        ])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/w2/members"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("content-type", "application/problem+json")
                .set_body_string(r#"{"code":"forbidden","title":"Forbidden","status":403}"#),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn tools_over_mcp_session() {
    let server = api().await;
    let client = Client::new(format!("{}/api/v1", server.uri()), Some("ttm_pat_good".into())).unwrap();
    let toolset = ToolSet::new(HttpBackend::new(client, None), Toolsets::parse("").unwrap());

    let (server_io, client_io) = tokio::io::duplex(64 * 1024);
    let server_task = tokio::spawn(async move {
        let running = toolset.serve(server_io).await.expect("server initialize");
        running.waiting().await.expect("server loop");
    });
    let mcp = ().serve(client_io).await.expect("client initialize");

    let info = mcp.peer_info().expect("server info");
    assert_eq!(info.server_info.as_ref().unwrap().name, "ttm");

    let tools = mcp.list_all_tools().await.unwrap();
    let names: Vec<_> = tools.iter().map(|t| t.name.to_string()).collect();
    assert_eq!(
        names,
        ["workspace-get-self", "workspace-list", "workspace-get-users"]
    );

    let me = mcp
        .call_tool(CallToolRequestParams::new("workspace-get-self"))
        .await
        .unwrap();
    let me = me.structured_content.unwrap();
    assert_eq!(me["user"]["email"], "an@example.com");
    assert_eq!(me["workspace"]["id"], "w1", "personal workspace is the default");

    let users = mcp
        .call_tool(CallToolRequestParams::new("workspace-get-users"))
        .await
        .unwrap();
    assert_eq!(
        users.structured_content.unwrap()["users"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let args = serde_json::json!({"workspace_id": "w2"})
        .as_object()
        .cloned()
        .unwrap();
    let denied = mcp
        .call_tool(CallToolRequestParams::new("workspace-get-users").with_arguments(args))
        .await
        .unwrap();
    assert_eq!(denied.is_error, Some(true));

    mcp.cancel().await.unwrap();
    let _ = server_task.await;
}
