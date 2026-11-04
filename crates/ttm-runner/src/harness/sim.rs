//! `ttm-sim`: a scripted fake harness for the end-to-end simulation (feature `sim-harness`, never in releases).
//!
//! It is available only when `TTM_SIM_SCRIPT_DIR` names an existing directory. For a job it reads the first script
//! found of `<task-key>.json`, `role-<ROLE>.json`, `default.json` in that directory:
//!
//! ```json
//! { "steps": [
//!     { "event": "thought", "text": "Reading the criteria of {{task.key}}" },
//!     { "patch": "patches/clear-completed.patch" },
//!     { "write": { "path": "src/x.ts", "content": "…" } },
//!     { "run": "pnpm vitest run", "report_tests": true },
//!     { "mcp": { "tool": "factory-task-comment", "args": { "task": "{{task.key}}", "body": "Model done" } } },
//!     { "usage": { "input": 12000, "output": 2400 } },
//!     { "sleep_ms": 1500 },
//!     { "commit": "feat: clear completed" } ],
//!   "outcome": "succeeded" }
//! ```
//!
//! Outcomes: `succeeded`, `failed` (`summary`), `tests_fail` (`tests_pass = false`), `budget` (`usage` events of 10%
//! of `job.limits.budget_tokens` until it is passed, then waits for the runner or server to stop it), `hang` (until
//! cancelled or timed out), `crash` (an error after `crash_after` steps, default half of them), `pause`
//! (`paused_until` = now + `pause_seconds`, default 60).
//!
//! Events have the kinds Claude Code's adapter sends (`thought`, `action`, `usage`, `progress`, `response`, `log`).
//! Commits go through the job's environment (`ctx.env`: agent identity and the `commit-msg` hook that strips
//! attribution lines), exactly like the real harness. `{{task.key}}`, `{{task.title}}`, `{{task.role}}`, `{{run.id}}`
//! and `{{branch}}` are replaced in every string of the script; title and role are read from the job prompt (the
//! server's context block: `# KEY: title` and `· role ROLE ·`).

pub mod mcp_client;

use super::{Harness, HarnessOutcome, RunContext};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use ttm_api::factory::{JobSpec, RunEvent};

/// Harness name in job specs and runner registration.
pub const NAME: &str = "ttm-sim";
/// Environment variable naming the scripts directory.
pub const SCRIPT_DIR_ENV: &str = "TTM_SIM_SCRIPT_DIR";
/// Timeout of a `run` step.
pub const RUN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// Timeout of an `mcp` step.
pub const MCP_TIMEOUT: Duration = Duration::from_secs(60);
/// Budget used by the `budget` outcome when the job has none.
pub const DEFAULT_BUDGET: u64 = 100_000;

#[derive(Debug, Clone, Default)]
pub struct SimHarness {
    /// Scripts directory (default: `$TTM_SIM_SCRIPT_DIR`, read at each call).
    pub script_dir: Option<PathBuf>,
}

