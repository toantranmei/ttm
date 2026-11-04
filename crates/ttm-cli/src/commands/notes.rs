//! `ttm notes …`: Notee from the terminal. Output is Markdown for people, JSON with `--json`.

use crate::app::App;
use crate::cli::{NotesCommand, NotesCommentArgs, NotesCreateArgs, NotesEditArgs, RowsCommand};
use crate::output::emit;
use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::notee::{
    ContentEdit, NewAnchor, NewPage, Parent, QueryRequest, RowUpsert, TextQuote, UpdatePage, WriteMode,
    resource_id,
};

pub async fn run(app: App, command: NotesCommand) -> Result<ExitCode> {
    let (client, _) = app.api().await?;
    match command {
        NotesCommand::Search {
            query,
            limit,
            workspace,
        } => {
            let workspace = workspace.or_else(|| app.ctx.workspace());
            let mut hits = client.search(&query, workspace.as_deref()).await?;
            hits.truncate(limit);
            emit(&app, &hits, |_ui| {
                if hits.is_empty() {
                    println!("No pages found.");
                }
                for h in &hits {
                    println!("{}  {}", h.id, h.title);
                    if !h.snippet.is_empty() {
                        println!("    {}", h.snippet.replace('\n', " "));
                    }
                }
            });
        }
        NotesCommand::Get { id, block, output } => {
            let id = resource_id(&id);
            let summary_kind = match block {
                Some(_) => "page".to_string(),
                None => match client.resource_summary(&id).await {
                    Ok(s) => s.kind,
                    Err(err) if err.status() == Some(404) => "page".into(),
                    Err(err) => return Err(err.into()),
                },
            };
            if summary_kind == "database" {
                let schema = client.database_schema(&id).await?;
                emit(&app, &schema, |_ui| {
                    println!("# {}", schema.title.as_deref().unwrap_or("(database)"));
                    for p in &schema.properties {
                        println!("- {} ({})", p.name, p.kind);
                    }
                });
            } else {
                let page = client.page_markdown(&id, block.as_deref()).await?;
                if let Some(path) = &output {
                    std::fs::write(path, &page.markdown)
                        .with_context(|| format!("writing {}", path.display()))?;
                }
                emit(&app, &page, |_ui| {
                    if output.is_none() {
                        println!("{}", page.markdown);
                    } else {
                        println!(
                            "Saved {} ({}).",
                            page.title,
                            output
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default()
                        );
                    }
                    if page.truncated {
                        eprintln!(
                            "note: page truncated; read the rest with --block <id>: {}",
                            page.unknown_block_ids.join(", ")
                        );
                    }
                });
            }
        }
        NotesCommand::Query {
            database,
            sql,
            filter,
            page_size,
            cursor,
        } => {
            let mut request = QueryRequest {
                sql,
                page_size,
                cursor,
                ..QueryRequest::default()
            };
            if let Some(file) = filter {
                let (filter, sorts) = read_filter(&file)?;
                request.filter = Some(filter);
                request.sorts = sorts;
            }
            let result = client.query_database(&resource_id(&database), &request).await?;
            emit(&app, &result, |_ui| {
                for row in &result.rows {
                    println!("{}  {}", row.id, Value::Object(row.values.clone()));
                }
                if let Some(next) = &result.next_cursor {
                    eprintln!("more rows: --cursor {next}");
                }
            });
        }
        NotesCommand::Comments { page, all } => {
            let threads: Vec<_> = client
                .comments(&resource_id(&page))
                .await?
                .into_iter()
                .filter(|t| all || t.resolved_at.is_none())
                .collect();
            emit(&app, &threads, |_ui| {
                if threads.is_empty() {
                    println!("No open comments.");
                }
                for t in &threads {
                    let quote = t
                        .quote
                        .as_ref()
                        .map(|q| format!(" on \"{}\"", q.exact))
                        .unwrap_or_default();
                    println!("Thread {}{quote}", t.id);
                    for c in &t.comments {
                        println!("  {}: {}", c.author.name, c.text().replace('\n', " "));
                    }
                }
            });
        }
        NotesCommand::Create(args) => create(&app, &client, args).await?,
        NotesCommand::Edit(args) => edit(&app, &client, args).await?,
        NotesCommand::Comment(args) => comment(&app, &client, args).await?,
        NotesCommand::Rows(command) => rows(&app, &client, command).await?,
        NotesCommand::Suggestions { page, all } => {
            let list: Vec<_> = client
                .suggestions(&resource_id(&page))
                .await?
                .into_iter()
                .filter(|s| all || s.status == "pending")
                .collect();
            emit(&app, &list, |_ui| {
                if list.is_empty() {
                    println!("No pending suggestions.");
                }
                for s in &list {
                    let who = s.author.as_ref().map(|a| a.name.as_str()).unwrap_or("?");
                    let via = s
                        .via_agent
                        .as_deref()
                        .map(|a| format!(" (via {a})"))
                        .unwrap_or_default();
                    println!(
                        "{}  {}  {who}{via}  {} change(s)",
                        s.id,
                        s.status,
                        s.changes.len()
                    );
                    for c in &s.changes {
                        println!("    [{}] {}", c.id.as_deref().unwrap_or("-"), c.kind);
                        if let Some(before) = &c.before_nfm {
                            println!("      - {}", before.replace('\n', "\n      - "));
                        }
                        if let Some(after) = &c.after_nfm {
                            println!("      + {}", after.replace('\n', "\n      + "));
                        }
                    }
                }
            });
        }
        NotesCommand::Resolve {
            suggestion,
            accept,
            reject: _,
            changes,
        } => {
            let result = if accept {
                let ids = (!changes.is_empty()).then_some(changes.as_slice());
                client.accept_suggestion(&suggestion, ids).await?
            } else {
                client.reject_suggestion(&suggestion).await?
            };
            emit(&app, &result, |_ui| {
                println!(
                    "Suggestion {suggestion}: {}",
                    result
                        .status
                        .as_deref()
                        .unwrap_or(if accept { "accepted" } else { "rejected" })
                );
                if !result.stale_change_ids.is_empty() {
                    println!("Stale (anchor text gone): {}", result.stale_change_ids.join(", "));
                }
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn create(app: &App, client: &Client, args: NotesCreateArgs) -> Result<()> {
    let markdown = read_input(&args.file)?;
    let title = args
        .title
        .clone()
        .unwrap_or_else(|| title_from(&markdown, &args.file));
    let parent = match (&args.parent, &args.database, &args.workspace) {
        (Some(p), _, _) => Parent::page(resource_id(p)),
        (_, Some(d), _) => Parent::database(resource_id(d)),
        (_, _, Some(w)) => Parent {
            workspace_id: Some(w.clone()),
            ..Parent::default()
        },
        _ => bail!("give --parent, --database or --workspace"),
    };
    let page = NewPage {
        title,
        icon: args.icon,
        markdown: Some(markdown),
        properties: None,
    };
    let created = client.create_page(&parent, &page).await?;
    emit(app, &created, |_ui| {
        println!(
            "Created {} ({})",
            created.title.as_deref().unwrap_or(&page.title),
            created.url.as_deref().unwrap_or(&created.id)
        )
    });
    Ok(())
}

/// First `# ` heading, else the file stem.
pub fn title_from(markdown: &str, file: &str) -> String {
    markdown
        .lines()
        .find_map(|l| l.strip_prefix("# ").map(|t| t.trim().to_string()))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| {
            std::path::Path::new(file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string()
        })
}

/// Pairs `--old` with `--new`.
pub fn edits(old: &[String], new: &[String], replace_all: bool) -> Result<Vec<ContentEdit>> {
    if old.len() != new.len() {
        bail!(
            "give one --new for every --old ({} --old, {} --new)",
            old.len(),
            new.len()
        );
    }
    Ok(old
        .iter()
        .zip(new)
        .map(|(o, n)| ContentEdit {
            old_str: o.clone(),
            new_str: n.clone(),
            replace_all,
        })
        .collect())
}

async fn edit(app: &App, client: &Client, args: NotesEditArgs) -> Result<()> {
    let mut update = UpdatePage {
        allow_deleting_content: args.allow_deleting_content,
        mode: args.suggest.then_some(WriteMode::Suggest),
        agent: Some("ttm-cli".into()),
        ..UpdatePage::default()
    };
    if args.replace {
        update.replace_content = Some(read_input(args.file.as_deref().unwrap_or("-"))?);
    } else {
        if args.old.is_empty() {
            bail!("nothing to change: give --old/--new pairs or --replace -f file.md");
        }
        update.update_content = Some(edits(&args.old, &args.new, args.replace_all)?);
    }
    let result = client.update_page(&resource_id(&args.page), &update).await?;
    emit(app, &result, |_ui| {
        match (&result.suggestion_id, result.applied) {
            (Some(id), _) => println!("Suggestion {id} created."),
            (None, true) => println!("Page updated."),
            (None, false) => println!("No change applied."),
        }
        for w in &result.warnings {
            eprintln!("warning: {w}");
        }
    });
    Ok(())
}

async fn comment(app: &App, client: &Client, args: NotesCommentArgs) -> Result<()> {
    if let Some(thread) = &args.thread {
        let reply = client.reply_comment(thread, &args.message).await?;
        emit(app, &reply, |_ui| println!("Replied in thread {thread}."));
        return Ok(());
    }
    let anchor = match args.quote {
        Some(exact) => NewAnchor::Quote {
            quote: TextQuote {
                exact,
                prefix: args.prefix,
                suffix: args.suffix,
            },
        },
        None => NewAnchor::Page,
    };
    let thread = client
        .create_comment(&resource_id(&args.page), &anchor, &args.message)
        .await?;
    emit(app, &thread, |_ui| {
        println!("Comment thread {} created.", thread.id)
    });
    Ok(())
}

async fn rows(app: &App, client: &Client, command: RowsCommand) -> Result<()> {
    match command {
        RowsCommand::Upsert { database, file } => {
            let rows: Vec<RowUpsert> = serde_json::from_str(&read_input(&file)?)
                .context("rows file must be [{\"id\"?: …, \"values\": {…}}]")?;
            if rows.is_empty() || rows.len() > 100 {
                bail!("give 1 to 100 rows per call");
            }
            let result = client.upsert_rows(&resource_id(&database), &rows).await?;
            emit(app, &result, |_ui| {
                println!("{} row(s) saved: {}", result.ids.len(), result.ids.join(", "))
            });
        }
        RowsCommand::Delete { database, ids } => {
            let result = client.delete_rows(&resource_id(&database), &ids).await?;
            emit(app, &result, |_ui| println!("{} row(s) deleted.", ids.len()));
        }
    }
    Ok(())
}

/// A file holding `{"filter": …, "sorts": …}` or the bare filter; `-` reads stdin.
pub fn read_filter(file: &str) -> Result<(Value, Option<Vec<Value>>)> {
    let text = read_input(file)?;
    let value: Value = serde_json::from_str(&text).context("the filter file is not valid JSON")?;
    Ok(split_filter(value))
}

pub fn split_filter(value: Value) -> (Value, Option<Vec<Value>>) {
    match value {
        Value::Object(mut map) if map.contains_key("filter") => {
            let sorts = map.remove("sorts").and_then(|s| s.as_array().cloned());
            (map.remove("filter").unwrap_or(json!({})), sorts)
        }
        other => (other, None),
    }
}

/// Reads a file, or stdin for `-`.
pub fn read_input(file: &str) -> Result<String> {
    if file == "-" {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        Ok(buf)
    } else {
        std::fs::read_to_string(file).with_context(|| format!("reading {file}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_and_edits() {
        assert_eq!(title_from("intro\n# Kế hoạch\n", "x.md"), "Kế hoạch");
        assert_eq!(title_from("no heading", "dir/plan-q4.md"), "plan-q4");
        assert_eq!(
            edits(&["a".into()], &["b".into()], false).unwrap()[0].new_str,
            "b"
        );
        assert!(edits(&["a".into()], &[], false).is_err());
    }

    #[test]
    fn filter_file_forms() {
        let (f, s) = split_filter(
            json!({"filter": {"property": "Status", "op": "eq", "value": "Done"}, "sorts": [{"property": "Due"}]}),
        );
        assert_eq!(f["op"], "eq");
        assert_eq!(s.unwrap().len(), 1);
        let (f, s) = split_filter(json!({"and": []}));
        assert!(f.get("and").is_some() && s.is_none());
    }
}
