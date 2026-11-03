//! The data source of the tools.
//!
//! Every method has a default that answers `unsupported`, so a backend can grow tool by tool; [`HttpBackend`]
//! implements all of them over `/api/v1`.

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use ttm_api::notee::{
    CreateDatabase, CreatePages, DatabaseSchema, NewAnchor, PageMarkdown, Parent, QueryRequest, QueryResult,
    ResourceRef, RowUpsert, SearchHit, Thread, UpdatePage, UpdateResult, UpsertResult,
};
use ttm_api::{ApiError, Client, MeUser, Member, WorkspaceSummary};

/// An error a tool reports back to the model (as a tool error, not a protocol error).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{message} ({code})")]
pub struct BackendError {
    /// Stable code (`unauthorized`, `insufficient_scope`, `not_found`, …).
    pub code: String,
    pub message: String,
}

impl BackendError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn unsupported(what: &str) -> Self {
        Self::new("unsupported", format!("{what} is not available on this server"))
    }
}

impl From<ApiError> for BackendError {
    fn from(err: ApiError) -> Self {
        let hint = if err.is_unauthorized() {
            " Sign in again with `ttm login`."
        } else {
            ""
        };
        Self {
            code: err.code().to_string(),
            message: format!("{err}.{hint}"),
        }
    }
}

pub type BackendResult<T> = Result<T, BackendError>;

/// `workspace-get-self`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelfInfo {
    pub user: MeUser,
    /// The current workspace (profile choice, else the personal workspace).
    pub workspace: Option<WorkspaceSummary>,
}

/// `notee-fetch`: a page as Markdown or a database schema.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Fetched {
    Page(PageMarkdown),
    Database(DatabaseSchema),
}

/// Everything the tools need, one async method per operation.
#[async_trait]
pub trait Backend: Send + Sync + 'static {
    async fn get_self(&self) -> BackendResult<SelfInfo>;
    async fn list_workspaces(&self) -> BackendResult<Vec<WorkspaceSummary>>;
    /// Members of `workspace_id`, or of the current workspace when `None`.
    async fn get_users(&self, workspace_id: Option<&str>) -> BackendResult<Vec<Member>>;

    // Notee, read.
    async fn notee_search(&self, _query: &str, _workspace_id: Option<&str>) -> BackendResult<Vec<SearchHit>> {
        Err(BackendError::unsupported("notee-search"))
    }
    /// `id` of a page or database; `block` continues a truncated page.
    async fn notee_fetch(&self, _id: &str, _block: Option<&str>) -> BackendResult<Fetched> {
        Err(BackendError::unsupported("notee-fetch"))
    }
    async fn notee_query_database(
        &self,
        _database_id: &str,
        _query: &QueryRequest,
    ) -> BackendResult<QueryResult> {
        Err(BackendError::unsupported("notee-query-database"))
    }
    async fn notee_get_comments(&self, _resource_id: &str) -> BackendResult<Vec<Thread>> {
        Err(BackendError::unsupported("notee-get-comments"))
    }

    // Notee, write.
    async fn notee_create_pages(&self, _request: &CreatePages) -> BackendResult<Vec<ResourceRef>> {
        Err(BackendError::unsupported("notee-create-pages"))
    }
    async fn notee_update_page(&self, _page_id: &str, _update: &UpdatePage) -> BackendResult<UpdateResult> {
        Err(BackendError::unsupported("notee-update-page"))
    }
    async fn notee_move_page(&self, _page_id: &str, _parent: &Parent) -> BackendResult<ResourceRef> {
        Err(BackendError::unsupported("notee-move-pages"))
    }
    async fn notee_duplicate_page(&self, _page_id: &str) -> BackendResult<ResourceRef> {
        Err(BackendError::unsupported("notee-duplicate-page"))
    }
    async fn notee_create_database(&self, _request: &CreateDatabase) -> BackendResult<ResourceRef> {
        Err(BackendError::unsupported("notee-create-database"))
    }
    async fn notee_update_database(&self, _database_id: &str, _changes: &Value) -> BackendResult<Value> {
        Err(BackendError::unsupported("notee-update-database"))
    }
    async fn notee_upsert_rows(
        &self,
        _database_id: &str,
        _rows: &[RowUpsert],
    ) -> BackendResult<UpsertResult> {
        Err(BackendError::unsupported("notee-upsert-rows"))
    }
    async fn notee_delete_rows(&self, _database_id: &str, _ids: &[String]) -> BackendResult<Value> {
        Err(BackendError::unsupported("notee-delete-rows"))
    }
    async fn notee_create_comment(
        &self,
        _resource_id: &str,
        _anchor: &NewAnchor,
        _text: &str,
    ) -> BackendResult<Thread> {
        Err(BackendError::unsupported("notee-create-comment"))
    }
    async fn notee_reply_comment(&self, _thread_id: &str, _text: &str) -> BackendResult<Value> {
        Err(BackendError::unsupported("notee-create-comment"))
    }
}