impl SimHarness {
    fn dir(&self) -> Option<PathBuf> {
        self.script_dir
            .clone()
            .or_else(|| std::env::var_os(SCRIPT_DIR_ENV).map(PathBuf::from))
            .filter(|d| d.is_dir())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    #[default]
    Succeeded,
    Failed,
    TestsFail,
    Budget,
    Hang,
    Crash,
    Pause,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WriteStep {
    pub path: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct McpStep {
    pub tool: String,
    #[serde(default)]
    pub args: Value,
    /// Server of the job's MCP config (default `ttm`).
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
}

/// One step of a script (the key of the object says which).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Step {
    Event {
        event: String,
        #[serde(default)]
        text: String,
        #[serde(default)]
        data: Option<Value>,
    },
    Patch {
        patch: String,
    },
    Write {
        write: WriteStep,
    },
    Run {
        run: String,
        #[serde(default)]
        report_tests: bool,
    },
    Mcp {
        mcp: McpStep,
    },
    Usage {
        usage: Usage,
    },
    Sleep {
        sleep_ms: u64,
    },
    Commit {
        commit: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Script {
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub outcome: Outcome,
    /// Final message (`failed`: the error).
    #[serde(default)]
    pub summary: Option<String>,
    /// `crash`: steps done before crashing (default: half of them).
    #[serde(default)]
    pub crash_after: Option<usize>,
    /// `pause`: seconds until `paused_until` (default 60).
    #[serde(default)]
    pub pause_seconds: Option<u64>,
    /// `budget`: pause between usage events (default 500 ms).
    #[serde(default)]
    pub budget_interval_ms: Option<u64>,
    /// `budget`: how long to wait for the stop once the budget is passed (default 60 s); then the run ends anyway.
    #[serde(default)]
    pub budget_grace_ms: Option<u64>,
}

/// Values of the script variables for a job.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vars {
    pub task_key: String,
    pub task_title: String,
    pub task_role: Option<String>,
    pub run_id: String,
    pub branch: String,
}

impl Vars {
    pub fn of(job: &JobSpec) -> Self {
        Self {
            task_key: job.task_key.clone(),
            task_title: task_title(&job.prompt, &job.task_key).unwrap_or_else(|| job.task_key.clone()),
            task_role: task_role(&job.prompt),
            run_id: job.run_id.clone(),
            branch: job.branch.clone(),
        }
    }

    pub fn apply(&self, text: &str) -> String {
        text.replace("{{task.key}}", &self.task_key)
            .replace("{{task.title}}", &self.task_title)
            .replace("{{task.role}}", self.task_role.as_deref().unwrap_or(""))
            .replace("{{run.id}}", &self.run_id)
            .replace("{{branch}}", &self.branch)
    }

    fn apply_value(&self, value: &mut Value) {
        match value {
            Value::String(s) => *s = self.apply(s),
            Value::Array(items) => items.iter_mut().for_each(|v| self.apply_value(v)),
            Value::Object(map) => map.values_mut().for_each(|v| self.apply_value(v)),
            _ => {}
        }
    }
}

/// Title from the context heading `# KEY: title` of the prompt.
pub fn task_title(prompt: &str, key: &str) -> Option<String> {
    let prefix = format!("# {key}: ");
    prompt
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Role from the context line `… · role ROLE · …` of the prompt.
pub fn task_role(prompt: &str) -> Option<String> {
    prompt.lines().find_map(|l| {
        let rest = &l[l.find("· role ")? + "· role ".len()..];
        let role = rest.split('·').next().unwrap_or("").trim();
        (!role.is_empty()).then(|| role.to_string())
    })
}

/// Whether `name` can be part of a script file name (no path separator, no `..`).
fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Script files tried for a job, in order.
pub fn candidates(vars: &Vars) -> Vec<String> {
    let mut out = Vec::new();
    if safe_name(&vars.task_key) {
        out.push(format!("{}.json", vars.task_key));
    }
    if let Some(role) = vars.task_role.as_deref().filter(|r| safe_name(r)) {
        out.push(format!("role-{role}.json"));
    }
    out.push("default.json".into());
    out
}

/// The first existing script of [`candidates`] in `dir`.
pub fn find_script(dir: &Path, vars: &Vars) -> Option<PathBuf> {
    candidates(vars)
        .into_iter()
        .map(|f| dir.join(f))
        .find(|p| p.is_file())
}

/// Reads a script and replaces its variables.
pub fn load_script(path: &Path, vars: &Vars) -> anyhow::Result<Script> {
    let mut value: Value = serde_json::from_slice(&std::fs::read(path)?)
        .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
    vars.apply_value(&mut value);
    serde_json::from_value(value).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))
}

/// `path` inside `root`, refusing absolute paths and `..`.
fn inside(root: &Path, path: &str) -> anyhow::Result<PathBuf> {
    let p = Path::new(path);
    if p.components()
        .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        anyhow::bail!("path `{path}` must be relative and stay inside the worktree");
    }
    Ok(root.join(p))
}

fn short(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}…", text.chars().take(max).collect::<String>())
    }
}

/// Waits `dur` (forever with `None`); `true` when cancelled first.
async fn wait(cancel: &mut watch::Receiver<bool>, dur: Option<Duration>) -> bool {
    if *cancel.borrow() {
        return true;
    }
    let sleep = async {
        match dur {
            Some(d) => tokio::time::sleep(d).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            _ = &mut sleep => return false,
            changed = cancel.changed() => {
                if changed.is_err() || *cancel.borrow() {
                    return true;
                }
            }
        }
    }
}

fn usage_event(input: u64, output: u64) -> RunEvent {
    RunEvent::now(
        "usage",
        format!("usage: {input} in, {output} out"),
        Some(json!({ "usage": { "input_tokens": input, "output_tokens": output } })),
    )
}

/// Where a step left the run.
enum Flow {
    Next,
    Cancelled,
}

