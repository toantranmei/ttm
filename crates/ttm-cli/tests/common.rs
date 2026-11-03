//! Helpers shared by the binary tests: a mock server and an isolated environment.

#![allow(dead_code)]

use std::path::Path;
use std::process::Output;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub fn me_body() -> serde_json::Value {
    serde_json::json!({
        "user": {"id": "u1", "email": "an@example.com", "name": "An", "locale": "vi", "plan": "free"},
        "workspaces": [
            {"id": "w1", "name": "Cá nhân", "kind": "personal", "role": "owner"},
            {"id": "w2", "name": "Team Aurora", "kind": "team", "role": "member", "slug": "aurora"}
        ]
    })
}

/// A server with discovery and `/me` accepting `ttm_pat_good`.
pub async fn server() -> MockServer {
    let server = MockServer::start().await;
    let base = server.uri();
    Mock::given(method("GET"))
        .and(path("/.well-known/ttm"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "api_base": format!("{base}/api/v1"),
            "issuer": base,
            "authorization_endpoint": format!("{base}/ket-noi/cli"),
            "token_endpoint": format!("{base}/api/v1/oauth/token"),
            "device_authorization_endpoint": format!("{base}/api/v1/oauth/device_authorization"),
            "verification_uri": format!("{base}/activate"),
            "api_version": "v1",
            "min_cli": "0.0.1",
            "features": ["notee"]
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .and(wiremock::matchers::header("authorization", "Bearer ttm_pat_good"))
        .respond_with(ResponseTemplate::new(200).set_body_json(me_body()))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("content-type", "application/problem+json")
                .set_body_string(r#"{"code":"unauthorized","title":"Unauthorized","status":401}"#),
        )
        .mount(&server)
        .await;
    server
}

/// Runs `ttm` with config/cache under `root`, keyring disabled, no browser.
pub async fn ttm(root: &Path, host: &str, args: &[&str]) -> Output {
    ttm_with_stdin(root, host, args, None).await
}

pub async fn ttm_with_stdin(root: &Path, host: &str, args: &[&str], stdin: Option<&str>) -> Output {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_ttm"));
    cmd.args(["--host", host])
        .args(args)
        .env("TTM_CONFIG_DIR", root.join("config"))
        .env("TTM_CACHE_DIR", root.join("cache"))
        .env("TTM_NO_KEYRING", "1")
        .env("TTM_NO_BROWSER", "1")
        .env("HOME", root)
        .env_remove("TTM_TOKEN")
        .env_remove("TTM_HOST")
        .env_remove("TTM_PROFILE")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().expect("spawn ttm");
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .await
            .unwrap();
    } else {
        drop(child.stdin.take());
    }
    child.wait_with_output().await.unwrap()
}

pub fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}):\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}
