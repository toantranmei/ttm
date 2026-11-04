//! A minimal MCP client for the simulated harness: for one server of the job's MCP config file it sends `initialize`,
//! `notifications/initialized` and one `tools/call`. Two transports:
//!
//! - stdio (`command`, `args`, `env`): newline-delimited JSON-RPC 2.0; the server is started, then stopped.
//! - streamable HTTP (`url`, `headers`: the runner's gateway entries `{host}/api/v1/mcp/gw/<name>` with the job token
//!   as Bearer): each message is a POST; the answer is JSON or an SSE stream; `Mcp-Session-Id` is kept.
//!
//! Requests the server sends meanwhile are answered: `elicitation/create` is declined (a two-step confirmation is
//! never accepted by the simulation), `ping` and `roots/list` get empty answers, anything else "method not found".

use serde_json::{Value, json};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};

/// MCP protocol version sent in `initialize`.
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// The `mcpServers.<server>` entry of an MCP config file.
pub fn server_entry(config: &Value, server: &str) -> anyhow::Result<Value> {
    let entry = config["mcpServers"]
        .get(server)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("MCP server `{server}` is not in the job's MCP config"))?;
    if entry["command"].as_str().is_none() && entry["url"].as_str().is_none() {
        anyhow::bail!("MCP server `{server}` has neither a `command` (stdio) nor a `url` (HTTP)");
    }
    Ok(entry)
}

/// Calls `tool` with `args` on `server` of the MCP config file `config`; returns the `tools/call` result (with its
/// `content` and `isError`). The whole exchange is bounded by `timeout`.
pub async fn call(
    config: &Path,
    server: &str,
    tool: &str,
    args: Value,
    timeout: Duration,
) -> anyhow::Result<Value> {
    let config: Value = serde_json::from_slice(&std::fs::read(config)?)?;
    let entry = server_entry(&config, server)?;
    let result = if entry["command"].as_str().is_some() {
        let mut child = spawn(&entry)?;
        let result = tokio::time::timeout(timeout, exchange(&mut child, tool, args)).await;
        let _ = child.kill().await;
        result
    } else {
        tokio::time::timeout(timeout, http_exchange(&entry, tool, args, timeout)).await
    };
    match result {
        Ok(r) => r,
        Err(_) => anyhow::bail!(
            "MCP server `{server}` did not answer within {}s",
            timeout.as_secs()
        ),
    }
}

fn spawn(entry: &Value) -> anyhow::Result<Child> {
    let command = entry["command"].as_str().unwrap_or_default();
    let args: Vec<String> = entry["args"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| a.as_str().map(String::from))
        .collect();
    let env: Vec<(String, String)> = entry["env"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
        .collect();
    Ok(tokio::process::Command::new(command)
        .args(args)
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?)
}

struct Conn {
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
}

impl Conn {
    async fn send(&mut self, msg: &Value) -> anyhow::Result<()> {
        let mut line = serde_json::to_vec(msg)?;
        line.push(b'\n');
        self.stdin.write_all(&line).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Sends request `id` and reads until its answer, answering the server's own requests meanwhile.
    async fn request(&mut self, id: i64, method: &str, params: Value) -> anyhow::Result<Value> {
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await?;
        loop {
            let Some(line) = self.lines.next_line().await? else {
                anyhow::bail!("MCP server closed its output before answering `{method}`");
            };
            let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if let Some(server_method) = msg["method"].as_str() {
                if let Some(req_id) = msg.get("id").filter(|v| !v.is_null()) {
                    let reply = answer_server_request(req_id, server_method);
                    self.send(&reply).await?;
                }
                continue;
            }
            if msg["id"].as_i64() != Some(id) {
                continue;
            }
            if let Some(err) = msg.get("error") {
                anyhow::bail!(
                    "MCP `{method}` failed: {}",
                    err["message"].as_str().unwrap_or("error")
                );
            }
            return Ok(msg["result"].clone());
        }
    }
}

/// The answer to a request the server sent to this client.
pub fn answer_server_request(id: &Value, method: &str) -> Value {
    match method {
        "elicitation/create" => json!({ "jsonrpc": "2.0", "id": id, "result": { "action": "decline" } }),
        "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
        "roots/list" => json!({ "jsonrpc": "2.0", "id": id, "result": { "roots": [] } }),
        _ => json!({ "jsonrpc": "2.0", "id": id,
            "error": { "code": -32601, "message": format!("method not supported by ttm-sim: {method}") } }),
    }
}

async fn exchange(child: &mut Child, tool: &str, args: Value) -> anyhow::Result<Value> {
    let stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("no stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
    let mut conn = Conn {
        stdin,
        lines: BufReader::new(stdout).lines(),
    };
    conn.request(
        1,
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "ttm-sim", "version": env!("CARGO_PKG_VERSION") }
        }),
    )
    .await?;
    conn.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
        .await?;
    conn.request(2, "tools/call", json!({ "name": tool, "arguments": args }))
        .await
}

