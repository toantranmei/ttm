//! `ttm hook pre-bash | stop`: called by the coding agent's hooks with the hook JSON on stdin. Exit 2 blocks the
//! action and the message on stderr goes back to the agent. Results are reported as `rule_check` events (best effort,
//! short timeout, skipped offline). Outside a linked project the hooks allow everything.

use crate::app::App;
use crate::cli::HookCommand;
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;
use ttm_api::factory::{RuleCheckReport, RuleSet};
use ttm_core::project_file::Linked;
use ttm_core::worktree;

pub const NO_PUSH_MAIN: &str = "no_push_main";
pub const NO_VERIFY: &str = "no_verify";
pub const CONVENTIONAL: &str = "conventional_commits";
pub const TESTS_BEFORE_DONE: &str = "tests_before_done";

const PROTECTED: [&str; 2] = ["main", "master"];
const TYPES: [&str; 12] = [
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert", "merge",
];

pub async fn run(app: App, command: HookCommand) -> Result<ExitCode> {
    let mut input = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut input);
    let hook: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
    let cwd = hook["cwd"]
        .as_str()
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);
    let Some(linked) = Linked::find(&cwd)? else {
        return Ok(ExitCode::SUCCESS);
    };
    let rules = cached_rules(&linked);
    let verdicts = match command {
        HookCommand::PreBash => {
            let command = hook["tool_input"]["command"].as_str().unwrap_or_default();
            let branch = worktree::current_branch(&cwd).ok();
            check_command(command, branch.as_deref(), &rules)
        }
        HookCommand::Stop => {
            if hook["stop_hook_active"].as_bool() == Some(true) {
                return Ok(ExitCode::SUCCESS);
            }
            let transcript = hook["transcript_path"]
                .as_str()
                .and_then(|p| std::fs::read_to_string(p).ok());
            check_stop(transcript.as_deref(), &rules)
        }
    };
    report(&app, &linked, &verdicts).await;
    match verdicts.iter().find(|v| !v.passed) {
        Some(blocked) => {
            eprintln!("Blocked by project rule `{}`: {}", blocked.rule, blocked.message);
            Ok(ExitCode::from(2))
        }
        None => Ok(ExitCode::SUCCESS),
    }
}

/// Rules from `.ttm/cache/rules.json` (written by `ttm rules apply`); without it, every built-in rule is on.
fn cached_rules(linked: &Linked) -> RuleSet {
    std::fs::read(linked.cache_dir().join("rules.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(default_rules)
}

pub fn default_rules() -> RuleSet {
    let check = |id: &str| ttm_api::factory::RuleCheck {
        id: id.into(),
        description: None,
        command: None,
        enabled: true,
    };
    RuleSet {
        checks: vec![check(NO_PUSH_MAIN), check(NO_VERIFY), check(CONVENTIONAL)],
        ..RuleSet::default()
    }
}

fn enabled<'a>(rules: &'a RuleSet, id: &str) -> Option<&'a ttm_api::factory::RuleCheck> {
    rules.checks.iter().find(|c| c.id == id && c.enabled)
}

fn verdict(rule: &str, passed: bool, message: impl Into<String>) -> RuleCheckReport {
    RuleCheckReport {
        rule: rule.into(),
        passed,
        message: message.into(),
        task: std::env::var("TTM_TASK").ok(),
        run_id: std::env::var("TTM_RUN_ID").ok(),
    }
}

/// Splits a shell command line into words, honouring quotes; `&&`, `||`, `;`, `|` become their own words.
pub fn shell_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let flush = |cur: &mut String, words: &mut Vec<String>| {
        if !cur.is_empty() {
            words.push(std::mem::take(cur));
        }
    };
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), '\\') => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => quote = Some(c),
            (None, c) if c.is_whitespace() => flush(&mut cur, &mut words),
            (None, ';') => {
                flush(&mut cur, &mut words);
                words.push(";".into());
            }
            (None, '&' | '|') => {
                flush(&mut cur, &mut words);
                let mut op = c.to_string();
                if chars.peek() == Some(&c) {
                    op.push(c);
                    chars.next();
                }
                words.push(op);
            }
            (None, c) => cur.push(c),
        }
    }
    flush(&mut cur, &mut words);
    words
}

