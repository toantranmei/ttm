//! Notee endpoints: search, pages as Notee-flavored Markdown, database schema and queries, comments.

use crate::workspace::encode;
use crate::{Client, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `GET /search` hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    #[serde(default)]
    pub workspace_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub snippet: String,
}

/// `GET /pages/{id}/markdown`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageMarkdown {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub parent: Option<Value>,
    /// Row properties when the page is a database row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
    pub markdown: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub unknown_block_ids: Vec<String>,
}

/// `GET /resources/{id}/summary`: `{id, kind, title, icon?, parent?, url?, level?}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub id: String,
    /// `page`, `database` or `row` (a database row, readable as a page).
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// The caller's access level (`view`, `comment`, `edit`, `full`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub parent: Option<Value>,
    #[serde(default)]
    pub url: Option<String>,
}

/// One property of a database (`GET /databases/{id}/schema`, assumed `{id?, name, type, options?}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertySchema {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Value>,
}

/// `GET /databases/{id}/schema` (assumed `{id, title, properties[], views[]}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseSchema {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub properties: Vec<PropertySchema>,
    #[serde(default)]
    pub views: Vec<Value>,
}

/// `POST /databases/{id}/query` body. Either `sql` or `filter`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct QueryRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sql: Option<String>,
    /// `{and|or: [...]}` or `{property, op, value}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sorts: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRow {
    pub id: String,
    #[serde(default)]
    pub values: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    /// Column descriptions (name, or `{name, type}`), as the server sends them.
    #[serde(default)]
    pub columns: Vec<Value>,
    #[serde(default)]
    pub rows: Vec<QueryRow>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentAuthor {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    /// Mei-er membership (`{tier}`), when the server sends it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership: Option<crate::types::TierRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: CommentAuthor,
    /// Editor JSON (TipTap) of the comment.
    #[serde(default)]
    pub body: Value,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub edited_at: Option<String>,
}

impl Comment {
    /// Plain text of the editor JSON.
    pub fn text(&self) -> String {
        plain_text(&self.body)
    }
}

/// `GET /resources/{id}/comments[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    /// `{kind: page}`, `{kind: block, block_id}`, `{kind: text, block_id, quote}`.
    #[serde(default)]
    pub anchor: Value,
    /// Anchored text with up to 32 characters of context on each side (added by the server for agents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<TextQuote>,
    #[serde(default)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