/// Streamable HTTP session with one server.
struct Http {
    client: reqwest::Client,
    url: String,
    headers: Vec<(String, String)>,
    session: Option<String>,
}

impl Http {
    /// POSTs one message; returns the JSON-RPC messages of the answer (none for `202 Accepted`).
    async fn post(&mut self, msg: &Value) -> anyhow::Result<Vec<Value>> {
        let mut req = self
            .client
            .post(&self.url)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .header("mcp-protocol-version", PROTOCOL_VERSION);
        for (k, v) in &self.headers {
            req = req.header(k.as_str(), v.as_str());
        }
        if let Some(id) = &self.session {
            req = req.header("mcp-session-id", id.as_str());
        }
        let res = req.body(serde_json::to_vec(msg)?).send().await?;
        if let Some(id) = res.headers().get("mcp-session-id").and_then(|v| v.to_str().ok()) {
            self.session = Some(id.to_string());
        }
        let status = res.status();
        let sse = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|t| t.starts_with("text/event-stream"));
        let body = res.text().await?;
        if !status.is_success() {
            anyhow::bail!(
                "MCP server answered HTTP {status}: {}",
                body.chars().take(300).collect::<String>()
            );
        }
        if body.trim().is_empty() {
            return Ok(Vec::new());
        }
        if sse {
            return Ok(parse_sse(&body));
        }
        Ok(match serde_json::from_str::<Value>(&body)? {
            Value::Array(items) => items,
            one => vec![one],
        })
    }

    /// Sends request `id`; answers the server's own requests found in the answer, until the result comes.
    async fn request(&mut self, id: i64, method: &str, params: Value) -> anyhow::Result<Value> {
        let mut pending = self
            .post(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await?;
        // A server request answered over POST may itself carry the awaited result in its answer.
        for _ in 0..16 {
            let mut next = Vec::new();
            for msg in pending {
                if let Some(server_method) = msg["method"].as_str() {
                    if let Some(req_id) = msg.get("id").filter(|v| !v.is_null()) {
                        let reply = answer_server_request(req_id, server_method);
                        next.extend(self.post(&reply).await?);
                    }
                    continue;
                }
                if msg["id"].as_i64() != Some(id) {
                    continue;
                }
                if let Some(err) = msg.get("error") {
                    anyhow::bail!(
                        "MCP `{method}` failed: {}",
                        err["message"].as_str().unwrap_or("error")
                    );
                }
                return Ok(msg["result"].clone());
            }
            if next.is_empty() {
                break;
            }
            pending = next;
        }
        anyhow::bail!("MCP server did not answer `{method}`")
    }
}

