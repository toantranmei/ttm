//! Claude Code, headless: `claude -p <prompt> --output-format stream-json --verbose --model <m> --max-turns <n>
//! --mcp-config <file> --allowedTools <…>`. Each stdout line is a JSON message:
//!
//! | Message | Event |
//! | --- | --- |
//! | `system` (init) | `log` |
//! | `assistant` text | `thought` |
//! | `assistant` tool_use | `action` (`TodoWrite` → `plan`) |
//! | `result` | `response` + token usage |
//!
//! A usage-limit message (`… usage limit reached|<unix time>`) ends the run as `paused` until that time.

use super::{Harness, HarnessOutcome};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, watch};
use ttm_api::factory::{JobSpec, RunEvent};

#[derive(Debug, Clone, Default)]
pub struct ClaudeCode {
    /// Binary to run (default: `claude` on PATH).
    pub program: Option<PathBuf>,
}

/// What one stream-json line produced.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mapped {
    pub events: Vec<RunEvent>,
    pub tokens_in: u64,
    pub tokens_out: u64,
    /// Set by the final `result` message.
    pub result: Option<(bool, String)>,
    pub paused_until: Option<String>,
}

/// What survives between lines: test commands waiting for their result, and the last test outcome.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamState {
    pending_tests: std::collections::HashMap<String, String>,
    pub tests_pass: Option<bool>,
    pub tests_command: Option<String>,
}

/// Whether a shell command runs a test suite (`cargo test`, `pnpm test`, `npm run test`, `pytest`, `go test`, …).
pub fn is_test_command(command: &str) -> bool {
    let c = command.to_lowercase();
    [
        "cargo test",
        "cargo nextest",
        "pnpm test",
        "npm test",
        "npm run test",
        "yarn test",
        "pnpm run test",
        "pytest",
        "go test",
        "vitest",
        "jest",
        "playwright test",
        "test:e2e",
        "mix test",
        "bundle exec rspec",
        "dotnet test",
    ]
    .iter()
    .any(|t| c.contains(t))
}

/// Arguments of the `claude` command for a job.
pub fn command_args(job: &JobSpec, mcp_config: &Path) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        job.prompt.clone(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--mcp-config".into(),
        mcp_config.display().to_string(),
    ];
    if let Some(model) = &job.model {
        args.extend(["--model".into(), model.clone()]);
    }
    if let Some(turns) = job.limits.max_turns {
        args.extend(["--max-turns".into(), turns.to_string()]);
    }
    if !job.allowed_tools.is_empty() {
        args.extend(["--allowedTools".into(), job.allowed_tools.join(",")]);
    }
    args
}