/// Concatenated text nodes of an editor JSON document, one line per block.
pub fn plain_text(doc: &Value) -> String {
    fn walk(node: &Value, out: &mut String) {
        if let Some(text) = node.get("text").and_then(Value::as_str) {
            out.push_str(text);
        }
        if let Some(children) = node.get("content").and_then(Value::as_array) {
            for child in children {
                walk(child, out);
            }
            let block = node
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|t| t != "doc" && t != "text");
            if block && !out.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    if let Some(s) = doc.as_str() {
        return s.to_string();
    }
    let mut out = String::new();
    walk(doc, &mut out);
    out.trim_end().to_string()
}

/// Editor JSON for a plain-text comment (one paragraph per line).
pub fn text_doc(text: &str) -> Value {
    let paragraphs: Vec<Value> = text
        .lines()
        .map(|line| {
            if line.is_empty() {
                serde_json::json!({"type": "paragraph"})
            } else {
                serde_json::json!({"type": "paragraph", "content": [{"type": "text", "text": line}]})
            }
        })
        .collect();
    serde_json::json!({"type": "doc", "content": paragraphs})
}

/// Accepts an id or any URL of a page/database and returns the id (the last UUID in the path, also as the tail of a
/// `title-<uuid>` slug).
pub fn resource_id(input: &str) -> String {
    let input = input.trim();
    let path = match url::Url::parse(input) {
        Ok(url) if url.scheme().starts_with("http") => url.path().to_string(),
        _ => return input.to_string(),
    };
    path.split('/')
        .rev()
        .filter(|s| !s.is_empty())
        .find_map(|segment| {
            let tail = if segment.len() > 36 {
                &segment[segment.len() - 36..]
            } else {
                segment
            };
            is_uuid(tail).then(|| tail.to_string())
        })
        .unwrap_or_else(|| input.to_string())
}

fn is_uuid(s: &str) -> bool {
    s.len() == 36
        && s.char_indices().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

impl Client {
    /// `GET /search?q=&workspace_id=` (up to 20 hits, title matches first).
    pub async fn search(&self, query: &str, workspace_id: Option<&str>) -> Result<Vec<SearchHit>> {
        let mut params = vec![("q", query.to_string())];
        if let Some(ws) = workspace_id {
            params.push(("workspace_id", ws.to_string()));
        }
        self.get_query("/search", &params).await
    }

    /// `GET /pages/{id}/markdown[?block=]`; `block` reads one part a truncated page left out.
    pub async fn page_markdown(&self, page_id: &str, block: Option<&str>) -> Result<PageMarkdown> {
        let params: Vec<(&str, String)> = block.map(|b| vec![("block", b.to_string())]).unwrap_or_default();
        self.get_query(&format!("/pages/{}/markdown", encode(page_id)), &params)
            .await
    }

    pub async fn resource_summary(&self, id: &str) -> Result<ResourceSummary> {
        self.get(&format!("/resources/{}/summary", encode(id))).await
    }

    pub async fn database_schema(&self, database_id: &str) -> Result<DatabaseSchema> {
        self.get(&format!("/databases/{}/schema", encode(database_id)))
            .await
    }

    pub async fn query_database(&self, database_id: &str, query: &QueryRequest) -> Result<QueryResult> {
        self.post(&format!("/databases/{}/query", encode(database_id)), query)
            .await
    }

    pub async fn comments(&self, resource_id: &str) -> Result<Vec<Thread>> {
        self.get(&format!("/resources/{}/comments", encode(resource_id)))
            .await
    }
}

// Writing (P3).

/// Where a page or database goes: under a page, as a row of a database, or at the top of a workspace.
/// Serialized as `{"page_id": …}`, `{"database_id": …}` or `{"workspace_id": …}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

impl Parent {
    pub fn page(id: impl Into<String>) -> Self {
        Self {
            page_id: Some(id.into()),
            ..Self::default()
        }
    }

    pub fn database(id: impl Into<String>) -> Self {
        Self {
            database_id: Some(id.into()),
            ..Self::default()
        }
    }

    /// Exactly one of the three must be set.
    pub fn is_valid(&self) -> bool {
        [&self.page_id, &self.database_id, &self.workspace_id]
            .iter()
            .filter(|v| v.is_some())
            .count()
            == 1
    }
}

/// One page to create.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewPage {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Notee-flavored Markdown body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Row values when the parent is a database: `{"Status": "Done"}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
}

/// `POST /pages` body.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CreatePage<'a> {
    pub parent: &'a Parent,
    #[serde(flatten)]
    pub page: &'a NewPage,
}

/// `POST /pages/batch` body (≤ 20 pages, ≤ 1,000 blocks in total).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatePages {
    pub parent: Parent,
    pub pages: Vec<NewPage>,
}

/// A created / moved / duplicated page or database (assumed `{id, title?, url?}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceRef {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CreatedPages {
    pages: Vec<ResourceRef>,
}

/// One find/replace of `update_content`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentEdit {
    pub old_str: String,
    pub new_str: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replace_all: bool,
}

/// `apply` writes now; `suggest` stores a suggestion for people to accept.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WriteMode {
    #[default]
    Apply,
    Suggest,
}

/// `PATCH /pages/{id}/markdown` body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdatePage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_content: Option<Vec<ContentEdit>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_content: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_deleting_content: bool,
    /// New title (assumed field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Row properties to change (assumed field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<WriteMode>,
    /// Name of the agent writing for the user ("User (via agent X)" in history).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

/// `PATCH /pages/{id}/markdown` answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateResult {
    #[serde(default)]
    pub applied: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_seq: Option<i64>,
    /// Set in `suggest` mode (assumed field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion_id: Option<String>,
}

/// `POST /databases` body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateDatabase {
    pub parent: Parent,
    pub title: String,
    /// `[{"name": "Status", "type": "select", "options": [...]}]`.
    #[serde(default)]
    pub properties: Vec<Value>,
}

/// One row of `rows/upsert`: no `id` creates a row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowUpsert {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub values: Map<String, Value>,
}

