//! `ttm notes …`: Notee from the terminal. Output is Markdown for people, JSON with `--json`.

use crate::app::App;
use crate::cli::{NotesCommand, NotesCommentArgs, NotesCreateArgs, NotesEditArgs, RowsCommand};
use crate::output::emit;
use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::notee::{
    ContentEdit, NewAnchor, NewPage, Parent, QueryRequest, QueryResult, RowUpsert, TextQuote, UpdatePage,
    WriteMode, resource_id,
};
use ttm_ui::vocab::Phase;
use ttm_ui::{Badge, Callout, Cell, Col, Diff, Status, Table, Token, next_line, shell_quote, truncate};

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
            emit(&app, &hits, |ui| {
                if hits.is_empty() {
                    ui.print(&Callout::info(format!("No pages match \"{query}\"")));
                    return;
                }
                ui.line(&format!("{} page(s) match \"{query}\"", hits.len()));
                let mut table = Table::new(&[
                    Col::new("TITLE").flex(2).min(10),
                    Col::new("SNIPPET").flex(3).min(10),
                    Col::new("ID"),
                ]);
                for h in &hits {
                    let title = match &h.icon {
                        Some(icon) if !ui.mode().plain => format!("{icon} {}", h.title),
                        _ => h.title.clone(),
                    };
                    table.push(vec![
                        Cell::new(title),
                        Cell::token(h.snippet.replace('\n', " "), Token::Muted),
                        Cell::new(h.id.as_str()),
                    ]);
                }
                ui.print(&table);
                ui.line(&next_line(ui.mode(), &format!("ttm notes get {}", hits[0].id)));
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
                emit(&app, &schema, |ui| {
                    ui.line(&format!(
                        "{} · database · {} propert{}",
                        ui.paint(Token::Ink, schema.title.as_deref().unwrap_or("(database)")),
                        schema.properties.len(),
                        if schema.properties.len() == 1 { "y" } else { "ies" }
                    ));
                    let mut table = Table::new(&[Col::new("PROPERTY").flex(1).min(8), Col::new("TYPE")])
                        .empty_text("No properties.");
                    for p in &schema.properties {
                        table.push(vec![
                            Cell::new(p.name.as_str()),
                            Cell::token(p.kind.as_str(), Token::Muted),
                        ]);
                    }
                    ui.print(&table);
                    ui.line(&next_line(
                        ui.mode(),
                        &format!("ttm notes query {id} --sql 'SELECT * FROM rows'"),
                    ));
                });
            } else {
                let page = client.page_markdown(&id, block.as_deref()).await?;
                if let Some(path) = &output {
                    std::fs::write(path, &page.markdown)
                        .with_context(|| format!("writing {}", path.display()))?;
                }
                emit(&app, &page, |ui| {
                    match &output {
                        // The Markdown is printed whole and untouched, so it can be piped; the meta line goes to
                        // stderr for a person at a terminal only.
                        None => {
                            if ui.mode().interactive {
                                ui.eline(&ui.paint(
                                    Token::Muted,
                                    &format!(
                                        "{} · {}{}",
                                        page.title,
                                        page.id,
                                        if page.truncated { " · truncated" } else { "" }
                                    ),
                                ));
                            }
                            let text = if page.markdown.ends_with('\n') {
                                page.markdown.clone()
                            } else {
                                format!("{}\n", page.markdown)
                            };
                            ui.raw(&text);
                        }
                        Some(path) => {
                            ui.print(&Callout::success(format!("Saved {}", page.title)).detail(format!(
                                "{} · {}",
                                path.display(),
                                page.id
                            )))
                        }
                    }
                    if page.truncated {
                        let first = page.unknown_block_ids.first().cloned().unwrap_or_default();
                        ui.eprint(
                            &Callout::warning("The page is truncated")
                                .detail(format!("unread parts: {}", page.unknown_block_ids.join(", ")))
                                .next(format!("ttm notes get {id} --block {first}")),
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
            let database = resource_id(&database);
            let paging_from = request.cursor.is_some();
            let result = client.query_database(&database, &request).await?;
            emit(&app, &result, |ui| {
                ui.print(&query_table(&result));
                let mut summary = format!("{} row(s)", result.rows.len());
                if paging_from {
                    summary.push_str(" · next page");
                }
                if result.next_cursor.is_some() {
                    summary.push_str(" · more rows");
                }
                ui.line(&ui.paint(Token::Muted, &summary));
                if let Some(next) = &result.next_cursor {
                    ui.line(&next_line(
                        ui.mode(),
                        &format!(
                            "ttm notes query {database} {} --cursor {next}",
                            query_flags(&request)
                        ),
                    ));
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
            emit(&app, &threads, |ui| {
                if threads.is_empty() {
                    ui.print(&Callout::info(if all {
                        "No comments"
                    } else {
                        "No open comments"
                    }));
                    return;
                }
                ui.line(&format!("{} thread(s)", threads.len()));
                for t in &threads {
                    ui.blank();
                    let state = if t.resolved_at.is_some() {
                        format!(" {}", ui.render(&Status::phase(Phase::Done)))
                    } else {
                        String::new()
                    };
                    let resolved = if t.resolved_at.is_some() { " resolved" } else { "" };
                    ui.line(&format!(
                        "{}{state}{resolved}",
                        ui.paint(Token::Ink, &format!("Thread {}", t.id))
                    ));
                    if let Some(q) = &t.quote {
                        let quote = truncate(&q.exact.replace('\n', " "), ui.mode().width.saturating_sub(4));
                        ui.line(&format!("  {}", ui.paint(Token::Muted, &format!("> {quote}"))));
                    }
                    for c in &t.comments {
                        ui.line(&format!(
                            "  {} {}: {}",
                            ui.render(&Badge::person()),
                            ui.paint(Token::Ink, &c.author.name),
                            c.text().replace('\n', " ")
                        ));
                    }
                }
                ui.blank();
                ui.line(&next_line(
                    ui.mode(),
                    &format!("ttm notes comment {page} --thread <id> -m <reply>"),
                ));
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
            emit(&app, &list, |ui| {
                if list.is_empty() {
                    ui.print(&Callout::info(if all {
                        "No suggestions"
                    } else {
                        "No pending suggestions"
                    }));
                    return;
                }
                ui.line(&format!("{} suggestion(s)", list.len()));
                let mut table = Table::new(&[
                    Col::new("ID"),
                    Col::new("STATUS"),
                    Col::new("AUTHOR").flex(1).min(8),
                    Col::new("CHANGES").right(),
                ]);
                for s in &list {
                    let who = s.author.as_ref().map(|a| a.name.as_str()).unwrap_or("?");
                    let author = match &s.via_agent {
                        Some(agent) => format!("{} {who} via {agent}", ui.render(&Badge::agent())),
                        None => format!("{} {who}", ui.render(&Badge::person())),
                    };
                    let status = match s.status.as_str() {
                        "pending" => Cell::token("pending", Token::Yellow),
                        "accepted" => Cell::token("accepted", Token::Teal),
                        "rejected" => Cell::token("rejected", Token::Coral),
                        other => Cell::new(other),
                    };
                    table.push(vec![
                        Cell::new(s.id.as_str()),
                        status,
                        Cell::new(author),
                        Cell::new(s.changes.len().to_string()),
                    ]);
                }
                ui.print(&table);
                for s in &list {
                    for c in &s.changes {
                        ui.blank();
                        let label = format!(
                            "suggestion {} · change {} · {}",
                            s.id,
                            c.id.as_deref().unwrap_or("-"),
                            c.kind
                        );
                        ui.print(&Diff::text(
                            &with_newline(c.before_nfm.as_deref().unwrap_or("")),
                            &with_newline(c.after_nfm.as_deref().unwrap_or("")),
                            label,
                        ));
                    }
                }
                ui.blank();
                ui.line(&next_line(
                    ui.mode(),
                    &format!("ttm notes resolve {} --accept", list[0].id),
                ));
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
            emit(&app, &result, |ui| {
                let status = result
                    .status
                    .as_deref()
                    .unwrap_or(if accept { "accepted" } else { "rejected" });
                let mut callout = Callout::success(format!("Suggestion {suggestion} {status}"));
                if !result.stale_change_ids.is_empty() {
                    callout = Callout::warning(format!(
                        "Suggestion {suggestion} {status}, some changes are stale"
                    ))
                    .detail(format!(
                        "anchor text gone: {}",
                        result.stale_change_ids.join(", ")
                    ));
                }
                ui.print(&callout);
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
    emit(app, &created, |ui| {
        let mut callout = Callout::success(format!(
            "Created {}",
            created.title.as_deref().unwrap_or(&page.title)
        ))
        .detail(format!("id {}", created.id));
        if let Some(url) = &created.url {
            callout = callout.detail(ui.paint(Token::Blue, url));
        }
        ui.print(&callout.next(format!("ttm notes get {}", created.id)));
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
    let page = resource_id(&args.page);
    emit(app, &result, |ui| {
        let callout = match (&result.suggestion_id, result.applied) {
            (Some(id), _) => Callout::success(format!("Suggestion {id} created"))
                .next(format!("ttm notes suggestions {page}")),
            (None, true) => Callout::success("Page updated").next(format!("ttm notes get {page}")),
            (None, false) => Callout::warning("No change applied"),
        };
        ui.print(&callout);
        for w in &result.warnings {
            ui.eprint(&Callout::warning(w.as_str()));
        }
    });
    Ok(())
}

async fn comment(app: &App, client: &Client, args: NotesCommentArgs) -> Result<()> {
    if let Some(thread) = &args.thread {
        let reply = client.reply_comment(thread, &args.message).await?;
        emit(app, &reply, |ui| {
            ui.print(&Callout::success(format!("Replied in thread {thread}")))
        });
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
    emit(app, &thread, |ui| {
        ui.print(
            &Callout::success(format!("Comment thread {} created", thread.id))
                .next(format!("ttm notes comments {}", resource_id(&args.page))),
        )
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
            emit(app, &result, |ui| {
                ui.print(&Callout::success(format!("{} row(s) saved", result.ids.len())));
                let mut table = Table::new(&[Col::new("ROW ID")]);
                for id in &result.ids {
                    table.push(vec![Cell::new(id.as_str())]);
                }
                ui.print(&table);
            });
        }
        RowsCommand::Delete { database, ids } => {
            let result = client.delete_rows(&resource_id(&database), &ids).await?;
            emit(app, &result, |ui| {
                ui.print(&Callout::success(format!("{} row(s) deleted", ids.len())))
            });
        }
    }
    Ok(())
}

/// Columns of a query result: the server's `columns` (names or `{name}` objects), else the keys of the rows in order.
pub fn query_columns(result: &QueryResult) -> Vec<String> {
    let named: Vec<String> = result
        .columns
        .iter()
        .filter_map(|c| match c {
            Value::String(s) => Some(s.clone()),
            Value::Object(o) => o.get("name").and_then(Value::as_str).map(String::from),
            _ => None,
        })
        .collect();
    if !named.is_empty() {
        return named;
    }
    let mut keys: Vec<String> = Vec::new();
    for row in &result.rows {
        for k in row.values.keys() {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
    }
    keys
}

/// A cell value: strings as they are, `null` as `-`, the rest as compact JSON.
pub fn cell_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "-".into(),
        Some(Value::String(s)) => s.replace('\n', " "),
        Some(Value::Array(items)) if items.iter().all(Value::is_string) => items
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(", "),
        Some(other) => other.to_string(),
    }
}

fn query_table(result: &QueryResult) -> Table {
    let columns = query_columns(result);
    let mut cols = vec![Col::new("ID")];
    cols.extend(columns.iter().map(|c| Col::new(c.to_uppercase()).flex(1).min(4)));
    let mut table = Table::new(&cols).empty_text("No rows.");
    for row in &result.rows {
        let mut cells = vec![Cell::token(row.id.as_str(), Token::Muted)];
        cells.extend(columns.iter().map(|c| Cell::new(cell_text(row.values.get(c)))));
        table.push(cells);
    }
    table
}

/// The flags of a query request, to repeat it with `--cursor`.
fn query_flags(request: &QueryRequest) -> String {
    let mut flags = Vec::new();
    if let Some(sql) = &request.sql {
        flags.push(format!("--sql {}", shell_quote(sql)));
    } else {
        flags.push("--filter <file>".to_string());
    }
    if let Some(n) = request.page_size {
        flags.push(format!("--page-size {n}"));
    }
    flags.join(" ")
}

fn with_newline(text: &str) -> String {
    if text.is_empty() || text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
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
    fn query_columns_and_cells() {
        let result: QueryResult = serde_json::from_value(json!({
            "columns": [],
            "rows": [{"id": "r1", "values": {"Name": "A", "Tags": ["x", "y"]}}, {"id": "r2", "values": {"Name": null, "Due": 3}}]
        }))
        .unwrap();
        assert_eq!(query_columns(&result), ["Name", "Tags", "Due"]);
        assert_eq!(cell_text(result.rows[0].values.get("Tags")), "x, y");
        assert_eq!(cell_text(result.rows[1].values.get("Name")), "-");
        assert_eq!(cell_text(result.rows[1].values.get("Due")), "3");
        assert_eq!(cell_text(None), "-");
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
