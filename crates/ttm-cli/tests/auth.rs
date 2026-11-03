//! `ttm login --token`, `whoami`, `status`, `logout` against a mock server.

mod common;

use common::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn login_with_token_then_whoami_status_logout() {
    let server = server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/revoke"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();

    let out = ttm_with_stdin(
        root.path(),
        &host,
        &["login", "--token", "-", "--json"],
        Some("ttm_pat_good\n"),
    )
    .await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let v = json(&out);
    assert_eq!(v["user"]["email"], "an@example.com");
    assert_eq!(v["stored_in"], "file");

    // The host was remembered in the profile, so later commands work without --host as well.
    let config = std::fs::read_to_string(root.path().join("config/config.toml")).unwrap();
    assert!(config.contains(&host), "{config}");

    let out = ttm(root.path(), &host, &["whoami", "--json"]).await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(json(&out)["user"]["name"], "An");

    let out = ttm(root.path(), &host, &["status", "--json"]).await;
    let v = json(&out);
    assert_eq!(v["signed_in"], true);
    assert_eq!(v["token"]["source"], "file");
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("ttm_pat_good"),
        "token must be masked"
    );

    let out = ttm(root.path(), &host, &["logout", "--json"]).await;
    let v = json(&out);
    assert_eq!(
        (v["revoked"].clone(), v["removed"].clone()),
        (true.into(), true.into())
    );

    let out = ttm(root.path(), &host, &["whoami"]).await;
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("ttm login"));
}

#[tokio::test]
async fn bad_token_is_not_saved() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let out = ttm(root.path(), &server.uri(), &["login", "--token", "ttm_pat_bad"]).await;
    assert!(!out.status.success());
    assert!(!root.path().join("config/credentials.toml").exists());
}

#[tokio::test]
async fn device_login_polls_until_approved() {
    let server = server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/device_authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "dc", "user_code": "WXYZ-2345", "verification_uri": format!("{}/activate", server.uri()),
            "expires_in": 600, "interval": 1
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/token"))
        .respond_with(
            ResponseTemplate::new(400).set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ttm_pat_good", "token_type": "Bearer", "expires_in": 2592000,
            "refresh_token": "ttm_ort_r", "scope": "workspace:read"
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let out = ttm(root.path(), &server.uri(), &["login", "--device", "--json"]).await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("WXYZ-2345"));
    assert_eq!(json(&out)["method"], "device");
}

#[tokio::test]
async fn doctor_reports_checks_as_json() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let out = ttm(root.path(), &server.uri(), &["doctor", "--json"]).await;
    let checks = json(&out);
    let names: Vec<_> = checks
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();
    for expected in [
        "host",
        "discovery",
        "version",
        "token",
        "keyring",
        "git",
        "claude",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing {expected}: {names:?}"
        );
    }
}