fn short(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// `Claude AI usage limit reached|1767225600` → RFC 3339 of that time; other limit wording → one hour from now.
pub fn usage_limit(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    if !(lower.contains("usage limit")
        || lower.contains("rate limit reached")
        || lower.contains("limit reached"))
    {
        return None;
    }
    let at = text
        .rsplit('|')
        .next()
        .and_then(|t| t.trim().parse::<i64>().ok())
        .and_then(|secs| time::OffsetDateTime::from_unix_timestamp(secs).ok())
        .unwrap_or_else(|| time::OffsetDateTime::now_utc() + time::Duration::hours(1));
    at.format(&time::format_description::well_known::Rfc3339).ok()
}

/// Maps one stdout line (no state kept between lines).
pub fn map_line(line: &str) -> Mapped {
    map_line_with(&mut StreamState::default(), line)
}

/// Maps one stdout line, tracking test commands and their results in `state`.
pub fn map_line_with(state: &mut StreamState, line: &str) -> Mapped {
    let mut out = Mapped::default();
    let Ok(msg) = serde_json::from_str::<Value>(line) else {
        if !line.trim().is_empty() {
            out.paused_until = usage_limit(line);
            out.events.push(RunEvent::now("log", short(line, 500), None));
        }
        return out;
    };
    match msg["type"].as_str() {
        Some("system") => {
            let model = msg["model"].as_str().unwrap_or("");
            out.events.push(RunEvent::now(
                "log",
                format!("agent started {model}").trim().to_string(),
                None,
            ));
        }
        Some("assistant") => {
            for block in msg["message"]["content"].as_array().into_iter().flatten() {
                match block["type"].as_str() {
                    Some("text") => {
                        let text = block["text"].as_str().unwrap_or("");
                        if let Some(until) = usage_limit(text) {
                            out.paused_until = Some(until);
                        }
                        out.events.push(RunEvent::now("thought", short(text, 2000), None));
                    }
                    Some("tool_use") => {
                        let name = block["name"].as_str().unwrap_or("tool");
                        let input = &block["input"];
                        if name == "Bash"
                            && let (Some(id), Some(cmd)) = (block["id"].as_str(), input["command"].as_str())
                            && is_test_command(cmd)
                        {
                            state.pending_tests.insert(id.to_string(), cmd.to_string());
                        }
                        let kind = if name == "TodoWrite" { "plan" } else { "action" };
                        let summary = input["command"]
                            .as_str()
                            .or(input["file_path"].as_str())
                            .or(input["pattern"].as_str())
                            .map(|s| format!("{name}: {}", short(s, 200)))
                            .unwrap_or_else(|| name.to_string());
                        out.events.push(RunEvent::now(
                            kind,
                            summary,
                            Some(json!({ "tool": name, "input": input })),
                        ));
                    }
                    _ => {}
                }
            }
            if let Some(usage) = msg["message"].get("usage") {
                out.tokens_in += usage["input_tokens"].as_u64().unwrap_or(0)
                    + usage["cache_creation_input_tokens"].as_u64().unwrap_or(0)
                    + usage["cache_read_input_tokens"].as_u64().unwrap_or(0);
                out.tokens_out += usage["output_tokens"].as_u64().unwrap_or(0);
            }
        }
        Some("user") => {
            for block in msg["message"]["content"].as_array().into_iter().flatten() {
                if block["type"].as_str() != Some("tool_result") {
                    continue;
                }
                let Some(command) = block["tool_use_id"]
                    .as_str()
                    .and_then(|id| state.pending_tests.remove(id))
                else {
                    continue;
                };
                let passed = !block["is_error"].as_bool().unwrap_or(false);
                state.tests_pass = Some(passed);
                state.tests_command = Some(command.clone());
                let verdict = if passed { "passed" } else { "failed" };
                out.events.push(RunEvent::now(
                    "log",
                    format!("tests {verdict}: {}", short(&command, 200)),
                    Some(json!({ "tests_pass": passed, "command": command })),
                ));
            }
        }
        Some("result") => {
            let ok = !msg["is_error"].as_bool().unwrap_or(false)
                && msg["subtype"].as_str().is_none_or(|s| s == "success");
            let text = msg["result"].as_str().unwrap_or("").to_string();
            if let Some(until) = usage_limit(&text) {
                out.paused_until = Some(until);
            }
            out.events.push(RunEvent::now("response", short(&text, 4000), Some(json!({
                "num_turns": msg["num_turns"], "total_cost_usd": msg["total_cost_usd"], "subtype": msg["subtype"]
            }))));
            out.result = Some((ok, text));
        }
        _ => {}
    }
    out
}

#[async_trait]
impl Harness for ClaudeCode {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    fn available(&self) -> bool {
        self.program.as_ref().is_some_and(|p| p.exists()) || find_on_path("claude").is_some()
    }

    async fn run(
        &self,
        job: &JobSpec,
        worktree: &Path,
        mcp_config: &Path,
        events: mpsc::UnboundedSender<RunEvent>,
        mut cancel: watch::Receiver<bool>,
    ) -> anyhow::Result<HarnessOutcome> {
        let program = self
            .program
            .clone()
            .or_else(|| find_on_path("claude"))
            .ok_or_else(|| anyhow::anyhow!("`claude` is not installed"))?;
        let mut child = tokio::process::Command::new(program)
            .args(command_args(job, mcp_config))
            .current_dir(worktree)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
        let stderr = child.stderr.take();
        let err_events = events.clone();
        let stderr_task = tokio::spawn(async move {
            let mut paused = None;
            if let Some(stderr) = stderr {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if let Some(until) = usage_limit(&line) {
                        paused = Some(until);
                    }
                    let _ = err_events.send(RunEvent::now("log", short(&line, 500), None));
                }
            }
            paused
        });

        let mut outcome = HarnessOutcome::default();
        let mut lines = BufReader::new(stdout).lines();
        let mut result: Option<(bool, String)> = None;
        let mut state = StreamState::default();
        loop {
            tokio::select! {
                line = lines.next_line() => match line? {
                    Some(line) => {
                        let mapped = map_line_with(&mut state, &line);
                        outcome.tokens_in += mapped.tokens_in;
                        outcome.tokens_out += mapped.tokens_out;
                        if mapped.paused_until.is_some() {
                            outcome.paused_until = mapped.paused_until;
                        }
                        if mapped.result.is_some() {
                            result = mapped.result;
                        }
                        for e in mapped.events {
                            let _ = events.send(e);
                        }
                    }
                    None => break,
                },
                changed = cancel.changed() => {
                    if changed.is_err() || *cancel.borrow() {
                        let _ = child.kill().await;
                        outcome.status = "cancelled".into();
                        return Ok(outcome);
                    }
                }
            }
        }
        let status = child.wait().await?;
        if let Ok(Some(until)) = stderr_task.await {
            outcome.paused_until.get_or_insert(until);
        }
        outcome.summary = result.as_ref().map(|(_, t)| t.clone());
        outcome.tests_pass = state.tests_pass;
        outcome.tests_command = state.tests_command;
        outcome.status = if outcome.paused_until.is_some() {
            "paused".into()
        } else if status.success() && result.as_ref().is_none_or(|(ok, _)| *ok) {
            "succeeded".into()
        } else {
            "failed".into()
        };
        Ok(outcome)
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(&exe))
        .find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job() -> JobSpec {
        serde_json::from_value(json!({
            "run_id": "r1", "task_key": "AUR-1", "branch": "aur-1", "prompt": "Fix login",
            "model": "sonnet", "allowed_tools": ["Read", "Edit", "Bash(cargo test:*)"], "limits": {"max_turns": 30},
            "mcp": {"token": "ttm_job_x", "toolsets": ["factory"]}
        }))
        .unwrap()
    }

    #[test]
    fn args() {
        let a = command_args(&job(), Path::new("/tmp/mcp.json"));
        assert_eq!(&a[..2], ["-p", "Fix login"]);
        assert!(a.windows(2).any(|w| w == ["--output-format", "stream-json"]));
        assert!(a.windows(2).any(|w| w == ["--model", "sonnet"]));
        assert!(a.windows(2).any(|w| w == ["--max-turns", "30"]));
        assert!(
            a.windows(2)
                .any(|w| w == ["--allowedTools", "Read,Edit,Bash(cargo test:*)"])
        );
        assert!(
            !a.join(" ").contains("ttm_job_x"),
            "the token goes in the MCP config file, not argv"
        );
    }

    #[test]
    fn maps_stream_json() {
        let m = map_line(
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Reading the code"},{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}],"usage":{"input_tokens":100,"output_tokens":20}}}"#,
        );
        let kinds: Vec<_> = m.events.iter().map(|e| e.kind.as_str()).collect();
        assert_eq!(kinds, ["thought", "action"]);
        assert_eq!(m.events[1].message, "Bash: cargo test");
        assert_eq!((m.tokens_in, m.tokens_out), (100, 20));

        let plan = map_line(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TodoWrite","input":{"todos":[]}}]}}"#,
        );
        assert_eq!(plan.events[0].kind, "plan");

        let done = map_line(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Done: login fixed","num_turns":7}"#,
        );
        assert_eq!(done.result, Some((true, "Done: login fixed".into())));
        assert_eq!(done.events[0].kind, "response");
    }

    #[test]
    fn tracks_test_results() {
        let mut st = StreamState::default();
        map_line_with(
            &mut st,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"command":"cargo test --workspace"}}]}}"#,
        );
        let m = map_line_with(
            &mut st,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","is_error":true,"content":"1 failed"}]}}"#,
        );
        assert_eq!(st.tests_pass, Some(false));
        assert!(m.events[0].message.starts_with("tests failed"));
        map_line_with(
            &mut st,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"cargo test"}}]}}"#,
        );
        map_line_with(
            &mut st,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t2","content":"ok"}]}}"#,
        );
        assert_eq!(
            (st.tests_pass, st.tests_command.as_deref()),
            (Some(true), Some("cargo test"))
        );
        map_line_with(
            &mut st,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t3","name":"Bash","input":{"command":"ls"}}]}}"#,
        );
        map_line_with(
            &mut st,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t3","is_error":true}]}}"#,
        );
        assert_eq!(st.tests_pass, Some(true), "non-test commands do not count");
        assert!(is_test_command("pnpm --filter web test:e2e"));
        assert!(!is_test_command("git status"));
    }

    #[test]
    fn usage_limit_detection() {
        let until = usage_limit("Claude AI usage limit reached|1767225600").unwrap();
        assert!(until.starts_with("2026-01-01"), "{until}");
        assert!(usage_limit("all good").is_none());
        let m = map_line(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"5-hour limit reached ∙ resets 5pm"}"#,
        );
        assert!(m.paused_until.is_some());
        assert!(!m.result.unwrap().0);
    }
}
