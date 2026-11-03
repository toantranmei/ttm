//! Factory tools inside a run (scope from the runner) against a mock API.

use serde_json::json;
use ttm_api::Client;
use ttm_mcp_tools::{FactoryScope, HttpBackend, ToolSet, Toolsets};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn args(v: serde_json::Value) -> Option<serde_json::Map<String, serde_json::Value>> {
    v.as_object().cloned()
}

async fn setup(scope: FactoryScope) -> (MockServer, ToolSet<HttpBackend>) {
    let server = MockServer::start().await;
    let client = Client::new(format!("{}/api/v1", server.uri()), Some("ttm_job_x".into())).unwrap();
    let backend = HttpBackend::new(client, None).with_factory_scope(scope);
    (server, ToolSet::new(backend, Toolsets::parse("factory").unwrap()))
}

fn in_run() -> FactoryScope {
    FactoryScope {
        run_id: Some("r1".into()),
        task: Some("AUR-12".into()),
        project: Some("p1".into()),
    }
}

#[tokio::test]
async fn task_context_progress_and_subtasks_use_the_run() {
    let (server, set) = setup(in_run()).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/tasks/AUR-12"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"id": "AUR-12", "key": "AUR-12", "uuid": "0b7c", "title": "Login", "state": "doing", "assignee": "DEV",
            "assigneeMember": {"kind": "agent", "id": "a1", "name": "Dev bot"}, "rev": 3})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/runs/r1/context"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"markdown": "# AUR-12 Login\n- [ ] AC 1"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/runs/r1/events"))
        .and(body_partial_json(
            json!({"events": [{"kind": "progress", "message": "Tests pass", "data": {"percent": 80}}]}),
        ))
        .respond_with(ResponseTemplate::new(202))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/tasks/AUR-12/subtasks"))
        .and(body_partial_json(
            json!({"run_id": "r1", "tasks": [{"title": "Write tests", "role": "QC"}]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"tasks": [{"id": "AUR-31", "key": "AUR-31", "title": "Write tests"}], "warnings": []}),
        ))
        .mount(&server)
        .await;

    let task = set.call("factory-get-task", None).await.unwrap();
    let task = task.structured_content.unwrap();
    assert_eq!(task["key"], "AUR-12");
    assert_eq!(task["assigneeMember"]["name"], "Dev bot");
    let ctx = set.call("factory-get-context", None).await.unwrap();
    assert!(ctx.content[0].as_text().unwrap().text.starts_with("# AUR-12"));
    set.call(
        "factory-report-progress",
        args(json!({"message": "Tests pass", "percent": 80})),
    )
    .await
    .unwrap();
    assert!(
        set.call(
            "factory-report-progress",
            args(json!({"message": "x", "percent": 120}))
        )
        .await
        .is_err()
    );
    let sub = set
        .call(
            "factory-create-subtasks",
            args(json!({"tasks": [{"title": "Write tests", "role": "QC"}]})),
        )
        .await
        .unwrap();
    assert_eq!(sub.structured_content.unwrap()["tasks"][0]["key"], "AUR-31");
}

#[tokio::test]
async fn split_limit_comes_back_as_tool_error() {
    let (server, set) = setup(in_run()).await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/tasks/AUR-12/subtasks"))
        .respond_with(
            ResponseTemplate::new(422)
                .insert_header("content-type", "application/problem+json")
                .set_body_string(r#"{"code":"split-limit","title":"Split limit","status":422,"params":{"per_run":10,"used":10}}"#),
        )
        .mount(&server)
        .await;
    let out = set
        .call(
            "factory-create-subtasks",
            args(json!({"tasks": [{"title": "x"}]})),
        )
        .await
        .unwrap();
    assert_eq!(out.is_error, Some(true));
    assert!(out.content[0].as_text().unwrap().text.contains("per_run"));
}

#[tokio::test]
async fn run_tools_need_a_run() {
    let (_server, set) = setup(FactoryScope::default()).await;
    let out = set
        .call("factory-request-approval", args(json!({"reason": "merge"})))
        .await
        .unwrap();
    assert_eq!(out.is_error, Some(true));
    assert_eq!(out.structured_content.unwrap()["error"]["code"], "no_run");
    assert_eq!(set.tools().len(), 3 + 8, "workspace + factory tools");
}