/// `rows/upsert` answer (assumed `{ids: [...]}`, in input order).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpsertResult {
    #[serde(default)]
    pub ids: Vec<String>,
}

/// W3C TextQuoteSelector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextQuote {
    pub exact: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

/// Anchor of a new comment thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NewAnchor {
    Page,
    Quote { quote: TextQuote },
}

impl Client {
    /// `POST /pages` (one page with a Markdown body).
    pub async fn create_page(&self, parent: &Parent, page: &NewPage) -> Result<ResourceRef> {
        self.post("/pages", &CreatePage { parent, page }).await
    }

    /// `POST /pages/batch`.
    pub async fn create_pages(&self, request: &CreatePages) -> Result<Vec<ResourceRef>> {
        Ok(self.post::<_, CreatedPages>("/pages/batch", request).await?.pages)
    }

    pub async fn update_page(&self, page_id: &str, update: &UpdatePage) -> Result<UpdateResult> {
        self.patch(&format!("/pages/{}/markdown", encode(page_id)), update)
            .await
    }

    pub async fn move_page(&self, page_id: &str, parent: &Parent) -> Result<ResourceRef> {
        self.post(
            &format!("/pages/{}/move", encode(page_id)),
            &serde_json::json!({ "parent": parent }),
        )
        .await
    }

    pub async fn duplicate_page(&self, page_id: &str) -> Result<ResourceRef> {
        self.post(
            &format!("/pages/{}/duplicate", encode(page_id)),
            &serde_json::json!({}),
        )
        .await
    }

    pub async fn create_database(&self, request: &CreateDatabase) -> Result<ResourceRef> {
        self.post("/databases", request).await
    }

    /// `PATCH /databases/{id}`: add/change/remove properties and views (`changes` passed through).
    pub async fn update_database(&self, database_id: &str, changes: &Value) -> Result<Value> {
        self.patch(&format!("/databases/{}", encode(database_id)), changes)
            .await
    }

    pub async fn upsert_rows(&self, database_id: &str, rows: &[RowUpsert]) -> Result<UpsertResult> {
        self.post(
            &format!("/databases/{}/rows/upsert", encode(database_id)),
            &serde_json::json!({ "rows": rows }),
        )
        .await
    }

    pub async fn delete_rows(&self, database_id: &str, ids: &[String]) -> Result<Value> {
        self.post(
            &format!("/databases/{}/rows/delete", encode(database_id)),
            &serde_json::json!({ "ids": ids }),
        )
        .await
    }

    /// `POST /resources/{id}/comments` with a plain-text body.
    pub async fn create_comment(&self, resource_id: &str, anchor: &NewAnchor, text: &str) -> Result<Thread> {
        let body = serde_json::json!({ "anchor": anchor, "body": text_doc(text) });
        self.post(&format!("/resources/{}/comments", encode(resource_id)), &body)
            .await
    }

    /// `POST /comments/{thread}/replies`.
    pub async fn reply_comment(&self, thread_id: &str, text: &str) -> Result<Value> {
        self.post(
            &format!("/comments/{}/replies", encode(thread_id)),
            &serde_json::json!({ "body": text_doc(text) }),
        )
        .await
    }
}

// Suggestions (P4).

/// One change of a suggestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuggestionChange {
    /// Change id, for accepting part of a suggestion (assumed field).
    #[serde(default)]
    pub id: Option<String>,
    /// `insert`, `delete` or `replace`.
    pub kind: String,
    /// `{quote: {exact, prefix, suffix}}` or `{after_quote: …}`.
    #[serde(default)]
    pub anchor: Value,
    #[serde(default)]
    pub before_nfm: Option<String>,
    #[serde(default)]
    pub after_nfm: Option<String>,
    /// `pending`, `accepted`, `rejected`, `stale` (assumed per-change field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// `GET /pages/{id}/suggestions[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    #[serde(default)]
    pub page_id: Option<String>,
    #[serde(default)]
    pub author: Option<CommentAuthor>,
    #[serde(default)]
    pub via_agent: Option<String>,
    /// `pending`, `accepted`, `rejected`, `stale`.
    pub status: String,
    #[serde(default)]
    pub base_seq: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub changes: Vec<SuggestionChange>,
}

