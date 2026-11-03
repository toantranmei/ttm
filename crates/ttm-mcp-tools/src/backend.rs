//! The data source of the tools.

use async_trait::async_trait;
use serde::Serialize;
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

/// `workspace-get-self`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SelfInfo {
    pub user: MeUser,
    /// The current workspace (profile choice, else the personal workspace).
    pub workspace: Option<WorkspaceSummary>,
}

/// Everything the tools need, one async method per tool.
#[async_trait]
pub trait Backend: Send + Sync + 'static {
    async fn get_self(&self) -> Result<SelfInfo, BackendError>;
    async fn list_workspaces(&self) -> Result<Vec<WorkspaceSummary>, BackendError>;
    /// Members of `workspace_id`, or of the current workspace when `None`.
    async fn get_users(&self, workspace_id: Option<&str>) -> Result<Vec<Member>, BackendError>;
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
    async fn get_self(&self) -> Result<SelfInfo, BackendError> {
        let me = self.client.me().await?;
        let workspace = self.pick_current(&me.workspaces);
        Ok(SelfInfo {
            user: me.user,
            workspace,
        })
    }

    async fn list_workspaces(&self) -> Result<Vec<WorkspaceSummary>, BackendError> {
        Ok(self.client.me().await?.workspaces)
    }

    async fn get_users(&self, workspace_id: Option<&str>) -> Result<Vec<Member>, BackendError> {
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
}
