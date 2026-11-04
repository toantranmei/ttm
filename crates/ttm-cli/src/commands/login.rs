//! `ttm login`: loopback + PKCE (default), device flow (`--device`), or an existing PAT (`--token`).

use crate::app::{App, stored_from};
use crate::cli::LoginArgs;
use crate::output::{emit, waiting};
use crate::util::open_browser;
use anyhow::{Context as _, Result, anyhow, bail};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::process::ExitCode;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use ttm_api::{ApiError, Client};
use ttm_core::{Discovery, StoredToken, TokenSource};
use ttm_ui::{Callout, Header, Token};

/// How long the browser has to come back to the loopback page.
pub const LOOPBACK_TIMEOUT: Duration = Duration::from_secs(5 * 60);

#[derive(Serialize)]
struct LoginOutput {
    host: String,
    profile: String,
    method: &'static str,
    stored_in: TokenSource,
    user: Option<ttm_api::MeUser>,
}

pub async fn run(mut app: App, args: LoginArgs) -> Result<ExitCode> {
    if app.ctx.credentials.has_env_token() {
        tracing::warn!("TTM_TOKEN is set and will keep overriding the token saved by this login");
    }
    let (token, method) = if let Some(pat) = &args.token {
        (StoredToken::bare(read_pat(pat)?), "token")
    } else {
        let discovery = app.discovery(true).await?;
        let client = app.anonymous(&discovery)?;
        if args.device {
            (
                device_flow(&app, &client, &discovery, &args.scope).await?,
                "device",
            )
        } else {
            (
                loopback(&app, &client, &discovery, &args.scope, args.no_browser).await?,
                "browser",
            )
        }
    };

    // Check the token before keeping it.
    let discovery = app.discovery(false).await?;
    let client = Client::new(discovery.api_base(app.host()), Some(token.access_token.clone()))?
        .with_cache_dir(&app.ctx.paths.cache_dir);
    let me = match client.me().await {
        Ok(me) => Some(me),
        Err(err) if err.is_unauthorized() => bail!("the server rejected this token ({err})"),
        // A PAT without `workspace:read` cannot read /me; it is still a valid token.
        Err(err) if err.status() == Some(403) => None,
        Err(err) => return Err(err.into()),
    };
    let stored_in = app.save_token(&token)?;
    let host = app.host().to_string();
    let profile = app.ctx.profile();
    if profile.host.as_deref() != Some(host.as_str()) && (app.host_flag || profile.host.is_none()) {
        app.ctx.update_profile(|p| p.host = Some(host.clone()))?;
    }

    let out = LoginOutput {
        host: host.clone(),
        profile: app.ctx.profile_name.clone(),
        method,
        stored_in,
        user: me.map(|m| m.user),
    };
    let next = next_after_login();
    emit(&app, &out, |ui| {
        let callout = match &out.user {
            Some(user) => Callout::success(format!("Signed in as {} <{}>", user.name, user.email)),
            None => Callout::success("Token saved (it cannot read your profile: no workspace:read scope)"),
        };
        ui.print(
            &callout
                .detail(format!(
                    "host {host} · profile {} · token in {}",
                    out.profile, out.stored_in
                ))
                .next(next),
        );
    });
    Ok(ExitCode::SUCCESS)
}

/// `ttm project init` when this directory is not linked to a project yet, else `ttm whoami`.
fn next_after_login() -> &'static str {
    let linked = std::env::current_dir()
        .ok()
        .and_then(|cwd| ttm_core::project_file::Linked::find(&cwd).ok().flatten())
        .is_some();
    if linked { "ttm whoami" } else { "ttm project init" }
}

/// `--token ttm_pat_…` or `--token -` (stdin).
fn read_pat(value: &str) -> Result<String> {
    let token = if value == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
            .context("reading the token from stdin")?;
        buf
    } else {
        value.to_string()
    };
    let token = token.trim().to_string();
    if token.is_empty() {
        bail!("empty token");
    }
    if !token.starts_with("ttm_") {
        tracing::warn!("this does not look like a ttm token (expected a ttm_pat_… prefix)");
    }
    Ok(token)
}

