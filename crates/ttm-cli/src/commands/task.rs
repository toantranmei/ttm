//! `ttm task list | show | claim | done | approve | split` on the linked project.

use super::notes::read_input;
use super::project::linked;
use crate::app::App;
use crate::cli::TaskCommand;
use crate::output::emit;
use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};
use std::process::ExitCode;
use ttm_api::factory::{SUBTASK_KEYS, Task, normalize_subtask};

pub async fn run(app: App, command: TaskCommand) -> Result<ExitCode> {
    let (client, _) = app.api().await?;
    match command {
        TaskCommand::List { state, mine } => {
            let project = linked()?;
            let me = if mine {
                Some(client.me().await?.user.id)
            } else {
                None
            };
            let tasks: Vec<Task> = client
                .tasks(&project.file.project_id)
                .await?
                .into_iter()
                .filter(|t| state.as_deref().is_none_or(|s| matches_state(t, s)))
                .filter(|t| me.as_deref().is_none_or(|id| t.assignee_id() == Some(id)))
                .collect();
            emit(app.json, &tasks, || {
                for t in &tasks {
                    println!("{}", row(t));
                }
            });
        }
        TaskCommand::Show { key } => {
            let task = client.task(&key).await?;
            emit(app.json, &task, || {
                println!("{}  {}", task.key(), task.title);
                println!(
                    "state: {}  role: {}  assignee: {}",
                    task.state.as_deref().unwrap_or("-"),
                    task.work_role().unwrap_or("-"),
                    task.assignee_name().unwrap_or_else(|| "-".into())
                );
                if let Some(desc) = task.description.as_deref().filter(|d| !d.is_empty()) {
                    println!("\n{desc}");
                }
            });
        }
        TaskCommand::Claim { key } => {
            let me = client.me().await?.user;
            let task = client.task(&key).await?;
            // `assignee` as an object sets the member doing the task; as a string it would set the role.
            let updated = client
                .update_task(&task, &json!({ "assignee": { "kind": "user", "id": me.id } }))
                .await?;
            emit(app.json, &updated, || println!("{} is now yours.", updated.key()));
        }
        TaskCommand::Done { key } => {
            let task = client.transition_task(&key, "done").await?;
            emit(app.json, &task, || {
                println!(
                    "{} is now {}.",
                    task.key(),
                    task.state.as_deref().unwrap_or("done")
                )
            });
        }
        TaskCommand::Approve { key, reject } => {
            let decision = if reject { "reject" } else { "approve" };
            let task = client.approve_task(&key, decision).await?;
            emit(app.json, &task, || {
                println!(
                    "{} {}; state {}.",
                    task.key(),
                    decided(reject),
                    task.state.as_deref().unwrap_or("-")
                )
            });
        }
        TaskCommand::Split { key, file } => {
            let (tasks, warnings) = parse_subtasks(&read_input(&file)?)?;
            for w in &warnings {
                eprintln!("warning: {w}");
            }
            let result = client.create_subtasks(&key, &tasks, None).await?;
            emit(app.json, &result, || {
                let created = result["tasks"].as_array().map(Vec::len).unwrap_or(tasks.len());
                println!("{created} subtask(s) created under {key}.");
                for w in result["warnings"].as_array().into_iter().flatten() {
                    eprintln!("warning: {}", w.as_str().unwrap_or_default());
                }
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `approved` / `rejected`.
fn decided(reject: bool) -> &'static str {
    if reject { "rejected" } else { "approved" }
}

/// `--state` matches the workflow state or its category.
fn matches_state(task: &Task, state: &str) -> bool {
    task.state.as_deref() == Some(state) || task.category.as_deref() == Some(state)
}

/// `AUR-12     doing          DEV       Dev bot          Login`.
pub fn row(t: &Task) -> String {
    format!(
        "{:<10} {:<14} {:<9} {:<16} {}",
        t.key(),
        t.state.as_deref().or(t.category.as_deref()).unwrap_or("-"),
        t.work_role().unwrap_or("-"),
        t.assignee_name().unwrap_or_else(|| "-".into()),
        t.title
    )
}

/// `[…]` or `{"tasks": […]}`; each needs a `title`. Keys may be snake_case (like the `factory-create-subtasks` MCP
/// tool: `acceptance_criteria`, `depends_on`) or camelCase (`acceptanceCriteria`, `dependencies`); they are sent as the
/// server reads them. Returns the tasks and a warning per key the server would ignore (dropped).
pub fn parse_subtasks(text: &str) -> Result<(Vec<Value>, Vec<String>)> {
    let value: Value = serde_json::from_str(text).context("subtasks file is not valid JSON")?;
    let list = match value {
        Value::Array(list) => list,
        Value::Object(mut map) => match map.remove("tasks") {
            Some(Value::Array(list)) => list,
            _ => bail!("expected an array of tasks or {{\"tasks\": [...]}}"),
        },
        _ => bail!("expected an array of tasks"),
    };
    if list.is_empty() {
        bail!("no subtasks in the file");
    }
    let mut tasks = Vec::with_capacity(list.len());
    let mut warnings = Vec::new();
    for (i, task) in list.into_iter().enumerate() {
        if !task.is_object() {
            bail!("subtask #{} is not an object", i + 1);
        }
        let (task, unknown) = normalize_subtask(task);
        if task
            .get("title")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            bail!("subtask #{} has no title", i + 1);
        }
        for key in unknown {
            warnings.push(format!(
                "subtask #{}: unknown key `{key}` ignored (known: {}, or acceptance_criteria, depends_on)",
                i + 1,
                SUBTASK_KEYS.join(", ")
            ));
        }
        tasks.push(task);
    }
    Ok((tasks, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_shows_member_and_role() {
        let t: Task = serde_json::from_value(json!({
            "id": "AUR-12", "key": "AUR-12", "title": "Login", "state": "doing", "assignee": "DEV",
            "assigneeMember": {"kind": "agent", "id": "a1", "name": "Dev bot"}
        }))
        .unwrap();
        let r = row(&t);
        assert!(
            r.starts_with("AUR-12") && r.contains("DEV") && r.contains("Dev bot"),
            "{r}"
        );
        assert!(matches_state(&t, "doing"));
    }

    #[test]
    fn subtask_files() {
        assert_eq!(
            parse_subtasks(r#"[{"title": "A"}, {"title": "B", "role": "QC"}]"#)
                .unwrap()
                .0
                .len(),
            2
        );
        assert_eq!(
            parse_subtasks(r#"{"tasks": [{"title": "A"}]}"#).unwrap().0.len(),
            1
        );
        assert!(parse_subtasks("[]").is_err());
        assert!(parse_subtasks(r#"[{"role": "DEV"}]"#).is_err());
        assert!(parse_subtasks(r#"["A"]"#).is_err());
    }

    #[test]
    fn subtask_files_accept_snake_case_and_warn_on_unknown_keys() {
        let (tasks, warnings) = parse_subtasks(
            r##"[{"title": "A", "acceptance_criteria": ["works"], "depends_on": ["#1"], "owner": "me"},
                {"title": "B", "acceptanceCriteria": ["ok"], "dependencies": ["AUR-3"]}]"##,
        )
        .unwrap();
        assert_eq!(
            tasks[0],
            json!({"title": "A", "acceptanceCriteria": ["works"], "dependencies": ["#1"]})
        );
        assert_eq!(tasks[1]["dependencies"], json!(["AUR-3"]));
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("subtask #1") && warnings[0].contains("`owner`"),
            "{warnings:?}"
        );
    }

    #[test]
    fn decision_wording() {
        assert_eq!((decided(false), decided(true)), ("approved", "rejected"));
    }
}
