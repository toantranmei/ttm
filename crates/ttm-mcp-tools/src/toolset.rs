//! [`ToolSet`]: the MCP server handler.

use crate::common::{CallContext, ToolFailure, ToolResult};
use crate::{Backend, notee, notee_suggest, notee_write, workspace};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, ReadResourceRequestParams,
    ReadResourceResponse, ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerConfig,
    Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler};
use std::sync::Arc;

/// A group of tools that can be switched on with `--toolsets`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Toolset {
    /// Always on.
    Workspace,
    Notee,
    Factory,
}

impl std::str::FromStr for Toolset {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "workspace" => Ok(Toolset::Workspace),
            "notee" | "notes" => Ok(Toolset::Notee),
            "factory" => Ok(Toolset::Factory),
            other => Err(format!(
                "unknown toolset `{other}` (expected workspace, notee, factory)"
            )),
        }
    }
}

/// Enabled toolsets. `workspace` is always included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toolsets(Vec<Toolset>);

impl Toolsets {
    pub fn all() -> Self {
        Self(vec![Toolset::Workspace, Toolset::Notee, Toolset::Factory])
    }

    /// `notee,factory` → workspace + notee + factory.
    pub fn parse(list: &str) -> Result<Self, String> {
        let mut sets = vec![Toolset::Workspace];
        for part in list.split(',').filter(|p| !p.trim().is_empty()) {
            let set: Toolset = part.parse()?;
            if !sets.contains(&set) {
                sets.push(set);
            }
        }
        Ok(Self(sets))
    }

    pub fn contains(&self, set: Toolset) -> bool {
        self.0.contains(&set)
    }
}

impl Default for Toolsets {
    fn default() -> Self {
        Self::all()
    }
}

pub const INSTRUCTIONS: &str = "Tools for toantranmei.com (Notee notes, Factory projects) acting as the signed-in user. \
Start with workspace-get-self to learn who the user is and which workspace is current.";

/// MCP server over a [`Backend`].
pub struct ToolSet<B: Backend> {
    backend: Arc<B>,
    toolsets: Toolsets,
    server_name: String,
    server_version: String,
}

impl<B: Backend> Clone for ToolSet<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
            toolsets: self.toolsets.clone(),
            server_name: self.server_name.clone(),
            server_version: self.server_version.clone(),
        }
    }
}

impl<B: Backend> ToolSet<B> {
    pub fn new(backend: B, toolsets: Toolsets) -> Self {
        Self::from_arc(Arc::new(backend), toolsets)
    }

    pub fn from_arc(backend: Arc<B>, toolsets: Toolsets) -> Self {
        Self {
            backend,
            toolsets,
            server_name: "ttm".into(),
            server_version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    /// Name and version reported in `initialize` (the hosted server sets its own).
    pub fn with_server_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.server_name = name.into();
        self.server_version = version.into();
        self
    }

    /// Tools of the enabled toolsets.
    pub fn tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();
        if self.toolsets.contains(Toolset::Workspace) {
            tools.extend(workspace::tools());
        }
        if self.toolsets.contains(Toolset::Notee) {
            tools.extend(notee::tools());
            tools.extend(notee_write::tools());
            tools.extend(notee_suggest::tools());
        }
        tools
    }

    /// Runs a tool by name (also used by tests without a transport).
    pub async fn call(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        self.call_with(name, arguments, &CallContext::default()).await
    }

    /// [`ToolSet::call`] with caller information (the MCP client name is recorded as the writing agent).
    pub async fn call_with(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
        ctx: &CallContext,
    ) -> Result<CallToolResult, ErrorData> {
        if !self.tools().iter().any(|t| t.name == name) {
            return Err(ErrorData::invalid_params(format!("unknown tool `{name}`"), None));
        }
        let outcome = match self.dispatch(name, arguments, ctx).await {
            Some(outcome) => outcome,
            None => return Err(ErrorData::invalid_params(format!("unknown tool `{name}`"), None)),
        };
        match outcome {
            Ok(output) => {
                let mut result = CallToolResult::structured(output.value);
                if let Some(text) = output.text {
                    result.content = vec![ContentBlock::text(text)];
                }
                Ok(result)
            }
            Err(ToolFailure::InvalidArgs(message)) => Err(ErrorData::invalid_params(message, None)),
            Err(ToolFailure::Backend(err)) => {
                tracing::debug!(tool = name, code = %err.code, "tool failed");
                let mut result = CallToolResult::error(vec![ContentBlock::text(err.to_string())]);
                result.structured_content = Some(serde_json::json!({ "error": err }));
                Ok(result)
            }
        }
    }
}

impl<B: Backend> ToolSet<B> {
    async fn dispatch(
        &self,
        name: &str,
        args: Option<serde_json::Map<String, serde_json::Value>>,
        ctx: &CallContext,
    ) -> Option<ToolResult> {
        let backend = self.backend.as_ref();
        if name.starts_with("workspace-") {
            return workspace::call(backend, name, args).await;
        }
        if name.starts_with("notee-") {
            if let Some(result) = notee::call(backend, name, args.clone()).await {
                return Some(result);
            }
            if let Some(result) = notee_suggest::call(backend, name, args.clone()).await {
                return Some(result);
            }
            return notee_write::call(backend, name, args, ctx).await;
        }
        None
    }