/// PKCE pair: (verifier, S256 challenge).
pub fn pkce() -> (String, String) {
    let verifier = random_token(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

pub fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::fill(&mut buf[..]);
    URL_SAFE_NO_PAD.encode(buf)
}

/// The consent page URL with every OAuth parameter.
pub fn authorize_url(
    endpoint: &str,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
    scope: &str,
) -> Result<String> {
    let mut url =
        url::Url::parse(endpoint).with_context(|| format!("invalid authorization endpoint {endpoint}"))?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", ttm_api::oauth::CLI_CLIENT_ID)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("scope", scope);
    Ok(url.into())
}

async fn loopback(
    app: &App,
    client: &Client,
    discovery: &Discovery,
    scope: &str,
    no_browser: bool,
) -> Result<StoredToken> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .context("cannot open a local port for the login")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    let (verifier, challenge) = pkce();
    let state = random_token(16);
    let url = authorize_url(
        &discovery.authorization_endpoint(app.host()),
        &redirect_uri,
        &challenge,
        &state,
        scope,
    )?;
    open_browser(&app.ui, &url, no_browser);
    let spinner = waiting(
        &app.ui,
        "Waiting for the browser (5 minutes)… Without a browser here, use `ttm login --device`.",
    );
    let code = tokio::time::timeout(LOOPBACK_TIMEOUT, wait_for_code(&listener, &state))
        .await
        .map_err(|_| anyhow!("timed out after 5 minutes waiting for the browser"))??;
    spinner.clear();
    let response = client
        .exchange_code(
            &discovery.token_endpoint(app.host()),
            &code,
            &redirect_uri,
            &verifier,
        )
        .await
        .context("exchanging the authorization code")?;
    Ok(stored_from(response, None))
}

/// Outcome of one request to the loopback server.
#[derive(Debug, PartialEq, Eq)]
pub enum Callback {
    Code(String),
    Denied(String),
    /// Not the callback (favicon, wrong state…): answer and keep waiting.
    Ignore(&'static str),
}

/// Parses the request target (`/callback?code=…&state=…`) and checks `state`.
pub fn parse_callback(target: &str, expected_state: &str) -> Callback {
    let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Callback::Ignore("bad request");
    };
    if url.path() != "/callback" {
        return Callback::Ignore("not found");
    }
    let get = |key: &str| {
        url.query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
    };
    if get("state").as_deref() != Some(expected_state) {
        return Callback::Ignore("state mismatch");
    }
    if let Some(error) = get("error") {
        let description = get("error_description")
            .map(|d| format!(": {d}"))
            .unwrap_or_default();
        return Callback::Denied(format!("{error}{description}"));
    }
    match get("code") {
        Some(code) if !code.is_empty() => Callback::Code(code),
        _ => Callback::Ignore("missing code"),
    }
}

pub const SUCCESS_PAGE: &str = r#"<!doctype html><html lang="vi"><head><meta charset="utf-8"><title>ttm</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{font-family:system-ui,sans-serif;display:grid;place-items:center;min-height:90vh;margin:0;background:#fafafa;color:#111}
main{text-align:center;padding:16px}h1{font-size:1.4rem}p{color:#555}@media(prefers-color-scheme:dark){body{background:#111;color:#eee}p{color:#aaa}}</style>
</head><body><main><h1>Đã đăng nhập, quay lại terminal.</h1><p>Signed in. You can close this tab and return to the terminal.</p></main></body></html>"#;

pub const DENIED_PAGE: &str = r#"<!doctype html><html lang="vi"><head><meta charset="utf-8"><title>ttm</title></head>
<body style="font-family:system-ui,sans-serif;text-align:center;padding:40px"><h1>Đăng nhập không thành công.</h1>
<p>Sign-in was cancelled. Return to the terminal and run <code>ttm login</code> again.</p></body></html>"#;

/// Serves the loopback listener until the browser brings back a code with the right `state`.
pub async fn wait_for_code(listener: &TcpListener, state: &str) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let mut len = 0;
        while len < buf.len() {
            let n = stream.read(&mut buf[len..]).await.unwrap_or(0);
            if n == 0 {
                break;
            }
            len += n;
            if buf[..len].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8_lossy(&buf[..len]);
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");
        let outcome = parse_callback(target, state);
        let (status, body) = match &outcome {
            Callback::Code(_) => ("200 OK", SUCCESS_PAGE),
            Callback::Denied(_) => ("200 OK", DENIED_PAGE),
            Callback::Ignore(_) => ("404 Not Found", "Not found"),
        };
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
        match outcome {
            Callback::Code(code) => return Ok(code),
            Callback::Denied(reason) => bail!("sign-in was refused in the browser ({reason})"),
            Callback::Ignore(why) => tracing::debug!("loopback ignored a request: {why}"),
        }
    }
}