/// The `git` invocations of a command line: the words after `git` (global `-C x` / `-c k=v` skipped).
fn git_calls(line: &str) -> Vec<Vec<String>> {
    let words = shell_words(line);
    let mut calls = Vec::new();
    for segment in words.split(|w| matches!(w.as_str(), "&&" | "||" | ";" | "|" | "&")) {
        let Some(pos) = segment.iter().position(|w| w == "git" || w.ends_with("/git")) else {
            continue;
        };
        let mut rest = segment[pos + 1..].iter().cloned().peekable();
        let mut args = Vec::new();
        while let Some(w) = rest.next() {
            if args.is_empty() && (w == "-C" || w == "-c") {
                rest.next();
                continue;
            }
            args.push(w);
        }
        calls.push(args);
    }
    calls
}

/// Evaluates the Bash rules for one command.
pub fn check_command(line: &str, branch: Option<&str>, rules: &RuleSet) -> Vec<RuleCheckReport> {
    let mut out = Vec::new();
    for args in git_calls(line) {
        let Some(sub) = args.first().map(String::as_str) else {
            continue;
        };
        if !matches!(sub, "push" | "commit") {
            continue;
        }
        if enabled(rules, NO_VERIFY).is_some() {
            let bad = args.iter().any(|a| a == "--no-verify")
                || (sub == "commit" && args.iter().any(|a| a == "-n"));
            out.push(verdict(
                NO_VERIFY,
                !bad,
                if bad {
                    "skipping git hooks (--no-verify) is not allowed"
                } else {
                    "ok"
                },
            ));
        }
        if sub == "push" && enabled(rules, NO_PUSH_MAIN).is_some() {
            let refspecs: Vec<&String> = args[1..].iter().filter(|a| !a.starts_with('-')).skip(1).collect();
            let targets_main = refspecs.iter().any(|r| {
                let dst = r
                    .rsplit(':')
                    .next()
                    .unwrap_or(r)
                    .trim_start_matches('+')
                    .trim_start_matches("refs/heads/");
                PROTECTED.contains(&dst)
            });
            let bare_on_main = refspecs.is_empty() && branch.is_some_and(|b| PROTECTED.contains(&b));
            let bad = targets_main || bare_on_main;
            out.push(verdict(
                NO_PUSH_MAIN,
                !bad,
                if bad {
                    "do not push to main/master directly; push a branch and open a pull request"
                } else {
                    "ok"
                },
            ));
        }
        if sub == "commit"
            && enabled(rules, CONVENTIONAL).is_some()
            && let Some(message) = commit_message(&args)
        {
            {
                let ok = is_conventional(&message);
                out.push(verdict(
                    CONVENTIONAL,
                    ok,
                    if ok {
                        "ok".to_string()
                    } else {
                        format!("commit message must follow Conventional Commits (`type(scope): summary`), got `{}`", message.lines().next().unwrap_or(""))
                    },
                ));
            }
        }
    }
    out
}

/// `-m msg`, `-mmsg`, `--message=msg`, `--message msg`.
fn commit_message(args: &[String]) -> Option<String> {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "-m" || a == "--message" {
            return it.next().cloned();
        }
        if let Some(m) = a.strip_prefix("--message=") {
            return Some(m.to_string());
        }
        if let Some(m) = a
            .strip_prefix("-m")
            .filter(|m| !m.is_empty() && !a.starts_with("--"))
        {
            return Some(m.to_string());
        }
    }
    None
}

pub fn is_conventional(message: &str) -> bool {
    let first = message.lines().next().unwrap_or("").trim();
    let Some((head, summary)) = first.split_once(": ") else {
        return false;
    };
    if summary.trim().is_empty() {
        return false;
    }
    let head = head.strip_suffix('!').unwrap_or(head);
    let (kind, scope_ok) = match head.split_once('(') {
        Some((kind, rest)) => (kind, rest.ends_with(')') && rest.len() > 1),
        None => (head, true),
    };
    scope_ok && TYPES.contains(&kind)
}

