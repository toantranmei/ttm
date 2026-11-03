//! Notee tools. Names and arguments follow Notion's MCP so agents feel at home.
//!
//! Read: `notee-search`, `notee-fetch`, `notee-query-database`, `notee-get-comments`.

use crate::Backend;
use crate::backend::Fetched;
use crate::common::{ToolFailure, ToolOutput, ToolResult, parse_required, read_only, to_value};
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::fmt::Write as _;
use ttm_api::notee::{QueryRequest, resource_id};

pub const SEARCH: &str = "notee-search";
pub const FETCH: &str = "notee-fetch";
pub const QUERY_DATABASE: &str = "notee-query-database";
pub const GET_COMMENTS: &str = "notee-get-comments";

/// URI of the Markdown specification resource.
pub const SPEC_URI: &str = "notee://spec/markdown";
/// `docs/en/notee-markdown.md`, embedded at build time.
pub const SPEC_MARKDOWN: &str = include_str!("../spec/notee-markdown.md");

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchArgs {
    /// Words to find in titles and bodies (accent-insensitive: "ke hoach" finds "Kế hoạch").
    pub query: String,
    /// Maximum hits (1-20, default 10).
    #[serde(default)]
    pub limit: Option<usize>,
    /// Limit to one workspace (default: the current workspace).
    #[serde(default)]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FetchArgs {
    /// Page or database id, or any Notee URL of it.
    pub id: String,
    /// Id from `unknown_block_ids` of a truncated page, to read that part.
    #[serde(default)]
    pub block: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QueryArgs {
    /// Database id or URL.
    pub database: String,
    /// Read-only SQL over the table `rows`, columns named after properties:
    /// `SELECT "Name", "Status" FROM rows WHERE "Status" = 'Done' ORDER BY "Due" LIMIT 20`.
    #[serde(default)]
    pub sql: Option<String>,
    /// JSON filter instead of SQL: `{"and": [{"property": "Status", "op": "eq", "value": "Done"}]}`.
    /// Ops: eq, neq, contains, starts_with, gt, gte, lt, lte, is_empty, is_not_empty, in.
    #[serde(default)]
    pub filter: Option<Value>,
    /// With `filter`: `[{"property": "Due", "direction": "asc"}]`.
    #[serde(default)]
    pub sorts: Option<Vec<Value>>,
    /// 1-100 rows per page.
    #[serde(default)]
    pub page_size: Option<u32>,
    /// `next_cursor` of the previous page.
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommentsArgs {
    /// Page id or URL.
    pub page: String,
    /// Include resolved threads (default false).
    #[serde(default)]
    pub include_resolved: bool,
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool::new(
            SEARCH,
            "Search Notee pages the user can see by title and content. Returns ids, titles and a snippet; use notee-fetch to read one.",
            schema_for_type::<SearchArgs>(),
        )
        .annotate(read_only("Search Notee")),
        Tool::new(
            FETCH,
            "Read a Notee page as Notee-flavored Markdown (Notion-style tags; spec at notee://spec/markdown), or a database's \
             properties and views. Accepts an id or URL. Large pages come back with truncated=true and unknown_block_ids; \
             pass one as `block` to read the rest.",
            schema_for_type::<FetchArgs>(),
        )
        .annotate(read_only("Fetch page or database")),
        Tool::new(
            QUERY_DATABASE,
            "Query the rows of a Notee database with read-only SQL (table `rows`, columns = property names) or a JSON filter. \
             Fetch the database first to learn its properties.",
            schema_for_type::<QueryArgs>(),
        )
        .annotate(read_only("Query database")),
        Tool::new(
            GET_COMMENTS,
            "Comment threads of a Notee page, with the quoted text each thread is anchored to.",
            schema_for_type::<CommentsArgs>(),
        )
        .annotate(read_only("Read comments")),
    ]
}

pub async fn call<B: Backend + ?Sized>(
    backend: &B,
    name: &str,
    args: Option<Map<String, Value>>,
) -> Option<ToolResult> {
    Some(match name {
        SEARCH => search(backend, args).await,
        FETCH => fetch(backend, args).await,
        QUERY_DATABASE => query(backend, args).await,
        GET_COMMENTS => comments(backend, args).await,
        _ => return None,
    })
}

async fn search<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: SearchArgs = parse_required(args)?;
    if args.query.trim().is_empty() {
        return Err(ToolFailure::InvalidArgs("query is empty".into()));
    }
    let limit = args.limit.unwrap_or(10).clamp(1, 20);
    let mut hits = backend
        .notee_search(&args.query, args.workspace_id.as_deref())
        .await?;
    hits.truncate(limit);
    let mut text = String::new();
    if hits.is_empty() {
        text.push_str("No pages found.");
    }
    for hit in &hits {
        let _ = write!(text, "- {} (id: {})", hit.title, hit.id);
        if !hit.snippet.is_empty() {
            let _ = write!(text, ": {}", hit.snippet.replace('\n', " "));
        }
        text.push('\n');
    }
    Ok(ToolOutput::with_text(json!({ "results": hits }), text))
}