async fn device_flow(app: &App, client: &Client, discovery: &Discovery, scope: &str) -> Result<StoredToken> {
    let device = client
        .device_authorization(&discovery.device_authorization_endpoint(app.host()), scope)
        .await
        .context("starting the device login")?;
    let ui = &app.ui;
    // The URLs are printed whole on their own lines (never cut), so they can be copied.
    ui.eprint(
        &Header::new(format!("Code {}", device.user_code))
            .summary("Open this URL on any device and enter the code"),
    );
    ui.eline(&format!("  {}", ui.paint(Token::Blue, &device.verification_uri)));
    if let Some(complete) = &device.verification_uri_complete {
        ui.eline(&format!(
            "  {} {}",
            ui.paint(Token::Muted, "or"),
            ui.paint(Token::Blue, complete)
        ));
    }
    let _spinner = waiting(ui, "Waiting for approval…");
    let token_endpoint = discovery.token_endpoint(app.host());
    let deadline = tokio::time::Instant::now() + Duration::from_secs(device.expires_in.max(1));
    let mut interval = Duration::from_secs(device.interval.max(1));
    loop {
        tokio::time::sleep(interval).await;
        if tokio::time::Instant::now() >= deadline {
            bail!("the code expired before it was approved; run `ttm login --device` again");
        }
        match client
            .poll_device_token(&token_endpoint, &device.device_code)
            .await
        {
            Ok(response) => return Ok(stored_from(response, None)),
            Err(err) => match next_poll(&err, interval) {
                Some(next) => interval = next,
                None => return Err(anyhow::Error::new(err).context("device login failed")),
            },
        }
    }
}

/// RFC 8628 §3.5: keep polling on `authorization_pending`, add 5 s on `slow_down`, stop on anything else.
pub fn next_poll(err: &ApiError, interval: Duration) -> Option<Duration> {
    match err.code() {
        "authorization_pending" => Some(interval),
        "slow_down" => Some(interval + Duration::from_secs(5)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_s256_of_verifier() {
        let (verifier, challenge) = pkce();
        assert_eq!(verifier.len(), 43);
        assert_eq!(
            challenge,
            URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
        );
        assert_ne!(pkce().0, verifier, "random");
    }

    #[test]
    fn rfc7636_example() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn authorize_url_has_every_parameter() {
        let url = authorize_url(
            "https://a.com/ket-noi/cli",
            "http://127.0.0.1:5000/callback",
            "ch",
            "st",
            "workspace:read notee:read",
        )
        .unwrap();
        let parsed = url::Url::parse(&url).unwrap();
        let q: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(q["client_id"], "ttm-cli");
        assert_eq!(q["redirect_uri"], "http://127.0.0.1:5000/callback");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["scope"], "workspace:read notee:read");
        assert_eq!(parsed.path(), "/ket-noi/cli");
    }

    #[test]
    fn callback_parsing() {
        assert_eq!(
            parse_callback("/callback?code=abc&state=s1", "s1"),
            Callback::Code("abc".into())
        );
        assert_eq!(
            parse_callback("/callback?code=abc&state=bad", "s1"),
            Callback::Ignore("state mismatch")
        );
        assert_eq!(
            parse_callback("/favicon.ico", "s1"),
            Callback::Ignore("not found")
        );
        assert_eq!(
            parse_callback("/callback?error=access_denied&state=s1", "s1"),
            Callback::Denied("access_denied".into())
        );
    }

    #[test]
    fn device_poll_rules() {
        let pending = ApiError::OAuth {
            status: 400,
            error: "authorization_pending".into(),
            description: None,
        };
        let slow = ApiError::OAuth {
            status: 400,
            error: "slow_down".into(),
            description: None,
        };
        let denied = ApiError::OAuth {
            status: 400,
            error: "access_denied".into(),
            description: None,
        };
        let five = Duration::from_secs(5);
        assert_eq!(next_poll(&pending, five), Some(five));
        assert_eq!(next_poll(&slow, five), Some(Duration::from_secs(10)));
        assert_eq!(next_poll(&denied, five), None);
    }

    #[tokio::test]
    async fn loopback_server_returns_code_and_ignores_noise() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let browser = tokio::spawn(async move {
            for target in [
                "/favicon.ico",
                "/callback?code=wrong&state=nope",
                "/callback?code=the-code&state=s",
            ] {
                let mut s = tokio::net::TcpStream::connect(("127.0.0.1", port)).await.unwrap();
                s.write_all(format!("GET {target} HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes())
                    .await
                    .unwrap();
                let mut out = String::new();
                s.read_to_string(&mut out).await.unwrap();
                if target.contains("the-code") {
                    assert!(
                        out.starts_with("HTTP/1.1 200") && out.contains("Đã đăng nhập"),
                        "{out}"
                    );
                } else {
                    assert!(out.starts_with("HTTP/1.1 404"), "{out}");
                }
            }
        });
        let code = wait_for_code(&listener, "s").await.unwrap();
        browser.await.unwrap();
        assert_eq!(code, "the-code");
    }
}
