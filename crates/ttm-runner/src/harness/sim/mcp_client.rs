//! A minimal MCP client over stdio (newline-delimited JSON-RPC 2.0) for the simulated harness: starts one server of
//! the job's MCP config file, sends `initialize`, `notifications/initialized` and one `tools/call`, then stops it.
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
    if entry["command"].as_str().is_none() {
        anyhow::bail!("MCP server `{server}` is not a stdio server; the simulated client only speaks stdio");
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
    let mut child = spawn(&entry)?;
    let result = tokio::time::timeout(timeout, exchange(&mut child, tool, args)).await;
    let _ = child.kill().await;
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
    fn only_stdio_servers() {
        let config = json!({ "mcpServers": {
            "ttm": { "command": "/bin/ttm-mcp", "args": [] },
            "linear": { "type": "http", "url": "http://h/api/v1/mcp/gw/linear" }
        }});
        assert!(server_entry(&config, "ttm").is_ok());
        assert!(server_entry(&config, "linear").is_err());
        assert!(server_entry(&config, "nope").is_err());
    }
}