async fn fetch<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: FetchArgs = parse_required(args)?;
    let id = resource_id(&args.id);
    let fetched = backend.notee_fetch(&id, args.block.as_deref()).await?;
    let text = match &fetched {
        Fetched::Page(page) => render_page(page),
        Fetched::Database(db) => {
            let mut text = format!(
                "# {} (database {})\n\n| Property | Type |\n| --- | --- |\n",
                db.title.as_deref().unwrap_or(""),
                id
            );
            for p in &db.properties {
                let _ = writeln!(text, "| {} | {} |", p.name, p.kind);
            }
            if !db.views.is_empty() {
                let _ = write!(text, "\nViews: {}", db.views.len());
            }
            text
        }
    };
    Ok(ToolOutput::with_text(to_value(&fetched), text))
}

/// Markdown handed to the model for a page.
pub fn render_page(page: &ttm_api::notee::PageMarkdown) -> String {
    let mut text = String::new();
    let _ = writeln!(
        text,
        "<page id=\"{}\" title=\"{}\"{}>",
        page.id,
        page.title.replace('"', "'"),
        page.url
            .as_deref()
            .map(|u| format!(" url=\"{u}\""))
            .unwrap_or_default()
    );
    if let Some(props) = &page.properties {
        let _ = writeln!(text, "<properties>{props}</properties>");
    }
    text.push_str(&page.markdown);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str("</page>");
    if page.truncated {
        let _ = write!(
            text,
            "\n\nThe page was truncated. Read the missing parts with notee-fetch and `block` set to one of: {}",
            page.unknown_block_ids.join(", ")
        );
    }
    text
}

async fn query<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: QueryArgs = parse_required(args)?;
    if args.sql.is_some() == args.filter.is_some() {
        return Err(ToolFailure::InvalidArgs(
            "give exactly one of `sql` or `filter`".into(),
        ));
    }
    let request = QueryRequest {
        sql: args.sql,
        filter: args.filter,
        sorts: args.sorts,
        page_size: args.page_size.map(|n| n.clamp(1, 100)),
        cursor: args.cursor,
    };
    let result = backend
        .notee_query_database(&resource_id(&args.database), &request)
        .await?;
    Ok(ToolOutput::json(to_value(&result)))
}

async fn comments<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: CommentsArgs = parse_required(args)?;
    let threads: Vec<_> = backend
        .notee_get_comments(&resource_id(&args.page))
        .await?
        .into_iter()
        .filter(|t| args.include_resolved || t.resolved_at.is_none())
        .collect();
    let mut text = String::new();
    if threads.is_empty() {
        text.push_str("No open comments.");
    }
    for thread in &threads {
        let quote = thread.quote.as_ref().map(|q| q.exact.clone()).or_else(|| {
            thread
                .anchor
                .get("quote")
                .and_then(Value::as_str)
                .map(String::from)
        });
        let _ = writeln!(
            text,
            "Thread {}{}:",
            thread.id,
            quote.map(|q| format!(" on \"{q}\"")).unwrap_or_default()
        );
        for c in &thread.comments {
            let _ = writeln!(text, "  - {}: {}", c.author.name, c.text().replace('\n', " "));
        }
    }
    let simplified: Vec<Value> = threads
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "anchor": t.anchor,
                "quote": t.quote,
                "resolved": t.resolved_at.is_some(),
                "comments": t.comments.iter().map(|c| json!({
                    "id": c.id, "author": c.author.name, "text": c.text(), "created_at": c.created_at
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    Ok(ToolOutput::with_text(json!({ "threads": simplified }), text))
}
