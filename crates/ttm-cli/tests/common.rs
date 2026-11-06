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
    server_with(&["notee"], me_body()).await
}

/// Every v2 feature of the server (S0 Hợp đồng §11).
pub const V2_FEATURES: &[&str] = &[
    "notee",
    "factory",
    "workflow_v2",
    "agent_v2",
    "ascode_v2",
    "templates",
    "membership",
    "confirmations",
];

/// A server with these discovery `features` and this `/me` body.
pub async fn server_with(features: &[&str], me: serde_json::Value) -> MockServer {
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
            "features": features
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .and(wiremock::matchers::header("authorization", "Bearer ttm_pat_good"))
        .respond_with(ResponseTemplate::new(200).set_body_json(me))
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
    ttm_in(root, root, host, args, stdin).await
}

/// Runs `ttm` with `cwd` as the working directory.
pub async fn ttm_in(root: &Path, cwd: &Path, host: &str, args: &[&str], stdin: Option<&str>) -> Output {
    ttm_env(root, cwd, host, args, stdin, &[]).await
}

/// Runs `ttm` with extra environment variables. Output is always deterministic: no colour (`NO_COLOR`, and stdout is
/// a pipe), 80 columns, not plain unless `TTM_PLAIN` is passed.
pub async fn ttm_env(
    root: &Path,
    cwd: &Path,
    host: &str,
    args: &[&str],
    stdin: Option<&str>,
    envs: &[(&str, &str)],
) -> Output {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_ttm"));
    cmd.current_dir(cwd);
    cmd.args(["--host", host])
        .args(args)
        .env("TTM_CONFIG_DIR", root.join("config"))
        .env("TTM_CACHE_DIR", root.join("cache"))
        .env("TTM_NO_KEYRING", "1")
        .env("TTM_NO_BROWSER", "1")
        .env("HOME", root)
        .env("TTM_HOME", root)
        .env_remove("TTM_TOKEN")
        .env_remove("TTM_HOST")
        .env_remove("TTM_PROFILE")
        .env_remove("TTM_PLAIN")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("COLUMNS")
        .env("NO_COLOR", "1")
        .env("TTM_COLUMNS", "80")
        .envs(envs.iter().copied())
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

/// stdout with the mock host and the temporary root replaced by `[HOST]` and `[ROOT]`, for snapshots (see
/// [`normalize`]).
pub fn human(output: &Output, root: &Path, host: &str) -> String {
    normalize(&String::from_utf8_lossy(&output.stdout), root, host)
}

/// stderr, normalized like [`human`].
pub fn human_err(output: &Output, root: &Path, host: &str) -> String {
    normalize(&String::from_utf8_lossy(&output.stderr), root, host)
}

/// Replaces the mock host, the temporary root and this build's version (`ttm [VERSION]`), so snapshots do not change
/// with a release.
pub fn normalize(text: &str, root: &Path, host: &str) -> String {
    let mut out = text
        .replace(host, "[HOST]")
        .replace(&format!("ttm {}", env!("CARGO_PKG_VERSION")), "ttm [VERSION]");
    let mut roots = vec![root.display().to_string()];
    if let Ok(canonical) = root.canonicalize() {
        roots.push(canonical.display().to_string());
    }
    roots.sort_by_key(|r| std::cmp::Reverse(r.len()));
    for r in roots {
        out = out.replace(&r, "[ROOT]");
    }
    // Paths under the root are written with `/`, so Windows (`[ROOT]\repo`) gives the same text as Unix.
    out.split_inclusive(char::is_whitespace)
        .map(|word| {
            if word.contains("[ROOT]") {
                word.replace('\\', "/")
            } else {
                word.to_owned()
            }
        })
        .collect()
}

/// Key paths of a JSON value: `a`, `a.b`, `list[]`, `list[].c` (arrays through their first element).
pub fn key_paths(value: &serde_json::Value) -> std::collections::BTreeSet<String> {
    fn walk(v: &serde_json::Value, prefix: &str, out: &mut std::collections::BTreeSet<String>) {
        match v {
            serde_json::Value::Object(map) => {
                for (k, child) in map {
                    let path = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    out.insert(path.clone());
                    walk(child, &path, out);
                }
            }
            serde_json::Value::Array(items) => {
                let path = format!("{prefix}[]");
                out.insert(path.clone());
                if let Some(first) = items.first() {
                    walk(first, &path, out);
                }
            }
            _ => {}
        }
    }
    let mut out = std::collections::BTreeSet::new();
    walk(value, "", &mut out);
    out
}

/// Fails unless every key of the 0.1.6 fixture `tests/fixtures/json/<name>.json` is still in `actual`
/// (new keys may be added, none removed or renamed).
pub fn assert_json_contract(name: &str, actual: &serde_json::Value) {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/json/{name}.json"));
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let fixture: serde_json::Value = serde_json::from_str(&text).unwrap();
    let have = key_paths(actual);
    let missing: Vec<_> = key_paths(&fixture)
        .into_iter()
        .filter(|k| !have.contains(k))
        .collect();
    assert!(
        missing.is_empty(),
        "`{name}` --json lost keys {missing:?}:\n{actual:#}"
    );
}