    /// MCP resources: the Notee Markdown spec (when Notee is on).
    pub fn resources(&self) -> Vec<Resource> {
        if !self.toolsets.contains(Toolset::Notee) {
            return vec![];
        }
        vec![
            Resource::new(notee::SPEC_URI, "notee-markdown")
                .with_title("Notee-flavored Markdown")
                .with_description("How Notee pages are written as Markdown: tags for callouts, toggles, columns, mentions, databases.")
                .with_mime_type("text/markdown"),
        ]
    }

    pub fn read(&self, uri: &str) -> Option<String> {
        (uri == notee::SPEC_URI && self.toolsets.contains(Toolset::Notee))
            .then(|| notee::SPEC_MARKDOWN.to_string())
    }
}

impl<B: Backend> ServerHandler for ToolSet<B> {
    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        Ok(ListResourcesResult::with_all_items(self.resources()))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        match self.read(&request.uri) {
            Some(text) => Ok(ReadResourceResult::new(vec![
                ResourceContents::text(text, request.uri.clone()).with_mime_type("text/markdown"),
            ])
            .into()),
            None => Err(ErrorData::resource_not_found(
                format!("unknown resource {}", request.uri),
                None,
            )),
        }
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_server_info(Implementation::new(
            self.server_name.clone(),
            self.server_version.clone(),
        ))
        .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(self.tools()))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let agent = context.peer.peer_info().map(|info| info.client_info.name.clone());
        let ctx = CallContext { agent };
        self.call_with(&request.name, request.arguments, &ctx)
            .await
            .map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BackendError, SelfInfo};
    use async_trait::async_trait;
    use ttm_api::{MeUser, Member, MemberUser, WorkspaceSummary};

    struct Fake;

    fn ws(id: &str, kind: &str) -> WorkspaceSummary {
        WorkspaceSummary {
            id: id.into(),
            name: id.into(),
            kind: kind.into(),
            role: "owner".into(),
            slug: None,
        }
    }

    #[async_trait]
    impl Backend for Fake {
        async fn get_self(&self) -> Result<SelfInfo, BackendError> {
            Ok(SelfInfo {
                user: MeUser {
                    id: "u1".into(),
                    email: "an@example.com".into(),
                    name: "An".into(),
                    avatar_url: None,
                    locale: None,
                    plan: None,
                },
                workspace: Some(ws("w1", "personal")),
            })
        }
        async fn list_workspaces(&self) -> Result<Vec<WorkspaceSummary>, BackendError> {
            Ok(vec![ws("w1", "personal"), ws("w2", "team")])
        }
        async fn get_users(&self, workspace_id: Option<&str>) -> Result<Vec<Member>, BackendError> {
            if workspace_id == Some("forbidden") {
                return Err(BackendError::new("forbidden", "guests cannot list members"));
            }
            let member = |name: &str, email: &str| Member {
                user: MemberUser {
                    id: name.into(),
                    name: name.into(),
                    email: email.into(),
                    avatar_url: None,
                },
                role: "member".into(),
            };
            Ok(vec![
                member("An", "an@example.com"),
                member("Bình", "binh@example.com"),
            ])
        }
    }

    fn args(v: serde_json::Value) -> Option<serde_json::Map<String, serde_json::Value>> {
        v.as_object().cloned()
    }

    #[test]
    fn toolsets_parse() {
        assert_eq!(Toolsets::parse("").unwrap(), Toolsets(vec![Toolset::Workspace]));
        let sets = Toolsets::parse("notee, factory,notee").unwrap();
        assert!(
            sets.contains(Toolset::Notee)
                && sets.contains(Toolset::Factory)
                && sets.contains(Toolset::Workspace)
        );
        assert!(Toolsets::parse("languu").is_err());
    }

    #[test]
    fn lists_workspace_tools_with_schemas() {
        let set = ToolSet::new(Fake, Toolsets::parse("").unwrap());
        let names: Vec<_> = set.tools().iter().map(|t| t.name.to_string()).collect();
        assert_eq!(
            names,
            ["workspace-get-self", "workspace-list", "workspace-get-users"]
        );
        let users = set
            .tools()
            .into_iter()
            .find(|t| t.name == "workspace-get-users")
            .unwrap();
        let props = users.input_schema.get("properties").unwrap();
        assert!(props.get("workspace_id").is_some() && props.get("query").is_some());
        assert!(users.annotations.unwrap().read_only_hint.unwrap());
    }

    #[tokio::test]
    async fn calls_tools() {
        let set = ToolSet::new(Fake, Toolsets::default());
        let me = set.call("workspace-get-self", None).await.unwrap();
        assert_eq!(me.structured_content.unwrap()["user"]["email"], "an@example.com");
        let list = set.call("workspace-list", None).await.unwrap();
        assert_eq!(
            list.structured_content.unwrap()["workspaces"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let users = set
            .call("workspace-get-users", args(serde_json::json!({"query": "BÌNH"})))
            .await
            .unwrap();
        let users = users.structured_content.unwrap();
        assert_eq!(users["users"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn errors() {
        let set = ToolSet::new(Fake, Toolsets::default());
        assert!(
            set.call("nope", None).await.is_err(),
            "unknown tool is a protocol error"
        );
        assert!(
            set.call("workspace-get-users", args(serde_json::json!({"bogus": 1})))
                .await
                .is_err()
        );
        let failed = set
            .call(
                "workspace-get-users",
                args(serde_json::json!({"workspace_id": "forbidden"})),
            )
            .await
            .unwrap();
        assert_eq!(failed.is_error, Some(true));
        assert_eq!(failed.structured_content.unwrap()["error"]["code"], "forbidden");
    }
}
