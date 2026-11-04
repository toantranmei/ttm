//! Markdown of five representative tools (one shape: title, tables or key/values, Next). Review with
//! `cargo insta review`.

mod support;

use serde_json::json;
use support::{Fake, args, text};
use ttm_mcp_tools::{ToolSet, Toolsets};

async fn md(name: &str, a: serde_json::Value) -> String {
    let set = ToolSet::new(Fake::default(), Toolsets::all());
    text(&set.call(name, args(a)).await.unwrap())
}

#[tokio::test]
async fn markdown_snapshots() {
    insta::assert_snapshot!("workspace_get_self", md("workspace-get-self", json!({})).await);
    insta::assert_snapshot!("factory_get_task", md("factory-get-task", json!({})).await);
    insta::assert_snapshot!(
        "factory_workflow_get",
        md("factory-workflow-get", json!({})).await
    );
    insta::assert_snapshot!("factory_agent_list", md("factory-agent-list", json!({})).await);
    insta::assert_snapshot!(
        "factory_workflow_publish",
        md("factory-workflow-publish", json!({"note": "QC 2"})).await
    );
}
