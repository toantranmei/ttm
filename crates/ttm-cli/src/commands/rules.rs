//! `ttm rules apply`: the project's rules for local agents.
//!
//! - A managed block between `<!-- ttm:begin -->` and `<!-- ttm:end -->` in `AGENTS.md` and `CLAUDE.md` (created if
//!   missing; the rest of the files is never touched).
//! - Agent hooks in `.claude/settings.json`: `PreToolUse` (matcher `Bash`) → `ttm hook pre-bash`, `Stop` →
//!   `ttm hook stop`, merged with existing hooks.
//! - The rule list cached in `.ttm/cache/rules.json` for the hooks (they work offline).

use super::project::linked;
use crate::app::App;
use crate::cli::RulesCommand;
use crate::output::emit;
use anyhow::{Context as _, Result, bail};
use serde_json::{Map, Value, json};
use std::path::Path;
use std::process::ExitCode;
use ttm_api::factory::RuleSet;

pub const BEGIN: &str = "<!-- ttm:begin -->";
pub const END: &str = "<!-- ttm:end -->";
pub const PRE_BASH: &str = "ttm hook pre-bash";
pub const STOP: &str = "ttm hook stop";

pub async fn run(app: App, command: RulesCommand) -> Result<ExitCode> {
    let RulesCommand::Apply = command;
    let linked = linked()?;
    let (client, _) = app.api().await?;
    let rules = client.rules(&linked.file.project_id).await?;
    let changed = apply(&linked.root, &linked.cache_dir(), &rules)?;
    emit(
        &app,
        &json!({ "changed": changed, "checks": rules.checks }),
        |ui| {
            if changed.is_empty() {
                ui.print(&ttm_ui::Callout::success("Rules already up to date"));
            } else {
                ui.print(
                    &ttm_ui::Callout::success(format!("Rules written to {} file(s)", changed.len()))
                        .details(changed.iter().map(|f| format!("updated {f}")))
                        .next("git add AGENTS.md CLAUDE.md .claude/settings.json"),
                );
            }
        },
    );
    Ok(ExitCode::SUCCESS)
}

/// Writes everything; returns the files that changed.
pub fn apply(root: &Path, cache_dir: &Path, rules: &RuleSet) -> Result<Vec<String>> {
    let mut changed = Vec::new();
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let path = root.join(name);
        let current = std::fs::read_to_string(&path).ok();
        let next = managed_block(current.as_deref(), &rules.managed_markdown)?;
        if current.as_deref() != Some(next.as_str()) {
            std::fs::write(&path, next).with_context(|| format!("writing {}", path.display()))?;
            changed.push(name.to_string());
        }
    }
    let settings = root.join(".claude").join("settings.json");
    let current = std::fs::read_to_string(&settings).ok();
    let next = merge_hooks(current.as_deref())?;
    if current.as_deref() != Some(next.as_str()) {
        std::fs::create_dir_all(settings.parent().unwrap_or(root))?;
        std::fs::write(&settings, next)?;
        changed.push(".claude/settings.json".into());
    }
    std::fs::create_dir_all(cache_dir)?;
    std::fs::write(cache_dir.join("rules.json"), serde_json::to_vec_pretty(rules)?)?;
    Ok(changed)
}

/// Replaces (or appends) the managed block.
pub fn managed_block(current: Option<&str>, markdown: &str) -> Result<String> {
    let block = format!("{BEGIN}\n{}\n{END}", markdown.trim());
    let Some(text) = current else {
        return Ok(format!("{block}\n"));
    };
    match (text.find(BEGIN), text.find(END)) {
        (Some(start), Some(end)) if end > start => {
            Ok(format!("{}{block}{}", &text[..start], &text[end + END.len()..]))
        }
        (None, None) => {
            let sep = if text.is_empty() || text.ends_with("\n\n") {
                ""
            } else if text.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            Ok(format!("{text}{sep}{block}\n"))
        }
        _ => bail!("the ttm markers are broken (one of {BEGIN} / {END} is missing); fix the file and retry"),
    }
}

