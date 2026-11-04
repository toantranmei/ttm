//! API v2 clients (S2, S3, S5, S6, S7) against a mock server: paths, methods, bodies, the zip upload headers, and
//! answers with or without the new fields.

use serde_json::json;
use std::time::Duration;
use ttm_api::Client;
use ttm_api::factory::ChangeRequestOutcome;
use wiremock::matchers::{body_json, body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn client(server: &MockServer) -> Client {
    Client::new(format!("{}/api/v1", server.uri()), Some("ttm_pat_x".into()))
        .unwrap()
        .with_retry_base(Duration::ZERO)
}

fn ok(body: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

#[tokio::test]
async fn me_with_and_without_membership() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/me"))
        .respond_with(ok(json!({
            "user": {"id": "u1", "email": "a@b.c", "name": "An",
                     "membership": {"tier": "mei-er", "status": "active", "since": "2025-11-05T00:00:00Z", "until": "2026-12-05T00:00:00Z", "badge": "✦"}},
            "workspaces": [{"id": "w1", "name": "A", "kind": "team", "role": "owner", "owner_membership": {"tier": "sponsor"}}]
        })))
        .mount(&server)
        .await;
    let me = client(&server).await.me().await.unwrap();
    let m = me.user.membership.unwrap();
    assert_eq!(m.tier(), Some(ttm_api::vocab::Tier::Meier));
    assert_eq!(
        me.workspaces[0].owner_membership.as_ref().unwrap().tier(),
        Some(ttm_api::vocab::Tier::Sponsor)
    );
    // Older servers: no membership key at all.
    let old: ttm_api::Me =
        serde_json::from_value(json!({"user": {"id": "u", "email": "e", "name": "n"}})).unwrap();
    assert!(old.user.membership.is_none());
}

#[tokio::test]
async fn agents_and_presets() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/w1/agents"))
        .respond_with(ok(
            json!([{"id": "a1", "name": "DEV · Code", "handle": "dev", "role": "DEV",
            "skills": [{"skill_id": "s1", "slug": "ttm-tdd", "state": "pending"}],
            "profile": {"mcp": [{"server": "local:github-local", "tools": ["*"], "required": true}]}},
            {"id": "a2", "name": "Old"}]),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/workspaces/w1/agents"))
        .and(body_json(json!({"name": "QA", "handle": "qa", "preset": "qa"})))
        .respond_with(ok(json!({"id": "a3", "name": "QA", "handle": "qa"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/api/v1/workspaces/w1/agents/a1/skills"))
        .and(body_json(
            json!({"skills": [{"skill_id": "s1", "pinned_version": 3}]}),
        ))
        .respond_with(ok(json!({"ok": true})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/agent-presets"))
        .and(query_param("template", "balanced"))
        .respond_with(ok(
            json!([{"code": "dev", "name": "DEV · Code", "skills": ["ttm-tdd"]}]),
        ))
        .mount(&server)
        .await;
    let c = client(&server).await;
    let agents = c.agents("w1").await.unwrap();
    assert_eq!(agents[0].label(), "@dev");
    assert!(agents[0].matches("@DEV"));
    assert!(agents[0].mcp()[0].required);
    assert_eq!(agents[1].label(), "Old");
    let input = ttm_api::agents::AgentInput {
        name: Some("QA".into()),
        handle: Some("qa".into()),
        preset: Some("qa".into()),
        ..Default::default()
    };
    assert_eq!(c.create_agent("w1", &input).await.unwrap().id, "a3");
    let pins = [ttm_api::agents::SkillPin {
        skill_id: "s1".into(),
        pinned_version: Some(3),
    }];
    c.put_agent_skills("w1", "a1", &pins).await.unwrap();
    assert_eq!(c.agent_presets(Some("balanced")).await.unwrap()[0].code, "dev");
}

#[tokio::test]
async fn skill_push_sends_a_zip() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/skills/s1/versions"))
        .and(header("content-type", "application/zip"))
        .and(header("x-ttm-source", "ttm"))
        .and(query_param("note", "first"))
        .respond_with(ok(
            json!({"version": 2, "sha256": "ab", "size": 10, "review_status": "pending"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/skills"))
        .and(query_param("workspace_id", "w1"))
        .and(query_param("builtin", "1"))
        .respond_with(ok(
            json!([{"id": "s1", "slug": "ttm-tdd", "name": "TDD", "builtin": true}]),
        ))
        .mount(&server)
        .await;
    let c = client(&server).await;
    let v = c
        .skill_push("s1", b"PK\x03\x04".to_vec(), Some("first"))
        .await
        .unwrap();
    assert_eq!((v.version, v.review_status.as_deref()), (2, Some("pending")));
    assert!(c.skills("w1", None, None, true).await.unwrap()[0].builtin);
}

#[tokio::test]
async fn mcp_servers_connect() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/mcp-servers"))
        .and(query_param("workspace_id", "w1"))
        .respond_with(ok(json!([{"id": "m1", "name": "linear", "url": "https://mcp.linear.app/sse",
            "auth": "delegator", "tools": ["create_issue", {"name": "list_issues"}], "connected": {"workspace": false, "me": true}}])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/mcp-servers/m1/connect"))
        .and(body_json(json!({"for": "me"})))
        .respond_with(ok(json!({"authorize_url": "https://auth.example/authorize?x=1"})))
        .mount(&server)
        .await;
    let c = client(&server).await;
    let list = c.mcp_servers("w1").await.unwrap();
    assert_eq!(list[0].tool_names(), ["create_issue", "list_issues"]);
    assert!(list[0].connected.me);
    assert!(
        c.mcp_connect("m1", "me")
            .await
            .unwrap()
            .starts_with("https://auth.example")
    );
}

#[tokio::test]
async fn workflows_and_automation() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/workflows/wf1/simulate"))
        .and(body_partial_json(
            json!({"task": "AUR-1", "steps": ["qc"], "actor": {"kind": "user"}}),
        ))
        .respond_with(ok(
            json!({"steps": [{"from": "doing", "to": "qc", "allowed": false,
            "blocked": {"code": "approval-required", "message": "needs QA"}}], "end_state": "doing"}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/workflows/wf1/diff"))
        .and(query_param("from", "2"))
        .and(query_param("to", "draft"))
        .respond_with(ok(
            json!({"states": {"added": [{"id": "qc", "name": "QC"}]}, "layout_only": false}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/automation/rules/dry-run"))
        .and(body_json(
            json!({"rule_id": "r1", "project_id": "p1", "task": "AUR-1"}),
        ))
        .respond_with(ok(
            json!({"matched": true, "actions": [{"type": "assign", "will_run": true}]}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/automation/runs"))
        .and(query_param("rule_id", "r1"))
        .and(query_param("limit", "20"))
        .respond_with(ok(json!([{"id": "x", "rule_id": "r1", "status": "ok"}])))
        .mount(&server)
        .await;
    let c = client(&server).await;
    let sim = c
        .workflow_simulate(
            "wf1",
            &ttm_api::workflows::SimulateInput {
                project_id: Some("p1".into()),
                task: Some("AUR-1".into()),
                actor: ttm_api::workflows::SimActor {
                    kind: "user".into(),
                    roles: vec![],
                },
                steps: vec!["qc".into()],
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(sim.steps[0].blocked.as_ref().unwrap().code, "approval-required");
    let diff = c.workflow_diff("wf1", "2", "draft").await.unwrap();
    assert_eq!(ttm_api::workflows::item_label(&diff.states.added[0]), "QC");
    assert!(c.automation_test("r1", "p1", "AUR-1").await.unwrap().matched);
    assert_eq!(c.automation_runs(Some("r1"), None, 20).await.unwrap().len(), 1);
}

#[tokio::test]
async fn confirmations_and_inbox() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/confirmations"))
        .and(body_partial_json(
            json!({"method": "POST", "path": "/factory/workflows/wf1/publish"}),
        ))
        .respond_with(ok(
            json!({"confirm_id": "c1", "expires_at": "2026-11-05T10:10:00Z"}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/confirmations/c1/apply"))
        .respond_with(
            ResponseTemplate::new(409).set_body_json(json!({"code": "conflict", "rule": "confirm_stale"})),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/notifications"))
        .and(query_param("kind", "factory"))
        .and(query_param("unread", "1"))
        .and(query_param("since", "2026-11-05T10:00:00Z"))
        .respond_with(ok(
            json!([{"id": "n1", "kind": "factory", "text": "Skill ttm-tdd v3 waiting for review",
            "created_at": "2026-11-05T10:01:00Z", "data": {"reason": "skill_review"}}]),
        ))
        .mount(&server)
        .await;
    let c = client(&server).await;
    let conf = c
        .confirmation_create(&ttm_api::confirmations::NewConfirmation {
            method: "POST".into(),
            path: "/factory/workflows/wf1/publish".into(),
            body: json!({"note": "v3", "base_version": 2}),
            summary: "Publish v3".into(),
            preview: json!({}),
        })
        .await
        .unwrap();
    assert_eq!(conf.confirm_id, "c1");
    // The replayed status comes back as data, not as an error.
    let (status, body) = c.confirmation_apply("c1").await.unwrap();
    assert_eq!((status, body["rule"].as_str()), (409, Some("confirm_stale")));
    let inbox = c
        .notifications(&ttm_api::inbox::InboxQuery {
            kinds: vec!["factory".into()],
            unread: true,
            since: Some("2026-11-05T10:00:00Z".into()),
        })
        .await
        .unwrap();
    assert_eq!(inbox[0].reason(), "skill_review");
    assert!(inbox[0].unread());
}

#[tokio::test]
async fn change_requests_v1_and_v2_answers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects/p1/change-requests"))
        .and(body_partial_json(json!({"config_base": {"workflow": 3}})))
        .respond_with(ok(json!({"change_request": null, "skipped": [],
            "config_requests": [{"id": "cfg1", "kind": "agent", "target": "dev"}], "conflicts": []})))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects/p2/change-requests"))
        .respond_with(ok(
            json!({"id": "cr1", "status": "pending", "base_rev": 4, "changes": [], "skipped": []}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects/p3/change-requests"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({"conflicts": [
            {"kind": "workflow", "target": "Aurora", "path": "workflow.yaml", "base": "a", "local": "b", "server": "c"}]})))
        .mount(&server)
        .await;
    let c = client(&server).await;
    let files =
        std::collections::BTreeMap::from([("agents/dev.yaml".to_string(), Some("name: DEV\n".to_string()))]);
    let ChangeRequestOutcome::Created(v2) = c
        .create_change_request_v2("p1", 4, &files, Some(&json!({"workflow": 3})))
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(v2.is_empty());
    assert_eq!(v2.config_requests[0].kind, "agent");
    let ChangeRequestOutcome::Created(v1) = c.create_change_request("p2", 4, &files).await.unwrap() else {
        panic!()
    };
    assert_eq!(v1.id, "cr1");
    let ChangeRequestOutcome::Conflicts(conflicts) = c.create_change_request("p3", 4, &files).await.unwrap()
    else {
        panic!()
    };
    assert_eq!(conflicts[0].path.as_deref(), Some("workflow.yaml"));
    assert_eq!(
        ttm_api::factory::bundle_path("agents/dev.yaml"),
        "agents/dev.yaml"
    );
    assert_eq!(ttm_api::factory::bundle_path("tasks/A.md"), "plan/tasks/A.md");
}

#[tokio::test]
async fn templates_install_preview() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/catalog"))
        .and(query_param("kind", "workflow,skill"))
        .respond_with(ok(
            json!({"items": [{"id": "i1", "slug": "web-todo", "kind": "workflow", "name": "Web todo"}]}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/catalog/items/i1/install-preview"))
        .and(body_partial_json(json!({"workspace_id": "w1"})))
        .respond_with(ok(json!({"creates": {"rules": [{"slug": "r"}], "agents": [], "skills": [{"slug": "s"}]}, "runs_code": true})))
        .mount(&server)
        .await;
    let c = client(&server).await;
    assert_eq!(
        c.catalog(&["workflow", "skill"], None).await.unwrap()[0].slug,
        "web-todo"
    );
    let p = c.template_install_preview("i1", "w1", None).await.unwrap();
    assert!(p.runs_code && p.creates.skills.len() == 1);
}

#[test]
fn job_spec_v2_fields_default() {
    let old: ttm_api::factory::JobSpec = serde_json::from_value(json!({
        "run_id": "r1", "task_key": "AUR-1", "branch": "b", "prompt": "p"
    }))
    .unwrap();
    assert!(old.agent.is_none() && old.skills.is_empty() && !old.team_job && old.mcp.remote.is_empty());
    let new: ttm_api::factory::JobSpec = serde_json::from_value(json!({
        "run_id": "r1", "task_key": "AUR-1", "branch": "b", "prompt": "p",
        "agent": {"id": "a1", "handle": "dev", "name": "DEV · Code"}, "team_job": true,
        "git": {"author_name": "DEV · Code", "author_email": "dev+aurora@agents.toantranmei.com",
                "committer_name": "DEV · Code", "committer_email": "dev+aurora@agents.toantranmei.com", "trailers": []},
        "skills": [{"slug": "ttm-tdd", "version": 3, "url": "https://x/s.zip", "sha256": "ab", "size": 10}],
        "mcp": {"token": "ttm_job_x", "toolsets": ["factory"], "remote": [{"name": "linear", "url": "https://x/mcp/gw/linear"}], "local": ["github-local"]},
        "requires": ["skills", "mcp_gateway"]
    }))
    .unwrap();
    assert_eq!(new.agent.unwrap().handle, "dev");
    assert_eq!(new.mcp.local, ["github-local"]);
    assert_eq!(new.requires.len(), 2);
}
