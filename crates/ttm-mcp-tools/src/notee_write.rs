//! Notee write tools: `notee-create-pages`, `notee-update-page`, `notee-move-pages`, `notee-duplicate-page`,
//! `notee-create-database`, `notee-update-database`, `notee-upsert-rows`, `notee-delete-rows`, `notee-create-comment`.

use crate::Backend;
use crate::common::{CallContext, ToolFailure, ToolOutput, ToolResult, parse_required, to_value, writes};
use rmcp::handler::server::common::schema_for_type;
use rmcp::model::Tool;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use ttm_api::notee::{
    ContentEdit, CreateDatabase, CreatePages, NewAnchor, NewPage, Parent, RowUpsert, TextQuote, UpdatePage,
    WriteMode, resource_id,
};

pub const CREATE_PAGES: &str = "notee-create-pages";
pub const UPDATE_PAGE: &str = "notee-update-page";
pub const MOVE_PAGES: &str = "notee-move-pages";
pub const DUPLICATE_PAGE: &str = "notee-duplicate-page";
pub const CREATE_DATABASE: &str = "notee-create-database";
pub const UPDATE_DATABASE: &str = "notee-update-database";
pub const UPSERT_ROWS: &str = "notee-upsert-rows";
pub const DELETE_ROWS: &str = "notee-delete-rows";
pub const CREATE_COMMENT: &str = "notee-create-comment";

/// Most pages per `notee-create-pages`.
pub const MAX_PAGES: usize = 20;
/// Most rows per `notee-upsert-rows`.
pub const MAX_ROWS: usize = 100;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParentArg {
    /// Parent page id or URL.
    #[serde(default)]
    pub page_id: Option<String>,
    /// Database id or URL: the new pages become rows.
    #[serde(default)]
    pub database_id: Option<String>,
    /// Top level of a workspace.
    #[serde(default)]
    pub workspace_id: Option<String>,
}