/// `tests_before_done`: the test command must appear in the transcript.
pub fn check_stop(transcript: Option<&str>, rules: &RuleSet) -> Vec<RuleCheckReport> {
    let Some(rule) = enabled(rules, TESTS_BEFORE_DONE) else {
        return vec![];
    };
    let Some(command) = rule.command.as_deref().filter(|c| !c.is_empty()) else {
        return vec![];
    };
    let ran = transcript.is_some_and(|t| t.contains(command));
    vec![verdict(
        TESTS_BEFORE_DONE,
        ran,
        if ran {
            "tests were run".to_string()
        } else {
            format!("run `{command}` and fix failures before finishing")
        },
    )]
}

async fn report(app: &App, linked: &Linked, verdicts: &[RuleCheckReport]) {
    if verdicts.is_empty() {
        return;
    }
    let send = async {
        let (client, _) = app.api().await.ok()?;
        for v in verdicts {
            let _ = client.report_rule_check(&linked.file.project_id, v).await;
        }
        Some(())
    };
    let _ = tokio::time::timeout(Duration::from_secs(3), send).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocked(line: &str, branch: Option<&str>) -> Vec<String> {
        check_command(line, branch, &default_rules())
            .into_iter()
            .filter(|v| !v.passed)
            .map(|v| v.rule)
            .collect()
    }

    #[test]
    fn words_and_operators() {
        assert_eq!(
            shell_words(r#"git commit -m "feat: a b" && git push"#),
            ["git", "commit", "-m", "feat: a b", "&&", "git", "push"]
        );
    }

    #[test]
    fn push_to_main_is_blocked() {
        assert_eq!(blocked("git push origin main", Some("feature")), [NO_PUSH_MAIN]);
        assert_eq!(
            blocked("git push origin HEAD:refs/heads/master", Some("x")),
            [NO_PUSH_MAIN]
        );
        assert_eq!(blocked("git push", Some("main")), [NO_PUSH_MAIN]);
        assert!(blocked("git push -u origin feature/login", Some("feature/login")).is_empty());
        assert!(blocked("git push origin main-docs", Some("x")).is_empty());
        assert_eq!(
            blocked("cd x && git -C repo push origin main", None),
            [NO_PUSH_MAIN]
        );
    }

    #[test]
    fn no_verify_and_conventional_commits() {
        assert_eq!(blocked("git commit --no-verify -m 'feat: x'", None), [NO_VERIFY]);
        assert_eq!(blocked("git commit -m 'updated stuff'", None), [CONVENTIONAL]);
        assert!(blocked("git commit -m 'fix(api): retry on 503'", None).is_empty());
        assert!(
            blocked("git commit", None).is_empty(),
            "editor messages are not checked here"
        );
        assert!(blocked("ls -la", None).is_empty());
    }

    #[test]
    fn conventional_parser() {
        assert!(is_conventional("feat!: drop v0"));
        assert!(is_conventional("chore(deps): bump"));
        assert!(!is_conventional("feat(): x"));
        assert!(!is_conventional("feature: x"));
        assert!(!is_conventional("fix:x"));
    }

    #[test]
    fn stop_requires_tests_when_configured() {
        let mut rules = RuleSet::default();
        assert!(check_stop(None, &rules).is_empty());
        rules.checks.push(ttm_api::factory::RuleCheck {
            id: TESTS_BEFORE_DONE.into(),
            description: None,
            command: Some("cargo test".into()),
            enabled: true,
        });
        assert!(!check_stop(Some(r#"{"tool":"Bash","command":"ls"}"#), &rules)[0].passed);
        assert!(
            check_stop(
                Some(r#"{"tool":"Bash","command":"cargo test --workspace"}"#),
                &rules
            )[0]
            .passed
        );
    }
}