/// Adds our hooks to Claude Code settings JSON unless an identical command is already there.
pub fn merge_hooks(current: Option<&str>) -> Result<String> {
    let mut root: Value = match current.map(str::trim).filter(|t| !t.is_empty()) {
        Some(text) => serde_json::from_str(text).context(".claude/settings.json is not valid JSON")?,
        None => Value::Object(Map::new()),
    };
    let Some(obj) = root.as_object_mut() else {
        bail!(".claude/settings.json is not an object")
    };
    let hooks = obj.entry("hooks").or_insert_with(|| json!({}));
    let Some(hooks) = hooks.as_object_mut() else {
        bail!("`hooks` is not an object")
    };
    for (event, matcher, command) in [("PreToolUse", Some("Bash"), PRE_BASH), ("Stop", None, STOP)] {
        let list = hooks.entry(event).or_insert_with(|| json!([]));
        let Some(list) = list.as_array_mut() else {
            bail!("`hooks.{event}` is not an array")
        };
        let present = list.iter().any(|group| {
            group["hooks"]
                .as_array()
                .is_some_and(|hs| hs.iter().any(|h| h["command"].as_str() == Some(command)))
        });
        if !present {
            let mut group = json!({ "hooks": [{ "type": "command", "command": command }] });
            if let Some(m) = matcher {
                group["matcher"] = json!(m);
            }
            list.push(group);
        }
    }
    let mut text = serde_json::to_string_pretty(&root)?;
    text.push('\n');
    if let Some(cur) = current
        && serde_json::from_str::<Value>(cur).ok().as_ref() == Some(&root)
    {
        return Ok(cur.to_string());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_block_replace_append_create() {
        assert_eq!(
            managed_block(None, "Rule 1").unwrap(),
            format!("{BEGIN}\nRule 1\n{END}\n")
        );
        let appended = managed_block(Some("# Mine\n"), "Rule 1").unwrap();
        assert!(appended.starts_with("# Mine\n\n<!-- ttm:begin -->"));
        let replaced = managed_block(Some(&format!("A\n{BEGIN}\nold\n{END}\nB\n")), "new").unwrap();
        assert_eq!(replaced, format!("A\n{BEGIN}\nnew\n{END}\nB\n"));
        assert!(managed_block(Some(&format!("A\n{BEGIN}\n")), "x").is_err());
    }

    #[test]
    fn hooks_merge_keeps_existing_and_is_idempotent() {
        let current = r#"{"permissions": {"allow": ["Bash(ls)"]}, "hooks": {"PreToolUse": [{"matcher": "Edit", "hooks": [{"type": "command", "command": "fmt"}]}]}}"#;
        let merged = merge_hooks(Some(current)).unwrap();
        let v: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(v["permissions"]["allow"][0], "Bash(ls)");
        assert_eq!(v["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        assert_eq!(v["hooks"]["PreToolUse"][1]["matcher"], "Bash");
        assert_eq!(v["hooks"]["Stop"][0]["hooks"][0]["command"], STOP);
        assert_eq!(merge_hooks(Some(&merged)).unwrap(), merged);
    }

    #[test]
    fn apply_writes_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# Team notes\n").unwrap();
        let rules = RuleSet {
            managed_markdown: "- No push to main".into(),
            ..RuleSet::default()
        };
        let changed = apply(dir.path(), &dir.path().join(".ttm/cache"), &rules).unwrap();
        assert_eq!(changed, ["AGENTS.md", "CLAUDE.md", ".claude/settings.json"]);
        assert!(
            std::fs::read_to_string(dir.path().join("CLAUDE.md"))
                .unwrap()
                .starts_with("# Team notes")
        );
        assert!(
            apply(dir.path(), &dir.path().join(".ttm/cache"), &rules)
                .unwrap()
                .is_empty()
        );
    }
}