impl ParentArg {
    fn into_parent(self) -> Result<Parent, ToolFailure> {
        let parent = Parent {
            page_id: self.page_id.map(|v| resource_id(&v)),
            database_id: self.database_id.map(|v| resource_id(&v)),
            workspace_id: self.workspace_id,
        };
        if parent.is_valid() {
            Ok(parent)
        } else {
            Err(ToolFailure::InvalidArgs(
                "parent needs exactly one of page_id, database_id, workspace_id".into(),
            ))
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewPageArg {
    pub title: String,
    /// Emoji.
    #[serde(default)]
    pub icon: Option<String>,
    /// Body in Notee-flavored Markdown (see notee://spec/markdown).
    #[serde(default)]
    pub content: Option<String>,
    /// Row values when the parent is a database, by property name.
    #[serde(default)]
    pub properties: Option<Map<String, Value>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatePagesArgs {
    pub parent: ParentArg,
    /// 1-20 pages; at most 1,000 blocks in total.
    pub pages: Vec<NewPageArg>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditArg {
    /// Exact text of the current Markdown (case-sensitive); must match once unless replace_all.
    pub old_str: String,
    pub new_str: String,
    #[serde(default)]
    pub replace_all: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdatePageArgs {
    /// Page id or URL.
    pub page: String,
    /// Find/replace edits on the page Markdown (preferred: keeps other people's cursors).
    #[serde(default)]
    pub update_content: Option<Vec<EditArg>>,
    /// The whole new Markdown body. Keep every <unknown/> tag you read.
    #[serde(default)]
    pub replace_content: Option<String>,
    /// Required to remove child pages or databases.
    #[serde(default)]
    pub allow_deleting_content: bool,
    #[serde(default)]
    pub title: Option<String>,
    /// Row properties to change (database rows).
    #[serde(default)]
    pub properties: Option<Map<String, Value>>,
    /// `apply` (default) writes now; `suggest` proposes the change for people to accept or reject.
    #[serde(default)]
    pub mode: Option<ModeArg>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ModeArg {
    Apply,
    Suggest,
}

impl From<ModeArg> for WriteMode {
    fn from(mode: ModeArg) -> Self {
        match mode {
            ModeArg::Apply => WriteMode::Apply,
            ModeArg::Suggest => WriteMode::Suggest,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MovePagesArgs {
    /// Page ids or URLs.
    pub pages: Vec<String>,
    pub parent: ParentArg,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PageArg {
    /// Page id or URL.
    pub page: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateDatabaseArgs {
    pub parent: ParentArg,
    pub title: String,
    /// `[{"name": "Status", "type": "select", "options": [{"name": "Todo"}, {"name": "Done"}]}]`.
    /// Types: title, text, number, select, multi_select, status, date, person, checkbox, url, email, phone, relation.
    #[serde(default)]
    pub properties: Vec<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateDatabaseArgs {
    /// Database id or URL.
    pub database: String,
    /// `{"add_properties": [...], "update_properties": [...], "remove_properties": ["Name"], "views": [...], "title": "…"}`.
    pub changes: Map<String, Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RowArg {
    /// Existing row id to update; omit to create a row.
    #[serde(default)]
    pub id: Option<String>,
    /// Values by property name.
    pub values: Map<String, Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UpsertRowsArgs {
    pub database: String,
    /// 1-100 rows.
    pub rows: Vec<RowArg>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteRowsArgs {
    pub database: String,
    pub ids: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuoteArg {
    /// The exact text to anchor to, as it appears in the page.
    pub exact: String,
    /// A few words right before it, when `exact` appears more than once.
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateCommentArgs {
    /// Page id or URL.
    pub page: String,
    /// Comment text.
    pub text: String,
    /// Anchor to a quote of the page; omit for a page-level comment.
    #[serde(default)]
    pub quote: Option<QuoteArg>,
    /// Reply in this thread instead of starting one.
    #[serde(default)]
    pub thread_id: Option<String>,
}

pub fn tools() -> Vec<Tool> {
    vec![
        Tool::new(
            CREATE_PAGES,
            "Create up to 20 Notee pages under a page, as rows of a database, or at the top of a workspace. Content is \
             Notee-flavored Markdown (notee://spec/markdown); at most 1,000 blocks per call.",
            schema_for_type::<CreatePagesArgs>(),
        )
        .annotate(writes("Create pages", false)),
        Tool::new(
            UPDATE_PAGE,
            "Edit a Notee page: `update_content` find/replace edits (each old_str must match exactly once) or \
             `replace_content` for the whole body; also title and row properties. Read the page with notee-fetch first. \
             mode=suggest proposes the change instead of applying it.",
            schema_for_type::<UpdatePageArgs>(),
        )
        .annotate(writes("Update page", true)),
        Tool::new(MOVE_PAGES, "Move Notee pages under another page, database or workspace.", schema_for_type::<MovePagesArgs>())
            .annotate(writes("Move pages", false)),
        Tool::new(DUPLICATE_PAGE, "Duplicate a Notee page with its content.", schema_for_type::<PageArg>())
            .annotate(writes("Duplicate page", false)),
        Tool::new(CREATE_DATABASE, "Create a Notee database with properties.", schema_for_type::<CreateDatabaseArgs>())
            .annotate(writes("Create database", false)),
        Tool::new(
            UPDATE_DATABASE,
            "Change a Notee database: add, update or remove properties and views, rename it.",
            schema_for_type::<UpdateDatabaseArgs>(),
        )
        .annotate(writes("Update database", true)),
        Tool::new(
            UPSERT_ROWS,
            "Create or update up to 100 rows of a Notee database (values by property name).",
            schema_for_type::<UpsertRowsArgs>(),
        )
        .annotate(writes("Upsert rows", false)),
        Tool::new(DELETE_ROWS, "Delete rows of a Notee database (moved to trash).", schema_for_type::<DeleteRowsArgs>())
            .annotate(writes("Delete rows", true)),
        Tool::new(
            CREATE_COMMENT,
            "Comment on a Notee page, anchored to a quote of its text ({exact, prefix?, suffix?}) or on the whole page; or \
             reply in a thread.",
            schema_for_type::<CreateCommentArgs>(),
        )
        .annotate(writes("Comment", false)),
    ]
}

pub async fn call<B: Backend + ?Sized>(
    backend: &B,
    name: &str,
    args: Option<Map<String, Value>>,
    ctx: &CallContext,
) -> Option<ToolResult> {
    Some(match name {
        CREATE_PAGES => create_pages(backend, args).await,
        UPDATE_PAGE => update_page(backend, args, ctx).await,
        MOVE_PAGES => move_pages(backend, args).await,
        DUPLICATE_PAGE => duplicate(backend, args).await,
        CREATE_DATABASE => create_database(backend, args).await,
        UPDATE_DATABASE => update_database(backend, args).await,
        UPSERT_ROWS => upsert_rows(backend, args).await,
        DELETE_ROWS => delete_rows(backend, args).await,
        CREATE_COMMENT => create_comment(backend, args).await,
        _ => return None,
    })
}

async fn create_pages<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: CreatePagesArgs = parse_required(args)?;
    if args.pages.is_empty() || args.pages.len() > MAX_PAGES {
        return Err(ToolFailure::InvalidArgs(format!("give 1 to {MAX_PAGES} pages")));
    }
    let request = CreatePages {
        parent: args.parent.into_parent()?,
        pages: args
            .pages
            .into_iter()
            .map(|p| NewPage {
                title: p.title,
                icon: p.icon,
                markdown: p.content,
                properties: p.properties.map(Value::Object),
            })
            .collect(),
    };
    let created = backend.notee_create_pages(&request).await?;
    Ok(ToolOutput::json(json!({ "pages": created })))
}

/// Builds the PATCH body; at least one change is required.
pub fn build_update(
    args: UpdatePageArgs,
    agent: Option<String>,
) -> Result<(String, UpdatePage), ToolFailure> {
    if args.update_content.is_some() && args.replace_content.is_some() {
        return Err(ToolFailure::InvalidArgs(
            "use update_content or replace_content, not both".into(),
        ));
    }
    let update = UpdatePage {
        update_content: args.update_content.map(|edits| {
            edits
                .into_iter()
                .map(|e| ContentEdit {
                    old_str: e.old_str,
                    new_str: e.new_str,
                    replace_all: e.replace_all,
                })
                .collect()
        }),
        replace_content: args.replace_content,
        allow_deleting_content: args.allow_deleting_content,
        title: args.title,
        properties: args.properties.map(Value::Object),
        mode: args.mode.map(Into::into),
        agent,
    };
    if update.update_content.as_ref().is_none_or(Vec::is_empty)
        && update.replace_content.is_none()
        && update.title.is_none()
        && update.properties.is_none()
    {
        return Err(ToolFailure::InvalidArgs("nothing to change".into()));
    }
    Ok((resource_id(&args.page), update))
}

async fn update_page<B: Backend + ?Sized>(
    backend: &B,
    args: Option<Map<String, Value>>,
    ctx: &CallContext,
) -> ToolResult {
    let (page, update) = build_update(parse_required(args)?, ctx.agent.clone())?;
    let result = backend.notee_update_page(&page, &update).await?;
    let mut text = match (&result.suggestion_id, result.applied) {
        (Some(id), _) => format!("Suggestion {id} created; people with edit access can accept or reject it."),
        (None, true) => "Page updated.".to_string(),
        (None, false) => "No change applied.".to_string(),
    };
    for w in &result.warnings {
        text.push_str(&format!("\nWarning: {w}"));
    }
    Ok(ToolOutput::with_text(to_value(&result), text))
}

async fn move_pages<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: MovePagesArgs = parse_required(args)?;
    let parent = args.parent.into_parent()?;
    let mut moved = Vec::new();
    for page in &args.pages {
        moved.push(backend.notee_move_page(&resource_id(page), &parent).await?);
    }
    Ok(ToolOutput::json(json!({ "pages": moved })))
}

async fn duplicate<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: PageArg = parse_required(args)?;
    Ok(ToolOutput::json(to_value(
        &backend.notee_duplicate_page(&resource_id(&args.page)).await?,
    )))
}

async fn create_database<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: CreateDatabaseArgs = parse_required(args)?;
    let request = CreateDatabase {
        parent: args.parent.into_parent()?,
        title: args.title,
        properties: args.properties,
    };
    Ok(ToolOutput::json(to_value(
        &backend.notee_create_database(&request).await?,
    )))
}

async fn update_database<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: UpdateDatabaseArgs = parse_required(args)?;
    let result = backend
        .notee_update_database(&resource_id(&args.database), &Value::Object(args.changes))
        .await?;
    Ok(ToolOutput::json(result))
}

async fn upsert_rows<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: UpsertRowsArgs = parse_required(args)?;
    if args.rows.is_empty() || args.rows.len() > MAX_ROWS {
        return Err(ToolFailure::InvalidArgs(format!("give 1 to {MAX_ROWS} rows")));
    }
    let rows: Vec<RowUpsert> = args
        .rows
        .into_iter()
        .map(|r| RowUpsert {
            id: r.id,
            values: r.values,
        })
        .collect();
    Ok(ToolOutput::json(to_value(
        &backend
            .notee_upsert_rows(&resource_id(&args.database), &rows)
            .await?,
    )))
}

async fn delete_rows<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: DeleteRowsArgs = parse_required(args)?;
    if args.ids.is_empty() {
        return Err(ToolFailure::InvalidArgs("ids is empty".into()));
    }
    Ok(ToolOutput::json(
        backend
            .notee_delete_rows(&resource_id(&args.database), &args.ids)
            .await?,
    ))
}

async fn create_comment<B: Backend + ?Sized>(backend: &B, args: Option<Map<String, Value>>) -> ToolResult {
    let args: CreateCommentArgs = parse_required(args)?;
    if args.text.trim().is_empty() {
        return Err(ToolFailure::InvalidArgs("text is empty".into()));
    }
    if let Some(thread) = &args.thread_id {
        return Ok(ToolOutput::json(
            backend.notee_reply_comment(thread, &args.text).await?,
        ));
    }
    let anchor = match args.quote {
        Some(q) => NewAnchor::Quote {
            quote: TextQuote {
                exact: q.exact,
                prefix: q.prefix,
                suffix: q.suffix,
            },
        },
        None => NewAnchor::Page,
    };
    Ok(ToolOutput::json(to_value(
        &backend
            .notee_create_comment(&resource_id(&args.page), &anchor, &args.text)
            .await?,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update_args(v: Value) -> UpdatePageArgs {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn update_needs_a_change_and_not_both_modes() {
        assert!(build_update(update_args(json!({"page": "p"})), None).is_err());
        assert!(
            build_update(
                update_args(json!({"page": "p", "update_content": [], "replace_content": "x"})),
                None
            )
            .is_err()
        );
        let (page, body) = build_update(
            update_args(
                json!({"page": "p", "update_content": [{"old_str": "a", "new_str": "b"}], "mode": "suggest"}),
            ),
            Some("cursor".into()),
        )
        .unwrap();
        assert_eq!(page, "p");
        assert_eq!(body.mode, Some(WriteMode::Suggest));
        assert_eq!(body.agent.as_deref(), Some("cursor"));
    }

    #[test]
    fn parent_must_be_single() {
        let p: ParentArg = serde_json::from_value(json!({"page_id": "a", "database_id": "b"})).unwrap();
        assert!(p.into_parent().is_err());
        let p: ParentArg = serde_json::from_value(
            json!({"database_id": "https://x.com/db/aaaaaaaa-1b2c-4d5e-8f90-123456789abc"}),
        )
        .unwrap();
        assert_eq!(
            p.into_parent().unwrap().database_id.as_deref(),
            Some("aaaaaaaa-1b2c-4d5e-8f90-123456789abc")
        );
    }
}
