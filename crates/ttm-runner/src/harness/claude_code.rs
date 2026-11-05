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

use super::{Harness, HarnessOutcome, ProcessGroup, RunContext, STOP_GRACE};
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
    /// Of `tokens_in`: prompt cache writes and reads.
    pub cache_creation: u64,
    pub cache_read: u64,
    /// Set by the final `result` message.
    pub result: Option<(bool, String)>,
    /// Session totals of the final `result` message, when it has `usage`.
    pub result_usage: Option<Usage>,
    pub paused_until: Option<String>,
}

/// Token counts of a `usage` object: `input` includes the cache writes and reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
}

impl Usage {
    fn max(self, other: Usage) -> Usage {
        Usage {
            input: self.input.max(other.input),
            output: self.output.max(other.output),
            cache_creation: self.cache_creation.max(other.cache_creation),
            cache_read: self.cache_read.max(other.cache_read),
        }
    }

    /// What `self` adds past `counted`, count by count.
    fn past(self, counted: Usage) -> Usage {
        Usage {
            input: self.input.saturating_sub(counted.input),
            output: self.output.saturating_sub(counted.output),
            cache_creation: self.cache_creation.saturating_sub(counted.cache_creation),
            cache_read: self.cache_read.saturating_sub(counted.cache_read),
        }
    }
}

/// What survives between lines: test commands waiting for their result, and the last test outcome.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamState {
    pending_tests: std::collections::HashMap<String, String>,
    /// Usage counted so far per API message id: Claude Code repeats one message's
    /// `usage` on every content block it streams as its own `assistant` line, and the output count can grow from one
    /// line to the next (the first line often carries the count at the start of the message). Each message is counted
    /// once, at its largest usage: a line only adds what goes past what was already counted.
    usage_seen: std::collections::HashMap<String, Usage>,
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

/// Arguments of the `claude` command for a job. Every job gets `--settings` (no Claude attribution in commits or
/// PRs); a teammate's job also gets `--setting-sources project,local` (never this machine's user settings).
pub fn command_args(job: &JobSpec, mcp_config: &Path, ctx: &RunContext) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        job.prompt.clone(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
        "--mcp-config".into(),
        mcp_config.display().to_string(),
        // Only the ttm MCP servers of the job config: never the MCP servers configured on this machine, so a job
        // delegated by a teammate cannot reach the owner's other tools.
        "--strict-mcp-config".into(),
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
    if let Some(settings) = &ctx.settings {
        args.extend(["--settings".into(), settings.display().to_string()]);
    }
    if job.team_job {
        args.extend(["--setting-sources".into(), "project,local".into()]);
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
                let usage = usage_tokens(usage);
                let counted = match msg["message"]["id"].as_str() {
                    Some(id) => {
                        let seen = state.usage_seen.entry(id.to_string()).or_default();
                        let before = *seen;
                        *seen = seen.max(usage);
                        before
                    }
                    None => Usage::default(),
                };
                let added = usage.past(counted);
                out.tokens_in += added.input;
                out.tokens_out += added.output;
                out.cache_creation += added.cache_creation;
                out.cache_read += added.cache_read;
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
            out.result_usage = msg.get("usage").filter(|u| u.is_object()).map(usage_tokens);
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
        ctx: &RunContext,
        events: mpsc::UnboundedSender<RunEvent>,
        mut cancel: watch::Receiver<bool>,
    ) -> anyhow::Result<HarnessOutcome> {
        let program = self
            .program
            .clone()
            .or_else(|| find_on_path("claude"))
            .ok_or_else(|| anyhow::anyhow!("`claude` is not installed"))?;
        let mut command = tokio::process::Command::new(program);
        command
            .args(command_args(job, mcp_config, ctx))
            .envs(ctx.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .current_dir(worktree)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        // Its own process group: a stop (budget, cancel, job timeout) reaches what the agent started too.
        ProcessGroup::prepare(&mut command);
        let mut child = command.spawn()?;
        let mut group = ProcessGroup::of(&child);
        ctx.usage.set(0, 0);
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
                        outcome.cache_creation += mapped.cache_creation;
                        outcome.cache_read += mapped.cache_read;
                        // Claude Code's own session totals, when it reports them, win over a smaller running sum.
                        if let Some(total) = mapped.result_usage {
                            outcome.tokens_in = outcome.tokens_in.max(total.input);
                            outcome.tokens_out = outcome.tokens_out.max(total.output);
                            outcome.cache_creation = outcome.cache_creation.max(total.cache_creation);
                            outcome.cache_read = outcome.cache_read.max(total.cache_read);
                        }
                        ctx.usage.set_outcome(&outcome);
                        // The job's token budget is a hard stop: the agent is stopped as soon as it is passed (a
                        // final `result` only settles the totals: the agent is already done). The cache counts by
                        // price (`weighted_tokens`).
                        if let Some(budget) = over_budget(&outcome, job.limits.budget_tokens)
                            .filter(|_| mapped.result.is_none())
                        {
                            let used = outcome.weighted_tokens();
                            // What that line said still reaches the run's events.
                            for e in mapped.events {
                                let _ = events.send(e);
                            }
                            group.stop(&mut child, STOP_GRACE).await;
                            let _ = events.send(RunEvent::now(
                                "log",
                                format!("budget exhausted: {used} of {budget} tokens, the agent was stopped"),
                                // The raw counts sit under `at_stop`: a server reads `tokens_in` / `usage` at the top
                                // of an event's data as usage to add, and these were already counted.
                                Some(json!({
                                    "budget_tokens": budget, "tokens": used, "at_stop": {
                                        "tokens_in": outcome.tokens_in, "tokens_out": outcome.tokens_out,
                                        "cache_creation_tokens": outcome.cache_creation,
                                        "cache_read_tokens": outcome.cache_read
                                    }
                                })),
                            ));
                            outcome.status = "failed".into();
                            outcome.summary = Some(format!("budget exhausted ({used} of {budget} tokens)"));
                            outcome.tests_pass = state.tests_pass;
                            outcome.tests_command = state.tests_command.clone();
                            return Ok(outcome);
                        }
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
                        group.stop(&mut child, STOP_GRACE).await;
                        outcome.status = "cancelled".into();
                        return Ok(outcome);
                    }
                }
            }
        }
        let status = child.wait().await?;
        group.release();
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

