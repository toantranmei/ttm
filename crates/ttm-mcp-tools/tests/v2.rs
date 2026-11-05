//! MCP v2: every tool declares an `outputSchema` and its `structuredContent` (always an object) matches it; admin
//! tools only stage changes; agents running a job never see or call them; elicitation applies or cancels.

mod support;

use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use support::{Fake, args, text};
use ttm_mcp_tools::common::CallContext;
use ttm_mcp_tools::{Asker, ToolSet, Toolsets};

fn people() -> ToolSet<Fake> {
    ToolSet::new(Fake::default(), Toolsets::all())
}

async fn check(set: &ToolSet<Fake>, name: &str, a: Value) -> rmcp::model::CallToolResult {
    let tool = set
        .tools()
        .into_iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("no tool {name}"));
    let schema = Value::Object(tool.output_schema.clone().expect("outputSchema").as_ref().clone());
    let result = set.call(name, args(a)).await.unwrap();
    assert_ne!(result.is_error, Some(true), "{name}: {}", text(&result));
    let value = result.structured_content.clone().expect("structuredContent");
    assert!(value.is_object(), "{name}: {value}");
    assert!(
        jsonschema::is_valid(&schema, &value),
        "{name}: {value} does not match {schema}"
    );
    assert!(
        text(&result).starts_with("**"),
        "{name}: Markdown title expected: {}",
        text(&result)
    );
    result
}

#[test]
fn every_tool_has_an_object_output_schema() {
    for tool in people().tools() {
        let schema = tool
            .output_schema
            .unwrap_or_else(|| panic!("{} has no outputSchema", tool.name));
        assert_eq!(schema.get("type"), Some(&json!("object")), "{}", tool.name);
    }
}

#[tokio::test]
async fn outputs_match_their_schemas() {
    let set = people();
    let me = check(&set, "workspace-get-self", json!({})).await;
    assert_eq!(me.structured_content.unwrap()["membership"]["tier"], "mei-er");
    check(&set, "workspace-list", json!({})).await;
    let users = check(&set, "workspace-get-users", json!({})).await;
    assert!(text(&users).contains("✦✦ Sponsor"));
    check(&set, "factory-get-task", json!({})).await;
    check(&set, "factory-workflow-get", json!({})).await;
    check(&set, "factory-workflow-get", json!({"version": "draft"})).await;
    let list = check(&set, "factory-agent-list", json!({})).await;
    assert!(
        list.structured_content.unwrap()["items"].is_array(),
        "lists are {{items}}"
    );
    check(&set, "factory-agent-get", json!({"agent": "@dev"})).await;
    check(&set, "factory-automation-list", json!({})).await;
    check(&set, "factory-automation-get", json!({"rule": "auto-assign"})).await;
    let skill = check(&set, "factory-skill-get", json!({"skill": "ttm-tdd"})).await;
    assert!(text(&skill).contains("Write the test first."));
    check(&set, "factory-template-search", json!({"kind": "workflow"})).await;
    check(&set, "factory-inbox", json!({"unread": true})).await;
    check(&set, "factory-workflow-publish", json!({"note": "QC 2"})).await;
}

#[tokio::test]
async fn admin_tools_only_stage() {
    let set = people();
    let staged = check(&set, "factory-workflow-publish", json!({"note": "QC 2"})).await;
    let v = staged.structured_content.clone().unwrap();
    assert_eq!(
        (v["confirm_id"].as_str(), v["status"].as_str()),
        (Some("c1"), Some("pending"))
    );
    assert_eq!(v["action"]["path"], "/factory/workflows/wf1/publish");
    assert!(
        text(&staged).starts_with(
            "**! Confirm: Publish workflow Professional v3 (1 Automation rule(s) stop firing)**"
        ),
        "{}",
        text(&staged)
    );
    assert!(text(&staged).contains("`factory-confirm`"));
    let calls = set_calls(&set);
    assert!(calls.contains(&"POST /confirmations".to_string()), "{calls:?}");
    assert!(
        !calls
            .iter()
            .any(|c| c.ends_with("/publish") || c.ends_with("/apply")),
        "never applies: {calls:?}"
    );
    let applied = set
        .call("factory-confirm", args(json!({"confirm_id": "c1"})))
        .await
        .unwrap();
    assert_ne!(applied.is_error, Some(true));
    assert_eq!(applied.structured_content.unwrap()["status"], "applied");
    assert!(set_calls(&set).contains(&"POST /confirmations/c1/apply".to_string()));
}

fn set_calls(set: &ToolSet<Fake>) -> Vec<String> {
    // The backend is shared; read its log through a fresh call list.
    set.backend().calls()
}

