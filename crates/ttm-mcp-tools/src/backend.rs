//! The data source of the tools.
//!
//! Every method has a default that answers `unsupported`, so a backend can grow tool by tool; [`HttpBackend`]
//! implements all of them over `/api/v1`.

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use ttm_api::factory::{Lease, RuleSet, RunEvent, Task};
use ttm_api::notee::{
    CreateDatabase, CreatePages, DatabaseSchema, NewAnchor, PageMarkdown, Parent, QueryRequest, QueryResult,
    ResolveResult, ResourceRef, RowUpsert, SearchHit, Suggestion, Thread, UpdatePage, UpdateResult,
    UpsertResult,
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

    // Notee, suggestions.
    async fn notee_get_suggestions(&self, _page_id: &str) -> BackendResult<Vec<Suggestion>> {
        Err(BackendError::unsupported("notee-get-suggestions"))
    }
    /// `accept`: all changes, or only `change_ids`; otherwise reject.
    async fn notee_resolve_suggestion(
        &self,
        _id: &str,
        _accept: bool,
        _change_ids: Option<&[String]>,
    ) -> BackendResult<ResolveResult> {
        Err(BackendError::unsupported("notee-resolve-suggestion"))
    }

    // Factory (agent inside a run). `None` arguments mean the current task / run / project of the job.
    async fn factory_get_task(&self, _task: Option<&str>) -> BackendResult<Task> {
        Err(BackendError::unsupported("factory-get-task"))
    }
    async fn factory_get_context(&self, _task: Option<&str>) -> BackendResult<Value> {
        Err(BackendError::unsupported("factory-get-context"))
    }
    async fn factory_report_progress(&self, _message: &str, _percent: Option<u8>) -> BackendResult<()> {
        Err(BackendError::unsupported("factory-report-progress"))
    }
    async fn factory_request_approval(&self, _reason: &str) -> BackendResult<Value> {
        Err(BackendError::unsupported("factory-request-approval"))
    }
    async fn factory_create_subtasks(&self, _parent: Option<&str>, _tasks: &[Value]) -> BackendResult<Value> {
        Err(BackendError::unsupported("factory-create-subtasks"))
    }
    async fn factory_attach_artifact(&self, _artifact: &Value) -> BackendResult<Value> {
        Err(BackendError::unsupported("factory-attach-artifact"))
    }
    async fn factory_get_rules(&self, _project: Option<&str>) -> BackendResult<RuleSet> {
        Err(BackendError::unsupported("factory-get-rules"))
    }
    async fn factory_renew_lease(&self, _paths: &[String]) -> BackendResult<Lease> {
        Err(BackendError::unsupported("factory-renew-lease"))
    }
}

/// The job an agent works in, from `TTM_RUN_ID`, `TTM_TASK`, `TTM_PROJECT` (set by the runner).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FactoryScope {
    pub run_id: Option<String>,
    pub task: Option<String>,
    pub project: Option<String>,
}

impl FactoryScope {
    pub fn from_env() -> Self {
        let var = |n: &str| std::env::var(n).ok().filter(|v| !v.is_empty());
        Self {
            run_id: var("TTM_RUN_ID"),
            task: var("TTM_TASK"),
            project: var("TTM_PROJECT"),
        }
    }
}

/// [`Backend`] over the public HTTP API.
#[derive(Debug, Clone)]
pub struct HttpBackend {
    client: Client,
    /// Workspace id chosen with `ttm workspace switch`.
    current_workspace: Option<String>,
    factory: FactoryScope,
}

impl HttpBackend {
    pub fn new(client: Client, current_workspace: Option<String>) -> Self {
        Self {
            client,
            current_workspace,
            factory: FactoryScope::default(),
        }
    }

    /// Current run / task / project for the Factory tools.
    pub fn with_factory_scope(mut self, scope: FactoryScope) -> Self {
        self.factory = scope;
        self
    }

    fn run_id(&self) -> BackendResult<&str> {
        self.factory.run_id.as_deref().ok_or_else(|| {
            BackendError::new(
                "no_run",
                "this tool works inside an agent run (TTM_RUN_ID is not set)",
            )
        })
    }

    fn task_or_current<'a>(&'a self, task: Option<&'a str>) -> BackendResult<&'a str> {
        task.or(self.factory.task.as_deref())
            .ok_or_else(|| BackendError::new("no_task", "give `task` (no current task: TTM_TASK is not set)"))
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

    async fn notee_get_suggestions(&self, page_id: &str) -> BackendResult<Vec<Suggestion>> {
        Ok(self.client.suggestions(page_id).await?)
    }

    async fn notee_resolve_suggestion(
        &self,
        id: &str,
        accept: bool,
        change_ids: Option<&[String]>,
    ) -> BackendResult<ResolveResult> {
        if accept {
            Ok(self.client.accept_suggestion(id, change_ids).await?)
        } else {
            Ok(self.client.reject_suggestion(id).await?)
        }
    }

    async fn factory_get_task(&self, task: Option<&str>) -> BackendResult<Task> {
        Ok(self.client.task(self.task_or_current(task)?).await?)
    }

    async fn factory_get_context(&self, task: Option<&str>) -> BackendResult<Value> {
        match (task, self.factory.run_id.as_deref()) {
            (None, Some(run)) => Ok(self.client.run_context(run).await?),
            _ => Ok(self.client.task_context(self.task_or_current(task)?).await?),
        }
    }

    async fn factory_report_progress(&self, message: &str, percent: Option<u8>) -> BackendResult<()> {
        let data = percent.map(|p| serde_json::json!({ "percent": p }));
        Ok(self
            .client
            .post_run_events(self.run_id()?, &[RunEvent::now("progress", message, data)])
            .await?)
    }

    async fn factory_request_approval(&self, reason: &str) -> BackendResult<Value> {
        Ok(self.client.request_approval(self.run_id()?, reason).await?)
    }

    async fn factory_create_subtasks(&self, parent: Option<&str>, tasks: &[Value]) -> BackendResult<Value> {
        let parent = self.task_or_current(parent)?;
        Ok(self
            .client
            // The job token already names its run (the server takes it from there); a different `run_id` is refused.
            .create_subtasks(parent, tasks, None)
            .await?)
    }

    async fn factory_attach_artifact(&self, artifact: &Value) -> BackendResult<Value> {
        Ok(self.client.attach_artifact(self.run_id()?, artifact).await?)
    }

    async fn factory_get_rules(&self, project: Option<&str>) -> BackendResult<RuleSet> {
        let project = project
            .or(self.factory.project.as_deref())
            .ok_or_else(|| BackendError::new("no_project", "give `project` (TTM_PROJECT is not set)"))?;
        let id = match self.client.find_project(project).await {
            Ok(Some(p)) => p.id,
            _ => project.to_string(),
        };
        Ok(self.client.rules(&id).await?)
    }

    async fn factory_renew_lease(&self, paths: &[String]) -> BackendResult<Lease> {
        Ok(self.client.request_lease(self.run_id()?, paths).await?)
    }
}