struct Ctx<'a> {
    worktree: &'a Path,
    script_dir: &'a Path,
    mcp_config: &'a Path,
    run: &'a RunContext,
    events: &'a mpsc::UnboundedSender<RunEvent>,
}

impl Ctx<'_> {
    fn send(&self, e: RunEvent) {
        let _ = self.events.send(e);
    }

    fn action(&self, tool: &str, summary: String, input: Value) {
        self.send(RunEvent::now(
            "action",
            format!("{tool}: {}", short(&summary, 200)),
            Some(json!({ "tool": tool, "input": input })),
        ));
    }
}

async fn step(
    c: &Ctx<'_>,
    step: &Step,
    out: &mut HarnessOutcome,
    cancel: &mut watch::Receiver<bool>,
) -> anyhow::Result<Flow> {
    match step {
        Step::Event { event, text, data } => {
            c.send(RunEvent::now(event, short(text, 2000), data.clone()));
        }
        Step::Patch { patch } => {
            let file = if Path::new(patch).is_absolute() {
                PathBuf::from(patch)
            } else {
                c.script_dir.join(patch)
            };
            c.action(
                "Bash",
                format!("git apply {patch}"),
                json!({ "command": format!("git apply {patch}") }),
            );
            let file = file.display().to_string();
            ttm_core::worktree::git_env(c.worktree, &["apply", &file], &c.run.env)
                .map_err(|e| anyhow::anyhow!("patch {patch} did not apply: {e}"))?;
        }
        Step::Write { write } => {
            let path = inside(c.worktree, &write.path)?;
            c.action("Write", write.path.clone(), json!({ "file_path": write.path }));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &write.content)?;
        }
        Step::Run { run, report_tests } => {
            c.action("Bash", run.clone(), json!({ "command": run }));
            let child = tokio::process::Command::new("sh")
                .arg("-c")
                .arg(run)
                .envs(c.run.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                .current_dir(c.worktree)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()?;
            let passed = tokio::select! {
                res = tokio::time::timeout(RUN_TIMEOUT, child.wait_with_output()) => match res {
                    Ok(Ok(o)) => {
                        let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                        let tail: String = text.chars().rev().take(500).collect::<Vec<_>>().into_iter().rev().collect();
                        c.send(RunEvent::now("log", format!("exit {}: {}", o.status.code().unwrap_or(-1), tail.trim()), None));
                        o.status.success()
                    }
                    Ok(Err(e)) => {
                        c.send(RunEvent::now("log", format!("command failed to run: {e}"), None));
                        false
                    }
                    Err(_) => {
                        c.send(RunEvent::now("log", format!("command timed out after {} min", RUN_TIMEOUT.as_secs() / 60), None));
                        false
                    }
                },
                cancelled = wait(cancel, None) => {
                    if cancelled {
                        return Ok(Flow::Cancelled);
                    }
                    false
                }
            };
            if *report_tests {
                out.tests_pass = Some(passed);
                out.tests_command = Some(run.clone());
                let verdict = if passed { "passed" } else { "failed" };
                c.send(RunEvent::now(
                    "log",
                    format!("tests {verdict}: {}", short(run, 200)),
                    Some(json!({ "tests_pass": passed, "command": run })),
                ));
            }
        }
        Step::Mcp { mcp } => {
            let server = mcp.server.as_deref().unwrap_or("ttm");
            let tool = format!("mcp__{server}__{}", mcp.tool);
            let args = if mcp.args.is_null() {
                json!({})
            } else {
                mcp.args.clone()
            };
            let call = mcp_client::call(c.mcp_config, server, &mcp.tool, args.clone(), MCP_TIMEOUT);
            let result = tokio::select! {
                r = call => r,
                cancelled = wait(cancel, None) => {
                    if cancelled {
                        return Ok(Flow::Cancelled);
                    }
                    Err(anyhow::anyhow!("stopped"))
                }
            };
            let data = match &result {
                Ok(r) => json!({ "tool": tool, "input": args, "result": r }),
                Err(e) => json!({ "tool": tool, "input": args, "error": e.to_string() }),
            };
            c.send(RunEvent::now("action", tool.clone(), Some(data)));
            if let Err(e) = result {
                c.send(RunEvent::now("log", format!("{tool} failed: {e}"), None));
            }
        }
        Step::Usage { usage } => {
            out.tokens_in += usage.input;
            out.tokens_out += usage.output;
            c.send(usage_event(usage.input, usage.output));
        }
        Step::Sleep { sleep_ms } => {
            if wait(cancel, Some(Duration::from_millis(*sleep_ms))).await {
                return Ok(Flow::Cancelled);
            }
        }
        Step::Commit { commit } => {
            c.action(
                "Bash",
                format!("git commit -m {commit}"),
                json!({ "command": format!("git commit -m {}", crate::identity::sh_quote(commit)) }),
            );
            if crate::git::is_dirty(c.worktree)? {
                // The job's identity and commit-msg hook (attribution lines stripped), as for the real harness.
                crate::git::commit_all(c.worktree, commit, &c.run.env)?;
            } else {
                c.send(RunEvent::now("log", "nothing to commit", None));
            }
        }
    }
    Ok(Flow::Next)
}