/// The counts of a `usage` object, `input` = input + cache creation + cache read.
fn usage_tokens(usage: &Value) -> Usage {
    let n = |k: &str| usage[k].as_u64().unwrap_or(0);
    let cache_creation = n("cache_creation_input_tokens");
    let cache_read = n("cache_read_input_tokens");
    Usage {
        input: n("input_tokens") + cache_creation + cache_read,
        output: n("output_tokens"),
        cache_creation,
        cache_read,
    }
}

/// The budget when the tokens used so far passed it, counted by price ([`HarnessOutcome::weighted_tokens`]).
pub fn over_budget(outcome: &HarnessOutcome, budget: Option<u64>) -> Option<u64> {
    budget.filter(|b| outcome.weighted_tokens() > *b)
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
        let ctx = RunContext {
            settings: Some("/c/settings-r1.json".into()),
            ..RunContext::default()
        };
        let a = command_args(&job(), Path::new("/tmp/mcp.json"), &ctx);
        assert!(a.windows(2).any(|w| w == ["--settings", "/c/settings-r1.json"]));
        assert!(
            !a.iter().any(|x| x == "--setting-sources"),
            "own job keeps the user's settings"
        );
        let mut team = job();
        team.team_job = true;
        let t = command_args(&team, Path::new("/tmp/mcp.json"), &ctx);
        assert!(t.windows(2).any(|w| w == ["--setting-sources", "project,local"]));
        assert_eq!(&a[..2], ["-p", "Fix login"]);
        assert!(a.windows(2).any(|w| w == ["--output-format", "stream-json"]));
        assert!(a.iter().any(|x| x == "--strict-mcp-config"));
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
    fn counts_one_message_usage_once() {
        let mut st = StreamState::default();
        let block = |text: &str| {
            format!(
                r#"{{"type":"assistant","message":{{"id":"msg_1","content":[{{"type":"text","text":"{text}"}}],"usage":{{"input_tokens":10,"cache_read_input_tokens":1000,"output_tokens":5}}}}}}"#
            )
        };
        let a = map_line_with(&mut st, &block("one"));
        let b = map_line_with(&mut st, &block("two"));
        assert_eq!((a.tokens_in, a.tokens_out), (1010, 5));
        assert_eq!(
            (b.tokens_in, b.tokens_out),
            (0, 0),
            "the same message's usage again"
        );
        assert_eq!(b.events.len(), 1, "its content still maps");
        let other = map_line_with(&mut st, &block("three").replace("msg_1", "msg_2"));
        assert_eq!(other.tokens_in, 1010);
    }

    #[test]
    fn the_budget_counts_the_cache_by_price() {
        let mut st = StreamState::default();
        let line = |id: &str| {
            format!(
                r#"{{"type":"assistant","message":{{"id":"{id}","content":[],"usage":{{"input_tokens":100,"cache_creation_input_tokens":400,"cache_read_input_tokens":10000,"output_tokens":10}}}}}}"#
            )
        };
        let mut outcome = HarnessOutcome::default();
        for id in ["m1", "m1", "m2"] {
            let m = map_line_with(&mut st, &line(id));
            outcome.tokens_in += m.tokens_in;
            outcome.tokens_out += m.tokens_out;
            outcome.cache_creation += m.cache_creation;
            outcome.cache_read += m.cache_read;
        }
        // two messages: raw 2 x 10 510, counted 2 x (100 + 10 + 500 + 1000)
        assert_eq!((outcome.tokens_in, outcome.tokens_out), (21_000, 20));
        assert_eq!((outcome.cache_creation, outcome.cache_read), (800, 20_000));
        assert_eq!(outcome.weighted_tokens(), 3220);
        assert_eq!(
            over_budget(&outcome, Some(5000)),
            None,
            "the raw 21 020 would have stopped it"
        );
        assert_eq!(over_budget(&outcome, Some(3000)), Some(3000));
    }

    #[test]
    fn counts_a_message_at_its_final_output() {
        let mut st = StreamState::default();
        let line = |id: &str, out: u64| {
            format!(
                r#"{{"type":"assistant","message":{{"id":"{id}","content":[{{"type":"text","text":"x"}}],"usage":{{"input_tokens":10,"cache_read_input_tokens":1000,"output_tokens":{out}}}}}}}"#
            )
        };
        // One message streamed as three lines whose output count grows, then the same count again.
        let got: Vec<(u64, u64)> = [1, 40, 312, 312]
            .into_iter()
            .map(|n| map_line_with(&mut st, &line("msg_1", n)))
            .map(|m| (m.tokens_in, m.tokens_out))
            .collect();
        assert_eq!(got, [(1010, 1), (0, 39), (0, 272), (0, 0)]);
        // A smaller count later (out of order) never takes tokens back.
        assert_eq!(map_line_with(&mut st, &line("msg_1", 5)).tokens_out, 0);
        let next = map_line_with(&mut st, &line("msg_2", 7));
        assert_eq!((next.tokens_in, next.tokens_out), (1010, 7));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn the_budget_stops_the_agent() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("claude");
        // a fake claude: one message of 600 tokens every 50 ms, forever
        std::fs::write(
            &program,
            "#!/bin/sh\ni=0\nwhile true; do i=$((i+1)); echo '{\"type\":\"assistant\",\"message\":{\"id\":\"m'$i'\",\"content\":[],\"usage\":{\"input_tokens\":500,\"output_tokens\":100}}}'; sleep 0.05; done\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut job = job();
        job.limits.budget_tokens = Some(2000);
        let harness = ClaudeCode {
            program: Some(program),
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (_stop, cancel) = watch::channel(false);
        let ctx = RunContext::default();
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            harness.run(&job, dir.path(), &dir.path().join("mcp.json"), &ctx, tx, cancel),
        )
        .await
        .expect("stopped, not left running")
        .unwrap();
        assert_eq!(out.status, "failed");
        assert_eq!(
            out.tokens_in + out.tokens_out,
            2400,
            "stopped at the first message past 2000"
        );
        assert!(out.summary.unwrap().contains("budget exhausted"));
        let mut logs = Vec::new();
        let mut stop_data = None;
        while let Ok(e) = rx.try_recv() {
            if e.message.starts_with("budget exhausted") {
                stop_data = e.data.clone();
            }
            logs.push(e.message);
        }
        assert!(logs.iter().any(|m| m.starts_with("budget exhausted")), "{logs:?}");
        // the raw counts are nested: a server must not add them as usage again
        let data = stop_data.unwrap();
        assert!(
            data.get("tokens_in").is_none() && data.get("usage").is_none(),
            "{data}"
        );
        assert_eq!(data["at_stop"]["tokens_in"], 2000);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_message_growing_output_counts_toward_the_budget() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("claude");
        // a fake claude: one message repeated on every streamed block, its output count growing by 100 each line
        std::fs::write(
            &program,
            "#!/bin/sh
o=0
while true; do o=$((o+100)); echo '{\"type\":\"assistant\",\"message\":{\"id\":\"m1\",\"content\":[],\"usage\":{\"input_tokens\":500,\"output_tokens\":'$o'}}}'; sleep 0.05; done\n",
        )
        .unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut job = job();
        job.limits.budget_tokens = Some(1000);
        let harness = ClaudeCode {
            program: Some(program),
        };
        let (tx, _rx) = mpsc::unbounded_channel();
        let (_stop, cancel) = watch::channel(false);
        let ctx = RunContext::default();
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            harness.run(&job, dir.path(), &dir.path().join("mcp.json"), &ctx, tx, cancel),
        )
        .await
        .expect("stopped, not left running")
        .unwrap();
        assert_eq!(out.status, "failed");
        assert_eq!(
            (out.tokens_in, out.tokens_out),
            (500, 600),
            "input once, output at its latest count, stopped past 1000"
        );
    }

    /// A fake `claude` (a shell script) in a temp dir; `pidfile` is where it may write a child's pid.
    #[cfg(unix)]
    fn fake_claude(script: &str) -> (tempfile::TempDir, ClaudeCode) {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("claude");
        std::fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        (
            dir,
            ClaudeCode {
                program: Some(program),
            },
        )
    }

    /// Waits up to 5 s for process `pid` to be gone.
    #[cfg(unix)]
    async fn gone(pid: &str) -> bool {
        for _ in 0..50 {
            let alive = std::process::Command::new("kill")
                .args(["-0", pid])
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.success());
            if !alive {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        false
    }

    #[cfg(unix)]
    async fn child_pid(dir: &Path) -> String {
        for _ in 0..50 {
            if let Ok(pid) = std::fs::read_to_string(dir.join("child.pid")) {
                let pid = pid.trim().to_string();
                if !pid.is_empty() {
                    return pid;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        panic!("the fake claude wrote no child pid");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_budget_stop_ends_what_the_agent_started_and_keeps_its_last_line() {
        let (dir, harness) = fake_claude(
            r#"sleep 60 & echo $! > child.pid
sleep 0.3
echo '{"type":"assistant","message":{"id":"m1","content":[{"type":"text","text":"about to pass the budget"}],"usage":{"input_tokens":900,"output_tokens":200}}}'
sleep 60"#,
        );
        let mut job = job();
        job.limits.budget_tokens = Some(1000);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (_stop, cancel) = watch::channel(false);
        let ctx = RunContext::default();
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            harness.run(&job, dir.path(), &dir.path().join("mcp.json"), &ctx, tx, cancel),
        )
        .await
        .expect("stopped")
        .unwrap();
        assert_eq!(out.status, "failed");
        let pid = child_pid(dir.path()).await;
        assert!(
            gone(&pid).await,
            "the agent's child {pid} survived the budget stop"
        );
        let mut kinds = Vec::new();
        while let Ok(e) = rx.try_recv() {
            kinds.push((e.kind, e.message));
        }
        let thought = kinds
            .iter()
            .position(|(k, m)| k == "thought" && m.contains("about to pass"));
        let stop = kinds.iter().position(|(_, m)| m.starts_with("budget exhausted"));
        assert!(thought.is_some() && thought < stop, "{kinds:?}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_cancel_ends_what_the_agent_started() {
        let (dir, harness) = fake_claude("sleep 60 & echo $! > child.pid\nsleep 60");
        let (tx, _rx) = mpsc::unbounded_channel();
        let (stop, cancel) = watch::channel(false);
        let ctx = RunContext::default();
        let job = job();
        let mcp = dir.path().join("mcp.json");
        let run = harness.run(&job, dir.path(), &mcp, &ctx, tx, cancel);
        let pid_dir = dir.path().to_path_buf();
        let (out, pid) = tokio::join!(run, async move {
            let pid = child_pid(&pid_dir).await;
            let _ = stop.send(true);
            pid
        });
        assert_eq!(out.unwrap().status, "cancelled");
        assert!(gone(&pid).await, "the agent's child {pid} survived the cancel");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_dropped_run_reports_its_tokens_and_ends_the_group() {
        // the job timeout drops the harness future
        let (dir, harness) = fake_claude(
            r#"sleep 60 & echo $! > child.pid
echo '{"type":"assistant","message":{"id":"m1","content":[],"usage":{"input_tokens":300,"cache_read_input_tokens":700,"output_tokens":50}}}'
sleep 60"#,
        );
        let (tx, _rx) = mpsc::unbounded_channel();
        let (_stop, cancel) = watch::channel(false);
        let ctx = RunContext::default();
        let job = job();
        let timed = tokio::time::timeout(
            std::time::Duration::from_millis(1500),
            harness.run(&job, dir.path(), &dir.path().join("mcp.json"), &ctx, tx, cancel),
        )
        .await;
        assert!(timed.is_err(), "still running at the timeout");
        let outcome = crate::executor::timed_out(&ctx.usage);
        assert_eq!((outcome.tokens_in, outcome.tokens_out), (1000, 50));
        assert_eq!((outcome.cache_creation, outcome.cache_read), (0, 700));
        assert_eq!(outcome.summary.as_deref(), Some("timed out"));
        let pid = child_pid(dir.path()).await;
        assert!(gone(&pid).await, "the agent's child {pid} survived the timeout");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn the_result_totals_win_over_a_smaller_running_sum() {
        let (dir, harness) = fake_claude(
            r#"echo '{"type":"assistant","message":{"id":"m1","content":[],"usage":{"input_tokens":100,"output_tokens":1}}}'
echo '{"type":"result","subtype":"success","is_error":false,"result":"done","usage":{"input_tokens":120,"cache_read_input_tokens":2000,"output_tokens":340}}'"#,
        );
        let (tx, _rx) = mpsc::unbounded_channel();
        let (_stop, cancel) = watch::channel(false);
        let ctx = RunContext::default();
        let mut job = job();
        // the totals pass the budget only at the final result (120 + 340 + 0.1 x 2000 = 660 counted): the agent is
        // done, the run is not failed for it
        job.limits.budget_tokens = Some(500);
        let out = harness
            .run(&job, dir.path(), &dir.path().join("mcp.json"), &ctx, tx, cancel)
            .await
            .unwrap();
        assert_eq!((out.tokens_in, out.tokens_out), (2120, 340));
        assert_eq!(out.status, "succeeded");
        assert_eq!(ctx.usage.get(), (2120, 340));
        assert_eq!((out.cache_read, out.weighted_tokens()), (2000, 660));
    }

    #[test]
    fn result_usage_is_read() {
        let m = map_line(
            r#"{"type":"result","subtype":"success","result":"ok","usage":{"input_tokens":1,"cache_creation_input_tokens":2,"cache_read_input_tokens":3,"output_tokens":4}}"#,
        );
        assert_eq!(
            m.result_usage,
            Some(Usage {
                input: 6,
                output: 4,
                cache_creation: 2,
                cache_read: 3
            })
        );
        assert_eq!(map_line(r#"{"type":"result","result":"ok"}"#).result_usage, None);
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
