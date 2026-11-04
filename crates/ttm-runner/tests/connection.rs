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
        .and(query_param("wait", "25"))
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
    assert!(
        c.poll_job("rn1", ttm_runner::connection::POLL_WAIT_SECS)
            .await
            .unwrap()
            .is_none()
    );
    let job = c
        .poll_job("rn1", ttm_runner::connection::POLL_WAIT_SECS)
        .await
        .unwrap()
        .unwrap();
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

#[tokio::test]
async fn register_returns_expiry_and_merge_result_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/runners/register"))
        .and(body_partial_json(
            json!({"name": "mac", "max_parallel": 2, "accept_team_jobs": false}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "runner_id": "rn1", "token": "ttm_job_abc", "expires_at": "2026-11-05T00:00:00Z"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/merge-queue"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id": "m1", "run_id": "r1", "task_key": "AUR-3", "branch": "aur-3", "pr_url": null, "status": "ready",
             "can_merge": true, "detail": null, "enqueued_at": "2026-11-04T00:00:00Z"}
        ])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/merge-queue/m1/result"))
        .and(body_partial_json(json!({"status": "conflict", "detail": "conflicting files: a.rs"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "m1", "run_id": "r1", "task_key": "AUR-3", "branch": "aur-3", "status": "conflict", "can_merge": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    let c = client(&server).await;
    let reg = c
        .register_runner(&ttm_api::factory::RegisterRunner {
            name: "mac".into(),
            os: "macos".into(),
            arch: "aarch64".into(),
            version: "0.1.0".into(),
            harnesses: vec!["claude-code".into()],
            max_parallel: 2,
            accept_team_jobs: false,
        })
        .await
        .unwrap();
    assert_eq!(reg.expires_at.as_deref(), Some("2026-11-05T00:00:00Z"));
    let queue = c.merge_queue("p1").await.unwrap();
    assert!(queue[0].can_merge);
    let entry = c
        .merge_result("m1", false, Some("conflicting files: a.rs"))
        .await
        .unwrap();
    assert_eq!(entry.status, "conflict");
}