fn cancelled(mut out: HarnessOutcome) -> HarnessOutcome {
    out.status = "cancelled".into();
    out
}

fn respond(c: &Ctx<'_>, text: &str, ok: bool, turns: usize) {
    let subtype = if ok { "success" } else { "error_during_execution" };
    c.send(RunEvent::now(
        "response",
        short(text, 4000),
        Some(json!({ "num_turns": turns, "subtype": subtype })),
    ));
}

#[async_trait]
impl Harness for SimHarness {
    fn name(&self) -> &'static str {
        NAME
    }

    fn available(&self) -> bool {
        self.dir().is_some()
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
        let dir = self
            .dir()
            .ok_or_else(|| anyhow::anyhow!("{SCRIPT_DIR_ENV} is not set or not a directory"))?;
        let vars = Vars::of(job);
        let path = find_script(&dir, &vars).ok_or_else(|| {
            anyhow::anyhow!(
                "no simulated script for {} in {} (tried {})",
                job.task_key,
                dir.display(),
                candidates(&vars).join(", ")
            )
        })?;
        let script = load_script(&path, &vars)?;
        let c = Ctx {
            worktree,
            script_dir: &dir,
            mcp_config,
            run: ctx,
            events: &events,
        };
        c.send(RunEvent::now(
            "log",
            format!(
                "agent started {NAME} ({})",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
            None,
        ));

        let mut out = HarnessOutcome::default();
        let crash_at =
            (script.outcome == Outcome::Crash).then(|| script.crash_after.unwrap_or(script.steps.len() / 2));
        for (i, s) in script.steps.iter().enumerate() {
            if crash_at == Some(i) {
                anyhow::bail!("{NAME} crashed after {i} step(s) (simulated)");
            }
            if *cancel.borrow() {
                return Ok(cancelled(out));
            }
            if let Flow::Cancelled = step(&c, s, &mut out, &mut cancel).await? {
                return Ok(cancelled(out));
            }
        }
        if crash_at.is_some() {
            anyhow::bail!("{NAME} crashed after {} step(s) (simulated)", script.steps.len());
        }
        let turns = script.steps.len();
        match script.outcome {
            Outcome::Succeeded | Outcome::TestsFail => {
                if script.outcome == Outcome::TestsFail {
                    out.tests_pass = Some(false);
                    out.tests_command.get_or_insert_with(|| "tests".into());
                }
                let text = script
                    .summary
                    .unwrap_or_else(|| format!("Done: {} {}", vars.task_key, vars.task_title));
                respond(&c, &text, true, turns);
                out.summary = Some(text);
                out.status = "succeeded".into();
            }
            Outcome::Failed => {
                let text = script.summary.unwrap_or_else(|| "simulated failure".into());
                respond(&c, &text, false, turns);
                out.summary = Some(text);
                out.status = "failed".into();
            }
            Outcome::Pause => {
                let at = time::OffsetDateTime::now_utc()
                    + time::Duration::seconds(script.pause_seconds.unwrap_or(60) as i64);
                let text = format!("usage limit reached|{}", at.unix_timestamp());
                respond(&c, &text, false, turns);
                out.paused_until = at.format(&time::format_description::well_known::Rfc3339).ok();
                out.summary = Some(text);
                out.status = "paused".into();
            }
            Outcome::Hang => {
                c.send(RunEvent::now("progress", "waiting (simulated hang)", None));
                wait(&mut cancel, None).await;
                return Ok(cancelled(out));
            }
            Outcome::Budget => {
                let budget = job.limits.budget_tokens.unwrap_or(DEFAULT_BUDGET).max(1);
                let chunk = (budget / 10).max(1);
                let interval = Duration::from_millis(script.budget_interval_ms.unwrap_or(500));
                loop {
                    let input = chunk * 4 / 5;
                    let output = chunk - input;
                    out.tokens_in += input;
                    out.tokens_out += output;
                    c.send(usage_event(input, output));
                    if out.tokens_in + out.tokens_out > budget {
                        break;
                    }
                    if wait(&mut cancel, Some(interval)).await {
                        break;
                    }
                }
                if !*cancel.borrow() {
                    let grace = Duration::from_millis(script.budget_grace_ms.unwrap_or(60_000));
                    wait(&mut cancel, Some(grace)).await;
                }
                out.summary = Some("budget exhausted".into());
                out.status = "failed".into();
            }
            Outcome::Crash => unreachable!("handled above"),
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROMPT: &str = "Do task TODO-3 on branch `ttm/todo-3-clear`.\n\n---\n\n# TODO-3: Clear completed\n\nProject Todo (TODO) · state `doing` · role DEV · priority 2 · estimate 3\n";

    fn job() -> JobSpec {
        serde_json::from_value(json!({
            "run_id": "r9", "task_key": "TODO-3", "branch": "ttm/todo-3-clear", "prompt": PROMPT, "harness": NAME,
            "mcp": {"token": "ttm_job_x"}
        }))
        .unwrap()
    }

    #[test]
    fn vars_from_the_prompt() {
        let v = Vars::of(&job());
        assert_eq!(v.task_title, "Clear completed");
        assert_eq!(v.task_role.as_deref(), Some("DEV"));
        assert_eq!(
            v.apply("{{task.key}} {{task.title}} {{run.id}} {{task.role}}"),
            "TODO-3 Clear completed r9 DEV"
        );
        assert_eq!(task_title("no heading", "X-1"), None);
    }

    #[test]
    fn lookup_order() {
        let v = Vars::of(&job());
        assert_eq!(candidates(&v), ["TODO-3.json", "role-DEV.json", "default.json"]);
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("default.json"), "{}").unwrap();
        assert!(find_script(dir.path(), &v).unwrap().ends_with("default.json"));
        std::fs::write(dir.path().join("role-DEV.json"), "{}").unwrap();
        assert!(find_script(dir.path(), &v).unwrap().ends_with("role-DEV.json"));
        std::fs::write(dir.path().join("TODO-3.json"), "{}").unwrap();
        assert!(find_script(dir.path(), &v).unwrap().ends_with("TODO-3.json"));
        let odd = Vars {
            task_key: "../x".into(),
            task_role: Some("a/b".into()),
            ..v
        };
        assert_eq!(candidates(&odd), ["default.json"]);
    }

    #[test]
    fn parses_every_step() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.json");
        std::fs::write(
            &path,
            r#"{"steps":[{"event":"thought","text":"Read {{task.key}}"},{"patch":"p.patch"},
            {"write":{"path":"a.txt","content":"x"}},{"run":"true","report_tests":true},
            {"mcp":{"tool":"factory-task-comment","args":{"task":"{{task.key}}"}}},{"usage":{"input":1,"output":2}},
            {"sleep_ms":5},{"commit":"feat: {{task.title}}"}],"outcome":"tests_fail"}"#,
        )
        .unwrap();
        let s = load_script(&path, &Vars::of(&job())).unwrap();
        assert_eq!(s.outcome, Outcome::TestsFail);
        assert_eq!(s.steps.len(), 8);
        assert!(
            matches!(&s.steps[0], Step::Event { event, text, .. } if event == "thought" && text == "Read TODO-3")
        );
        assert!(matches!(&s.steps[1], Step::Patch { .. }));
        assert!(matches!(&s.steps[2], Step::Write { .. }));
        assert!(matches!(
            &s.steps[3],
            Step::Run {
                report_tests: true,
                ..
            }
        ));
        assert!(matches!(&s.steps[4], Step::Mcp { mcp } if mcp.args["task"] == "TODO-3"));
        assert!(matches!(&s.steps[5], Step::Usage { usage } if usage.output == 2));
        assert!(matches!(&s.steps[6], Step::Sleep { sleep_ms: 5 }));
        assert!(matches!(&s.steps[7], Step::Commit { commit } if commit == "feat: Clear completed"));
    }

    #[test]
    fn paths_stay_in_the_worktree() {
        let root = Path::new("/w");
        assert!(inside(root, "src/a.ts").is_ok());
        assert!(inside(root, "../a").is_err());
        assert!(inside(root, "/etc/passwd").is_err());
    }
}
