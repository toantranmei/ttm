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

/// `GET /resources/{id}/summary` (assumed shape: `{id, kind: "page"|"database", title, parent?, url?}`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub id: String,
    /// `page` or `database`.
    pub kind: String,
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
    /// Anchored text (added by the server for agents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
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
}
