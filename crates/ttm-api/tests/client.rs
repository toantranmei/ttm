//! Client behaviour against a mock server: auth header, User-Agent, retries, errors, OAuth forms.

use std::time::Duration;
use ttm_api::{ApiError, Client};
use wiremock::matchers::{body_string_contains, header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn me_body() -> serde_json::Value {
    serde_json::json!({
        "user": {"id": "u1", "email": "a@b.c", "name": "An"},
        "workspaces": [{"id": "w1", "name": "Cá nhân", "kind": "personal", "role": "owner"}]
    })
}

async fn client(server: &MockServer, token: Option<&str>) -> Client {
    Client::new(format!("{}/api/v1", server.uri()), token.map(String::from))
        .unwrap()
        .with_retry_base(Duration::ZERO)
}

#[tokio::test]
async fn me_sends_bearer_and_user_agent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .and(header("authorization", "Bearer ttm_pat_x"))
        .and(header("user-agent", ttm_api::user_agent().as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(me_body()))
        .expect(1)
        .mount(&server)
        .await;
    let me = client(&server, Some("ttm_pat_x")).await.me().await.unwrap();
    assert_eq!(me.user.email, "a@b.c");
    assert_eq!(me.workspaces[0].kind, "personal");
}

#[tokio::test]
async fn retries_5xx_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(me_body()))
        .expect(1)
        .mount(&server)
        .await;
    client(&server, None).await.me().await.unwrap();
}

#[tokio::test]
async fn honours_retry_after_on_429_and_gives_up_after_three_retries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "0")
                .insert_header("content-type", "application/problem+json")
                .set_body_string(r#"{"code":"rate_limited","title":"Too many requests","status":429}"#),
        )
        .expect(4)
        .mount(&server)
        .await;
    let err = client(&server, None).await.me().await.unwrap_err();
    assert_eq!(err.code(), "rate_limited");
}

#[tokio::test]
async fn post_is_not_retried_on_500() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;
    let err = client(&server, None)
        .await
        .post::<_, serde_json::Value>("/things", &serde_json::json!({}))
        .await
        .unwrap_err();
    assert_eq!(err.status(), Some(500));
}

#[tokio::test]
async fn problem_json_and_upgrade_required() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("content-type", "application/problem+json")
                .set_body_string(
                    r#"{"code":"insufficient_scope","title":"Insufficient scope","status":403,"required":"workspace:read"}"#,
                ),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/w1/members"))
        .respond_with(
            ResponseTemplate::new(426)
                .insert_header("content-type", "application/problem+json")
                .set_body_string(
                    r#"{"code":"upgrade_required","title":"Upgrade","status":426,"detail":"need 9.0.0"}"#,
                ),
        )
        .mount(&server)
        .await;
    let c = client(&server, Some("t")).await;
    match c.me().await.unwrap_err() {
        ApiError::Problem(p) => assert_eq!(p.required.as_deref(), Some("workspace:read")),
        other => panic!("{other:?}"),
    }
    match c.workspace_members("w1").await.unwrap_err() {
        ApiError::UpgradeRequired { message } => assert_eq!(message, "need 9.0.0"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn deprecation_header_is_remembered() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("deprecation", "@1767225600")
                .insert_header("sunset", "Wed, 01 Jul 2027 00:00:00 GMT")
                .set_body_json(me_body()),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let c = client(&server, None).await.with_cache_dir(dir.path());
    c.me().await.unwrap();
    let seen = std::fs::read_to_string(dir.path().join("deprecations")).unwrap();
    assert!(seen.contains("GET /api/v1/me"), "{seen}");
}

#[tokio::test]
async fn device_flow_forms_and_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/device_authorization"))
        .and(body_string_contains("client_id=ttm-cli"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc", "user_code": "ABCD-EFGH", "verification_uri": "https://x/activate",
            "verification_uri_complete": "https://x/activate?user_code=ABCD-EFGH", "expires_in": 600, "interval": 5
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/token"))
        .and(body_string_contains(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code",
        ))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .mount(&server)
        .await;
    let c = client(&server, None).await;
    let base = format!("{}/api/v1", server.uri());
    let device = c
        .device_authorization(&format!("{base}/oauth/device_authorization"), "workspace:read")
        .await
        .unwrap();
    assert_eq!(device.user_code, "ABCD-EFGH");
    let err = c
        .poll_device_token(&format!("{base}/oauth/token"), &device.device_code)
        .await
        .unwrap_err();
    assert_eq!(err.code(), "authorization_pending");
}

#[tokio::test]
async fn oauth_calls_do_not_send_the_bearer() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/token"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/token"))
        .and(body_string_contains("code_verifier=v"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ttm_oat_a", "token_type": "Bearer", "expires_in": 2592000, "scope": "workspace:read"
        })))
        .mount(&server)
        .await;
    let c = client(&server, Some("ttm_pat_old")).await;
    let token = c
        .exchange_code(
            &format!("{}/api/v1/oauth/token", server.uri()),
            "code",
            "http://127.0.0.1:1/callback",
            "v",
        )
        .await
        .unwrap();
    assert_eq!(token.access_token, "ttm_oat_a");
}