/// JSON-RPC messages of the `data:` fields of an SSE body (multi-line data joined with newlines).
pub fn parse_sse(body: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut data = Vec::<&str>::new();
    let mut flush = |data: &mut Vec<&str>| {
        if !data.is_empty() {
            if let Ok(v) = serde_json::from_str::<Value>(&data.join("\n")) {
                out.push(v);
            }
            data.clear();
        }
    };
    for line in body.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            flush(&mut data);
        } else if let Some(rest) = line.strip_prefix("data:") {
            data.push(rest.strip_prefix(' ').unwrap_or(rest));
        }
    }
    flush(&mut data);
    out
}

async fn http_exchange(entry: &Value, tool: &str, args: Value, timeout: Duration) -> anyhow::Result<Value> {
    let headers = entry["headers"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
        .collect();
    let mut http = Http {
        client: reqwest::Client::builder()
            .user_agent(ttm_api::user_agent())
            .timeout(timeout)
            .build()?,
        url: entry["url"].as_str().unwrap_or_default().to_string(),
        headers,
        session: None,
    };
    http.request(
        1,
        "initialize",
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "ttm-sim", "version": env!("CARGO_PKG_VERSION") }
        }),
    )
    .await?;
    http.post(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
        .await?;
    http.request(2, "tools/call", json!({ "name": tool, "arguments": args }))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_requests_are_answered() {
        let id = json!(7);
        assert_eq!(
            answer_server_request(&id, "elicitation/create")["result"]["action"],
            "decline"
        );
        assert_eq!(answer_server_request(&id, "ping")["result"], json!({}));
        assert_eq!(
            answer_server_request(&id, "sampling/createMessage")["error"]["code"],
            -32601
        );
    }

    #[test]
    fn stdio_and_http_servers() {
        let config = json!({ "mcpServers": {
            "ttm": { "command": "/bin/ttm-mcp", "args": [] },
            "linear": { "type": "http", "url": "http://h/api/v1/mcp/gw/linear" },
            "odd": { "type": "sse" }
        }});
        assert!(server_entry(&config, "ttm").is_ok());
        assert!(server_entry(&config, "linear").is_ok());
        assert!(server_entry(&config, "odd").is_err());
        assert!(server_entry(&config, "nope").is_err());
    }

    #[test]
    fn sse_messages() {
        let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\ndata: \"result\":{}}\n\n: ping\n\ndata: {\"id\":3}\n";
        let msgs = parse_sse(body);
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["id"], 2);
        assert_eq!(msgs[1]["id"], 3);
    }

    #[tokio::test]
    async fn calls_a_gateway_tool_over_http() {
        use wiremock::matchers::{body_partial_json, header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        let gw = "/api/v1/mcp/gw/linear";
        Mock::given(method("POST"))
            .and(path(gw))
            .and(header("authorization", "Bearer ttm_job_x"))
            .and(body_partial_json(json!({ "method": "initialize" })))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("mcp-session-id", "s1")
                    .set_body_json(json!({ "jsonrpc": "2.0", "id": 1, "result": { "protocolVersion": PROTOCOL_VERSION } })),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(gw))
            .and(header("mcp-session-id", "s1"))
            .and(body_partial_json(
                json!({ "method": "notifications/initialized" }),
            ))
            .respond_with(ResponseTemplate::new(202))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(gw))
            .and(header("authorization", "Bearer ttm_job_x"))
            .and(header("mcp-session-id", "s1"))
            .and(body_partial_json(json!({ "method": "tools/call", "params": { "name": "create_issue" } })))
            .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/event-stream").set_body_string(
                "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"created\"}]}}\n\n",
            ))
            .expect(1)
            .mount(&server)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mcp.json");
        std::fs::write(
            &file,
            json!({ "mcpServers": { "linear": { "type": "http", "url": format!("{}{gw}", server.uri()),
                "headers": { "Authorization": "Bearer ttm_job_x" } } } })
            .to_string(),
        )
        .unwrap();
        let result = call(
            &file,
            "linear",
            "create_issue",
            json!({ "title": "x" }),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert_eq!(result["content"][0]["text"], "created");
    }
}