#[tokio::test]
async fn jobs_never_see_admin_tools() {
    let set = ToolSet::new(Fake::job(), Toolsets::all());
    assert!(set.is_job());
    let names: Vec<String> = set.tools().iter().map(|t| t.name.to_string()).collect();
    for admin in [
        "factory-workflow-publish",
        "factory-agent-draft",
        "factory-confirm",
        "factory-task-approve",
    ] {
        assert!(!names.contains(&admin.to_string()), "{admin} visible to a job");
    }
    for read in [
        "factory-workflow-get",
        "factory-skill-get",
        "factory-automation-list",
        "factory-get-task",
    ] {
        assert!(names.contains(&read.to_string()), "{read} missing for a job");
    }
    assert!(
        !names.contains(&"factory-agent-list".to_string()),
        "people-only read tool visible to a job"
    );
    assert!(
        set.call("factory-workflow-publish", args(json!({"note": "x"})))
            .await
            .is_err()
    );
    assert!(
        set.call("factory-confirm", args(json!({"confirm_id": "c1"})))
            .await
            .is_err()
    );
    // The hosted server marks job tokens explicitly.
    let hosted = ToolSet::new(Fake::default(), Toolsets::all()).for_job(true);
    assert!(!hosted.tools().iter().any(|t| t.name == "factory-confirm"));
}

struct Answer(Option<bool>, AtomicBool);

#[async_trait::async_trait]
impl Asker for Answer {
    async fn confirm(&self, message: &str) -> Option<bool> {
        assert!(message.contains("Apply this change?"));
        self.1.store(true, Ordering::SeqCst);
        self.0
    }
}

#[tokio::test]
async fn elicitation_accepts_or_declines() {
    let set = people();
    let ctx = CallContext::default();
    let yes = Answer(Some(true), AtomicBool::new(false));
    let out = set
        .call_confirming(
            "factory-workflow-publish",
            args(json!({"note": "QC 2"})),
            &ctx,
            &yes,
        )
        .await
        .unwrap();
    assert!(yes.1.load(Ordering::SeqCst));
    let v = out.structured_content.unwrap();
    assert_eq!(v["status"], "applied");
    assert_eq!(v["result"]["version"], 3);

    let set = people();
    let no = Answer(Some(false), AtomicBool::new(false));
    let out = set
        .call_confirming(
            "factory-workflow-publish",
            args(json!({"note": "QC 2"})),
            &ctx,
            &no,
        )
        .await
        .unwrap();
    assert_eq!(out.structured_content.unwrap()["status"], "cancelled");
    assert!(
        set.backend()
            .calls()
            .contains(&"POST /confirmations/c1/cancel".to_string())
    );

    // No answer (client without elicitation): the confirm_id comes back as is.
    let set = people();
    let none = Answer(None, AtomicBool::new(false));
    let out = set
        .call_confirming(
            "factory-workflow-publish",
            args(json!({"note": "QC 2"})),
            &ctx,
            &none,
        )
        .await
        .unwrap();
    assert_eq!(out.structured_content.unwrap()["status"], "pending");
}

#[test]
fn errors_after_a_used_confirmation_say_to_start_again() {
    let outcome = ttm_api::confirmations::ApplyOutcome {
        status: 409,
        body: json!({"code": "conflict", "rule": "draft-stale", "title": "The draft changed"}),
        applied: true,
    };
    let err = ttm_mcp_tools::confirm::apply_error(&outcome);
    assert_eq!(err.rule(), Some("draft-stale"));
    assert!(err.message.contains("refused by the server"));
    let step = ttm_api::confirmations::ApplyOutcome {
        status: 410,
        body: json!({"code": "confirm_expired", "title": "expired"}),
        applied: false,
    };
    assert_eq!(ttm_mcp_tools::confirm::apply_error(&step).code, "confirm_expired");
}

#[tokio::test]
async fn publishing_past_a_rule_state_lists_the_rules_and_acknowledges_them_on_confirm() {
    let set = people();
    let staged = check(&set, "factory-workflow-publish", json!({"note": "QC 2"})).await;
    let md = text(&staged);
    assert!(md.contains("1 Automation rule(s) name states"), "{md}");
    assert!(
        md.contains("| QC → notify (AUR) | qc-notify | trigger.to | qc renamed to qc2 |"),
        "{md}"
    );
    let v = staged.structured_content.unwrap();
    assert_eq!(
        v["summary"],
        "Publish workflow Professional v3 (1 Automation rule(s) stop firing)"
    );
    assert_eq!(v["preview"]["affected_rules"][0]["id"], "r9");
    let sent = set.backend().body("POST /confirmations").unwrap();
    assert_eq!(sent["body"]["acknowledged_rules"], json!(["r9"]), "{sent}");

    // No rule left behind: the body stays as before (no flag), no warning.
    let upgrade = check(&set, "factory-workflow-upgrade", json!({"project": "AUR"})).await;
    assert!(!text(&upgrade).contains("Automation rule"), "{}", text(&upgrade));
    let sent = set.backend().body("POST /confirmations").unwrap();
    assert_eq!(sent["path"], "/factory/projects/p1/workflow-upgrade");
    assert!(sent["body"].get("acknowledged_rules").is_none(), "{sent}");
}
