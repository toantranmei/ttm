//! `ttm task list | show | claim | done | approve | split` on the linked project.

use super::notes::read_input;
use super::project::linked;
use crate::app::App;
use crate::cli::TaskCommand;
use crate::output::emit;
use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};
use std::process::ExitCode;
use ttm_api::factory::{MemberRef, SUBTASK_KEYS, Task, normalize_subtask};
use ttm_ui::vocab::Phase;
use ttm_ui::{Badge, Callout, Cell, Col, Header, KeyValue, Status, Table, Token, Ui, next_line};

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
            emit(&app, &tasks, |ui| {
                ui.line(&format!(
                    "{} · {} task(s)",
                    ui.paint(Token::Ink, &project.file.key),
                    tasks.len()
                ));
                ui.print(&task_table(ui, &tasks));
                if let Some(first) = tasks.first() {
                    ui.line(&next_line(ui.mode(), &format!("ttm task show {}", first.key())));
                }
            });
        }
        TaskCommand::Show { key } => {
            let task = client.task(&key).await?;
            emit(&app, &task, |ui| show(ui, &task));
        }
        TaskCommand::Claim { key } => {
            let me = client.me().await?.user;
            let task = client.task(&key).await?;
            // `assignee` as an object sets the member doing the task; as a string it would set the role.
            let updated = client
                .update_task(&task, &json!({ "assignee": { "kind": "user", "id": me.id } }))
                .await?;
            emit(&app, &updated, |ui| {
                ui.print(
                    &Callout::success(format!("{} is now yours", updated.key()))
                        .detail(updated.title.as_str())
                        .next(format!("ttm agent run {}", updated.key())),
                )
            });
        }
        TaskCommand::Done { key } => {
            let task = client.transition_task(&key, "done").await?;
            emit(&app, &task, |ui| {
                let state = task.state.as_deref().unwrap_or("done");
                let callout = if task.category.as_deref().is_none_or(|c| c == "done") {
                    Callout::success(format!("{} is now {state}", task.key()))
                } else {
                    // An approval gate stopped it before done.
                    Callout::warning(format!("{} is now {state}", task.key()))
                        .detail("it waits for an approval before done")
                        .next(format!("ttm task approve {}", task.key()))
                };
                ui.print(&callout);
            });
        }
        TaskCommand::Approve { key, reject } => {
            let decision = if reject { "reject" } else { "approve" };
            let task = client.approve_task(&key, decision).await?;
            emit(&app, &task, |ui| {
                ui.print(
                    &Callout::success(format!("{} {}", task.key(), decided(reject)))
                        .detail(format!("state {}", task.state.as_deref().unwrap_or("-")))
                        .next(format!("ttm task show {}", task.key())),
                )
            });
        }
        TaskCommand::Split { key, file } => {
            let (tasks, warnings) = parse_subtasks(&read_input(&file)?)?;
            for w in &warnings {
                app.ui.eprint(&Callout::warning(w.as_str()));
            }
            let result = client.create_subtasks(&key, &tasks, None).await?;
            emit(&app, &result, |ui| {
                let created = result["tasks"].as_array().map(Vec::len).unwrap_or(tasks.len());
                ui.print(&Callout::success(format!(
                    "{created} subtask(s) created under {key}"
                )));
                let mut table = Table::new(&[Col::new("KEY"), Col::new("TITLE").flex(1).min(10)]);
                for t in result["tasks"].as_array().into_iter().flatten() {
                    let key = t["key"].as_str().or(t["id"].as_str()).unwrap_or("-");
                    table.push(vec![
                        Cell::token(key, Token::Ink),
                        Cell::new(t["title"].as_str().unwrap_or("")),
                    ]);
                }
                if !table.is_empty() {
                    ui.print(&table);
                }
                for w in result["warnings"].as_array().into_iter().flatten() {
                    ui.eprint(&Callout::warning(w.as_str().unwrap_or_default()));
                }
                ui.line(&next_line(ui.mode(), "ttm task list"));
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

/// `🤖 Dev bot` / `👤 An` (with a Mei-er badge when the server sends `membership.tier`), or `-`.
pub fn member(ui: &Ui, m: Option<&MemberRef>) -> String {
    let Some(m) = m.filter(|m| !m.name.is_empty()) else {
        return "-".into();
    };
    let badge = if m.kind == "agent" {
        Badge::agent()
    } else {
        Badge::person()
    };
    let mut text = format!("{} {}", ui.render(&badge), m.name);
    if let Some(member) = Badge::of_tier(m.tier()) {
        text.push(' ');
        text.push_str(&ui.render(&member));
    }
    text
}

/// The status of a task: its category, else its state name.
fn status_of(t: &Task) -> Status {
    Status::of_category(t.category.as_deref().or(t.state.as_deref()).unwrap_or("inbox"))
}

fn estimate(t: &Task) -> String {
    match t.estimate {
        Some(e) if e.fract() == 0.0 => format!("{e:.0}"),
        Some(e) => format!("{e}"),
        None => "-".into(),
    }
}

/// Status glyph, KEY, title (flex), role, assignee with badges, estimate.
pub fn task_table(ui: &Ui, tasks: &[Task]) -> Table {
    let mut table = Table::new(&[
        Col::new(""),
        Col::new("KEY"),
        Col::new("TITLE").flex(3).min(10),
        Col::new("ROLE").max(10),
        Col::new("ASSIGNEE").flex(1).min(6),
        Col::new("EST").right(),
    ])
    .empty_text("No tasks.");
    for t in tasks {
        table.push(vec![
            Cell::new(ui.render(&status_of(t))),
            Cell::token(t.key(), Token::Ink),
            Cell::new(t.title.as_str()),
            Cell::new(t.work_role().unwrap_or("-")),
            Cell::new(member(ui, t.assignee_member.as_ref())),
            Cell::new(estimate(t)),
        ]);
    }
    table
}

/// Text of a criterion: a string, or `{text|title, done|checked}`.
fn criterion(value: &Value) -> Option<(String, bool)> {
    match value {
        Value::String(s) => Some((s.clone(), false)),
        Value::Object(o) => {
            let text = o.get("text").or_else(|| o.get("title")).and_then(Value::as_str)?;
            let done = o
                .get("done")
                .or_else(|| o.get("checked"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Some((text.to_string(), done))
        }
        _ => None,
    }
}

/// `ttm task show`: header, key/values, acceptance criteria, description, comments.
fn show(ui: &Ui, task: &Task) {
    let state = task.state.as_deref().unwrap_or("-");
    ui.print(
        &Header::new(format!("{} · {}", task.key(), task.title))
            .summary(format!("{} {state}", ui.render(&status_of(task)))),
    );
    let extra = &task.extra;
    let delegator: Option<MemberRef> = extra
        .get("delegator")
        .or_else(|| extra.get("createdBy"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let sprint = extra.get("sprint").and_then(|v| match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Object(o) => o.get("name").and_then(Value::as_str).map(String::from),
        _ => None,
    });
    let labels: Vec<&str> = extra
        .get("labels")
        .and_then(Value::as_array)
        .map(|l| l.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let kv = KeyValue::new()
        .row(
            "status",
            format!("{state} ({})", task.category.as_deref().unwrap_or("-")),
        )
        .row("role", task.work_role().unwrap_or("-"))
        .row("assignee", member(ui, task.assignee_member.as_ref()))
        .opt("delegator", delegator.as_ref().map(|d| member(ui, Some(d))))
        .opt("sprint", sprint)
        .opt("labels", (!labels.is_empty()).then(|| labels.join(", ")))
        .opt("estimate", task.estimate.map(|_| estimate(task)));
    ui.print(&kv);
    let criteria: Vec<(String, bool)> = extra
        .get("acceptanceCriteria")
        .and_then(Value::as_array)
        .map(|l| l.iter().filter_map(criterion).collect())
        .unwrap_or_default();
    if !criteria.is_empty() {
        ui.blank();
        ui.line(&ui.paint(Token::Ink, "Acceptance criteria"));
        for (text, done) in &criteria {
            let phase = if *done { Phase::Done } else { Phase::Todo };
            ui.line(&format!("  {} {text}", ui.render(&Status::phase(phase))));
        }
    }
    if let Some(desc) = task.description.as_deref().filter(|d| !d.is_empty()) {
        ui.blank();
        ui.line(desc.trim_end());
    }
    let comments = extra
        .get("comments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !comments.is_empty() {
        ui.blank();
        ui.line(&ui.paint(Token::Ink, &format!("Comments ({})", comments.len())));
        for c in &comments {
            let author: Option<MemberRef> = c
                .get("author")
                .and_then(|a| serde_json::from_value(a.clone()).ok());
            let who = match &author {
                Some(a) => member(ui, Some(a)),
                None => c
                    .get("authorName")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .to_string(),
            };
            let body = c
                .get("body")
                .or_else(|| c.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            ui.line(&format!("  {who}: {}", body.replace('\n', " ")));
        }
    }
    let next = match task.category.as_deref() {
        Some("approval") => format!("ttm task approve {}", task.key()),
        Some("done") => "ttm task list".to_string(),
        _ if task.assignee_member.is_none() => format!("ttm task claim {}", task.key()),
        _ => format!("ttm agent run {}", task.key()),
    };
    ui.blank();
    ui.line(&next_line(ui.mode(), &next));
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
            "id": "AUR-12", "key": "AUR-12", "title": "Login", "state": "doing", "category": "in-progress",
            "assignee": "DEV", "assigneeMember": {"kind": "agent", "id": "a1", "name": "Dev bot",
            "ownerMembership": {"tier": "mei-er"}}
        }))
        .unwrap();
        let (ui, _) = Ui::buffered(ttm_ui::Mode::fixed(ttm_ui::ColorLevel::None, false, 80));
        let r = ui.render(&task_table(&ui, std::slice::from_ref(&t)));
        let line = r.lines().nth(2).unwrap();
        assert!(
            line.starts_with("●  AUR-12") && line.contains("DEV") && line.contains("🤖 Dev bot ✦ Mei-er"),
            "{r}"
        );
        assert!(matches_state(&t, "doing"));
        assert_eq!(
            criterion(&json!({"text": "works", "done": true})),
            Some(("works".into(), true))
        );
        assert_eq!(criterion(&json!("plain")), Some(("plain".into(), false)));
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
