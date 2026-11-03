//! Runner API calls against a mock server: polling, heartbeat replies, leases, events.

use serde_json::json;
use ttm_api::Client;
use ttm_api::factory::{Heartbeat, RunEvent};
use wiremock::matchers::{body_partial_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn client(server: &MockServer) -> Client {
    Client::new(format!("{}/api/v1", server.uri()), Some("ttm_job_runner".into())).unwrap()
}

#[tokio::test]
async fn poll_returns_none_on_204_and_a_job_otherwise() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/runners/rn1/jobs"))
        .and(query_param("wait", "15"))
        .respond_with(ResponseTemplate::new(204))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/runners/rn1/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"job": {
            "run_id": "r1", "task_key": "AUR-3", "branch": "aur-3", "prompt": "p", "mcp": {"token": "ttm_job_x"}
        }})))
        .mount(&server)
        .await;
    let c = client(&server).await;
    assert!(c.poll_job("rn1", 15).await.unwrap().is_none());
    let job = c.poll_job("rn1", 15).await.unwrap().unwrap();
    assert_eq!((job.run_id.as_str(), job.harness.as_str()), ("r1", "claude-code"));
}

#[tokio::test]
async fn heartbeat_and_lease_conflict() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/runners/rn1/heartbeat"))
        .and(body_partial_json(json!({"running": ["r1"], "capacity": 2})))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"stop_runs": ["r1"], "shutdown": false})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/runs/r1/leases"))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(json!({"held_by": {"run_id": "r9", "task": "AUR-9"}})),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/runs/r1/events"))
        .and(body_partial_json(
            json!({"events": [{"kind": "log", "message": "hi"}]}),
        ))
        .respond_with(ResponseTemplate::new(202))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server).await;
    let reply = c
        .heartbeat(
            "rn1",
            &Heartbeat {
                running: vec!["r1".into()],
                capacity: 2,
                paused_until: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(reply.stop_runs, ["r1"]);
    let lease = c.request_lease("r1", &["src/auth/**".into()]).await.unwrap();
    assert!(!lease.granted);
    assert_eq!(lease.held_by.unwrap()["task"], "AUR-9");
    c.post_run_events("r1", &[RunEvent::now("log", "hi", None)])
        .await
        .unwrap();
}