/// [`Backend`] over the public HTTP API.
#[derive(Debug, Clone)]
pub struct HttpBackend {
    client: Client,
    /// Workspace id chosen with `ttm workspace switch`.
    current_workspace: Option<String>,
}

impl HttpBackend {
    pub fn new(client: Client, current_workspace: Option<String>) -> Self {
        Self {
            client,
            current_workspace,
        }
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    fn pick_current(&self, workspaces: &[WorkspaceSummary]) -> Option<WorkspaceSummary> {
        self.current_workspace
            .as_deref()
            .and_then(|id| workspaces.iter().find(|w| w.id == id))
            .or_else(|| workspaces.iter().find(|w| w.kind == "personal"))
            .or_else(|| workspaces.first())
            .cloned()
    }
}

#[async_trait]
impl Backend for HttpBackend {
    async fn get_self(&self) -> BackendResult<SelfInfo> {
        let me = self.client.me().await?;
        let workspace = self.pick_current(&me.workspaces);
        Ok(SelfInfo {
            user: me.user,
            workspace,
        })
    }

    async fn list_workspaces(&self) -> BackendResult<Vec<WorkspaceSummary>> {
        Ok(self.client.me().await?.workspaces)
    }

    async fn get_users(&self, workspace_id: Option<&str>) -> BackendResult<Vec<Member>> {
        let id = match workspace_id {
            Some(id) => id.to_string(),
            None => match &self.current_workspace {
                Some(id) => id.clone(),
                None => self
                    .pick_current(&self.client.me().await?.workspaces)
                    .map(|w| w.id)
                    .ok_or_else(|| BackendError::new("not_found", "no workspace available"))?,
            },
        };
        Ok(self.client.workspace_members(&id).await?)
    }

    async fn notee_search(&self, query: &str, workspace_id: Option<&str>) -> BackendResult<Vec<SearchHit>> {
        Ok(self
            .client
            .search(query, workspace_id.or(self.current_workspace.as_deref()))
            .await?)
    }

    async fn notee_fetch(&self, id: &str, block: Option<&str>) -> BackendResult<Fetched> {
        if block.is_some() {
            return Ok(Fetched::Page(self.client.page_markdown(id, block).await?));
        }
        // Ask what it is first; servers without the summary endpoint get the page attempt.
        let kind = match self.client.resource_summary(id).await {
            Ok(summary) => summary.kind,
            Err(err) if err.status() == Some(404) => "page".into(),
            Err(err) => return Err(err.into()),
        };
        if kind == "database" {
            let mut schema = self.client.database_schema(id).await?;
            schema.id.get_or_insert_with(|| id.to_string());
            Ok(Fetched::Database(schema))
        } else {
            Ok(Fetched::Page(self.client.page_markdown(id, None).await?))
        }
    }

    async fn notee_query_database(
        &self,
        database_id: &str,
        query: &QueryRequest,
    ) -> BackendResult<QueryResult> {
        Ok(self.client.query_database(database_id, query).await?)
    }

    async fn notee_get_comments(&self, resource_id: &str) -> BackendResult<Vec<Thread>> {
        Ok(self.client.comments(resource_id).await?)
    }

    async fn notee_create_pages(&self, request: &CreatePages) -> BackendResult<Vec<ResourceRef>> {
        Ok(self.client.create_pages(request).await?)
    }

    async fn notee_update_page(&self, page_id: &str, update: &UpdatePage) -> BackendResult<UpdateResult> {
        Ok(self.client.update_page(page_id, update).await?)
    }

    async fn notee_move_page(&self, page_id: &str, parent: &Parent) -> BackendResult<ResourceRef> {
        Ok(self.client.move_page(page_id, parent).await?)
    }

    async fn notee_duplicate_page(&self, page_id: &str) -> BackendResult<ResourceRef> {
        Ok(self.client.duplicate_page(page_id).await?)
    }

    async fn notee_create_database(&self, request: &CreateDatabase) -> BackendResult<ResourceRef> {
        Ok(self.client.create_database(request).await?)
    }

    async fn notee_update_database(&self, database_id: &str, changes: &Value) -> BackendResult<Value> {
        Ok(self.client.update_database(database_id, changes).await?)
    }

    async fn notee_upsert_rows(&self, database_id: &str, rows: &[RowUpsert]) -> BackendResult<UpsertResult> {
        Ok(self.client.upsert_rows(database_id, rows).await?)
    }

    async fn notee_delete_rows(&self, database_id: &str, ids: &[String]) -> BackendResult<Value> {
        Ok(self.client.delete_rows(database_id, ids).await?)
    }

    async fn notee_create_comment(
        &self,
        resource_id: &str,
        anchor: &NewAnchor,
        text: &str,
    ) -> BackendResult<Thread> {
        Ok(self.client.create_comment(resource_id, anchor, text).await?)
    }

    async fn notee_reply_comment(&self, thread_id: &str, text: &str) -> BackendResult<Value> {
        Ok(self.client.reply_comment(thread_id, text).await?)
    }
}