/// Answer of accept/reject (assumed `{id, status, stale_change_ids[]}`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResolveResult {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub stale_change_ids: Vec<String>,
}

impl Client {
    pub async fn suggestions(&self, page_id: &str) -> Result<Vec<Suggestion>> {
        self.get(&format!("/pages/{}/suggestions", encode(page_id))).await
    }

    /// `POST /suggestions/{id}/accept {change_ids?}`; `None` accepts every change.
    pub async fn accept_suggestion(&self, id: &str, change_ids: Option<&[String]>) -> Result<ResolveResult> {
        let body = match change_ids {
            Some(ids) => serde_json::json!({ "change_ids": ids }),
            None => serde_json::json!({}),
        };
        self.post(&format!("/suggestions/{}/accept", encode(id)), &body)
            .await
    }

    pub async fn reject_suggestion(&self, id: &str) -> Result<ResolveResult> {
        self.post(
            &format!("/suggestions/{}/reject", encode(id)),
            &serde_json::json!({}),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_id_from_urls() {
        let id = "3f0c2a1e-1b2c-4d5e-8f90-123456789abc";
        assert_eq!(resource_id(id), id);
        assert_eq!(resource_id(&format!("https://toantranmei.com/notes/{id}")), id);
        assert_eq!(
            resource_id(&format!("https://toantranmei.com/notes/ke-hoach-{id}?x=1")),
            id
        );
        assert_eq!(resource_id(&format!("http://localhost:3000/n/{id}/")), id);
        assert_eq!(resource_id("not-a-url"), "not-a-url");
    }

    #[test]
    fn plain_text_of_editor_json() {
        let doc = serde_json::json!({"type": "doc", "content": [
            {"type": "paragraph", "content": [{"type": "text", "text": "Xin "}, {"type": "text", "text": "chào"}]},
            {"type": "paragraph", "content": [{"type": "text", "text": "dòng 2"}]}
        ]});
        assert_eq!(plain_text(&doc), "Xin chào\ndòng 2");
        assert_eq!(plain_text(&text_doc("a\nb")), "a\nb");
    }

    #[test]
    fn query_request_skips_empty_fields() {
        let q = QueryRequest {
            sql: Some("SELECT * FROM rows".into()),
            ..QueryRequest::default()
        };
        assert_eq!(
            serde_json::to_value(&q).unwrap(),
            serde_json::json!({"sql": "SELECT * FROM rows"})
        );
    }

    #[test]
    fn page_markdown_parses() {
        let page: PageMarkdown = serde_json::from_value(serde_json::json!({
            "id": "p1", "title": "Kế hoạch", "icon": null, "url": "https://x/p1", "parent": {"page_id": "p0"},
            "markdown": "# Kế hoạch\n<unknown url=\"…\" alt=\"sketch\"/>", "truncated": true, "unknown_block_ids": ["b1"]
        }))
        .unwrap();
        assert!(page.truncated);
        assert_eq!(page.unknown_block_ids, ["b1"]);
    }

    #[test]
    fn write_bodies() {
        let parent = Parent::page("p0");
        assert!(parent.is_valid());
        assert!(!Parent::default().is_valid());
        let page = NewPage {
            title: "T".into(),
            markdown: Some("# T".into()),
            ..NewPage::default()
        };
        assert_eq!(
            serde_json::to_value(CreatePage {
                parent: &parent,
                page: &page
            })
            .unwrap(),
            serde_json::json!({"parent": {"page_id": "p0"}, "title": "T", "markdown": "# T"})
        );
        let update = UpdatePage {
            update_content: Some(vec![ContentEdit {
                old_str: "a".into(),
                new_str: "b".into(),
                replace_all: false,
            }]),
            mode: Some(WriteMode::Suggest),
            ..UpdatePage::default()
        };
        assert_eq!(
            serde_json::to_value(&update).unwrap(),
            serde_json::json!({"update_content": [{"old_str": "a", "new_str": "b"}], "mode": "suggest"})
        );
        let anchor = NewAnchor::Quote {
            quote: TextQuote {
                exact: "x".into(),
                prefix: None,
                suffix: None,
            },
        };
        assert_eq!(
            serde_json::to_value(anchor).unwrap(),
            serde_json::json!({"kind": "quote", "quote": {"exact": "x"}})
        );
    }
}
