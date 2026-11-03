//! `ttm task list | show | claim | done | approve | split` on the linked project.

use super::notes::read_input;
use super::project::linked;
use crate::app::App;
use crate::cli::TaskCommand;
use crate::output::emit;
use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};
use std::process::ExitCode;
use ttm_api::factory::Task;

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
                .filter(|t| {
                    state
                        .as_deref()
                        .is_none_or(|s| t.state.as_deref() == Some(s) || t.category.as_deref() == Some(s))
                })
                .filter(|t| {
                    me.as_deref()
                        .is_none_or(|id| assignee_id(t).as_deref() == Some(id))
                })
                .collect();
            emit(app.json, &tasks, || {
                for t in &tasks {
                    println!(
                        "{:<10} {:<14} {:<9} {:<16} {}",
                        t.key,
                        t.state.as_deref().or(t.category.as_deref()).unwrap_or("-"),
                        t.role.as_deref().unwrap_or("-"),
                        t.assignee_name().unwrap_or_else(|| "-".into()),
                        t.title
                    );
                }
            });
        }
        TaskCommand::Show { key } => {
            let task = client.task(&key).await?;
            emit(app.json, &task, || {
                println!("{}  {}", task.key, task.title);
                println!(
                    "state: {}  role: {}  assignee: {}",
                    task.state.as_deref().unwrap_or("-"),
                    task.role.as_deref().unwrap_or("-"),
                    task.assignee_name().unwrap_or_else(|| "-".into())
                );
                if let Some(desc) = task.extra.get("description").and_then(Value::as_str) {
                    println!("\n{desc}");
                }
            });
        }
        TaskCommand::Claim { key } => {
            let me = client.me().await?.user;
            let task = client.task(&key).await?;
            let updated = client
                .update_task(&task, &json!({ "assignee": { "kind": "user", "id": me.id } }))
                .await?;
            emit(app.json, &updated, || println!("{} is now yours.", updated.key));
        }
        TaskCommand::Done { key } => {
            let task = client.task(&key).await?;
            let result = client.transition_task(&task.id, "done").await?;
            emit(app.json, &result, || println!("{} moved to done.", task.key));
        }
        TaskCommand::Approve { key, reject } => {
            let task = client.task(&key).await?;
            let decision = if reject { "reject" } else { "approve" };
            let result = client.approve_task(&task.id, decision).await?;
            emit(app.json, &result, || println!("{} {}d.", task.key, decision));
        }
        TaskCommand::Split { key, file } => {
            let tasks = parse_subtasks(&read_input(&file)?)?;
            let task = client.task(&key).await?;
            let result = client.create_subtasks(&task.id, &tasks, None).await?;
            emit(app.json, &result, || {
                println!("{} subtask(s) created under {}.", tasks.len(), task.key)
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn assignee_id(task: &Task) -> Option<String> {
    task.assignee.as_ref()?.get("id")?.as_str().map(String::from)
}

/// `[…]` or `{"tasks": […]}`; each needs a `title`.
pub fn parse_subtasks(text: &str) -> Result<Vec<Value>> {
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
    if let Some(i) = list
        .iter()
        .position(|t| t.get("title").and_then(Value::as_str).is_none_or(str::is_empty))
    {
        bail!("subtask #{} has no title", i + 1);
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtask_files() {
        assert_eq!(
            parse_subtasks(r#"[{"title": "A"}, {"title": "B", "role": "QC"}]"#)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(parse_subtasks(r#"{"tasks": [{"title": "A"}]}"#).unwrap().len(), 1);
        assert!(parse_subtasks("[]").is_err());
        assert!(parse_subtasks(r#"[{"role": "DEV"}]"#).is_err());
    }
}
