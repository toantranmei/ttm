//! The v2 commands (S7 Tasks 9–13, 20) against a mock server: feature checks, wizards with flags and `--yes`
//! without a terminal, tables, the Mei-er lines and the as-code sync. Snapshots: `cargo insta review`.

mod common;

use common::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ok(body: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

/// A v2 server, signed in, workspace `w2` current, a repo linked to project `p1` (`AUR`).
async fn setup(features: &[&str], me: Value) -> (MockServer, tempfile::TempDir, PathBuf) {
    let server = server_with(features, me).await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();
    let out = ttm(root.path(), &host, &["login", "--token", "ttm_pat_good"]).await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    ttm(root.path(), &host, &["workspace", "switch", "w2"]).await;
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join(".ttm/plan")).unwrap();
    std::fs::create_dir_all(repo.join(".ttm/base")).unwrap();
    std::fs::write(
        repo.join(".ttm/project.toml"),
        format!("host = \"{host}\"\nproject_id = \"p1\"\nkey = \"AUR\"\nsynced_rev = 5\n"),
    )
    .unwrap();
    std::fs::write(repo.join(".ttm/base/index.md"), "# A\n").unwrap();
    std::fs::write(repo.join(".ttm/plan/index.md"), "# A\n").unwrap();
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1"))
        .respond_with(ok(json!({"id": "p1", "key": "AUR", "name": "Aurora", "workflow_id": "wf1", "workflow_version": 2, "rev": 5})))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects"))
        .respond_with(ok(
            json!([{"id": "p1", "key": "AUR", "name": "Aurora", "rev": 5}]),
        ))
        .mount(&server)
        .await;
    (server, root, repo)
}

async fn run(
    root: &Path,
    cwd: &Path,
    host: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> (String, String, i32) {
    let out = ttm_env(root, cwd, host, args, None, envs).await;
    (
        human(&out, root, host),
        human_err(&out, root, host),
        out.status.code().unwrap_or(-1),
    )
}

fn member_me(tier: Option<&str>) -> Value {
    let mut me = me_body();
    if let Some(t) = tier {
        me["user"]["membership"] = json!({"tier": t, "status": "active", "since": "2025-01-15T00:00:00Z", "until": "2027-01-15T00:00:00Z"});
    }
    me
}

#[tokio::test]
async fn new_commands_check_server_features() {
    let (server, root, repo) = setup(&["notee"], me_body()).await;
    let (_, err, code) = run(root.path(), &repo, &server.uri(), &["workflow", "show"], &[]).await;
    assert_eq!(code, 2);
    assert!(
        err.contains("This server does not support workflow v2 yet"),
        "{err}"
    );
    assert!(err.contains("rule: unsupported_feature"), "{err}");
    let (_, err, code) = run(root.path(), &repo, &server.uri(), &["agent", "list"], &[]).await;
    assert_eq!(code, 2, "{err}");
}

#[tokio::test]
async fn whoami_thanks_members_and_invites_others() {
    let (server, root, repo) = setup(V2_FEATURES, member_me(Some("mei-er"))).await;
    let host = server.uri();
    let (text, _, _) = run(root.path(), &repo, &host, &["whoami"], &[]).await;
    assert!(
        text.contains("✦ Mei-er since 2025-01  Thank you for being a Mei-er. You keep this going."),
        "{text}"
    );
    assert!(!text.contains("Support toantranmei"), "{text}");
    insta::assert_snapshot!("whoami_meier", text);
    let (text, _, _) = run(root.path(), &repo, &host, &["whoami"], &[("TTM_PLAIN", "1")]).await;
    assert!(text.contains("[Mei-er] since 2025-01"), "{text}");
    let out = ttm_env(root.path(), &repo, &host, &["whoami", "--json"], None, &[]).await;
    assert_eq!(json(&out)["meta"]["membership"]["tier"], "mei-er");

    let (server, root, repo) = setup(V2_FEATURES, member_me(None)).await;
    let (text, _, _) = run(root.path(), &repo, &server.uri(), &["whoami"], &[]).await;
    assert!(
        text.contains("Support toantranmei → https://toantranmei.com/mei-er"),
        "{text}"
    );
    let out = ttm_env(
        root.path(),
        &repo,
        &server.uri(),
        &["whoami", "--json"],
        None,
        &[],
    )
    .await;
    assert!(json(&out)["meta"]["membership"].is_null());
}

async fn agents(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/w2/agents"))
        .respond_with(ok(json!([
            {"id": "a1", "name": "DEV · Code", "handle": "dev", "role": "DEV", "model": "sonnet", "status": "active",
             "skills": [{"skill_id": "s1", "slug": "ttm-tdd", "state": "ok"}, {"skill_id": "s2", "slug": "ttm-open-pr", "state": "pending"}],
             "profile": {"mcp": [{"server": "m1", "tools": ["*"]}]}, "owner_membership": {"tier": "sponsor"}},
            {"id": "a2", "name": "QA · Test", "handle": "qa", "role": "QA", "model": "haiku", "status": "paused"}
        ])))
        .mount(server)
        .await;
}

#[tokio::test]
async fn agent_list_show_and_create() {
    let (server, root, repo) = setup(V2_FEATURES, me_body()).await;
    agents(&server).await;
    Mock::given(method("POST"))
        .and(path("/api/v1/workspaces/w2/agents"))
        .and(body_partial_json(
            json!({"name": "Reviewer", "handle": "reviewer", "role": "REVIEW", "model": "opus"}),
        ))
        .respond_with(ok(json!({"id": "a3", "name": "Reviewer", "handle": "reviewer"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/agent-presets"))
        .respond_with(ok(json!([])))
        .mount(&server)
        .await;
    let host = server.uri();
    let (text, _, _) = run(root.path(), &repo, &host, &["agent", "list"], &[]).await;
    assert!(text.starts_with("2 agent(s)"), "{text}");
    assert!(
        text.contains("🤖 DEV · Code ✦✦ Sponsor") && text.contains("◐ ttm-open-pr"),
        "{text}"
    );
    insta::assert_snapshot!("agent_list", text);
    let (text, _, _) = run(root.path(), &repo, &host, &["agent", "show", "@dev"], &[]).await;
    assert!(text.starts_with("╭─ DEV · Code · @dev"), "{text}");
    insta::assert_snapshot!("agent_show", text);

    // Without a terminal: every answer must come from a flag.
    let (_, err, code) = run(root.path(), &repo, &host, &["agent", "create", "--yes"], &[]).await;
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("missing --name"), "{err}");
    let (text, _, code) = run(
        root.path(),
        &repo,
        &host,
        &[
            "agent", "create", "--name", "Reviewer", "--role", "REVIEW", "--model", "opus", "--yes",
        ],
        &[],
    )
    .await;
    assert_eq!(code, 0);
    assert!(text.starts_with("✓ Created agent Reviewer (@reviewer)"), "{text}");
    // A confirmation without --yes and without a terminal is refused.
    let (_, err, code) = run(root.path(), &repo, &host, &["agent", "pause", "dev"], &[]).await;
    assert_eq!(code, 2);
    assert!(err.contains("confirmation_required"), "{err}");
}

#[tokio::test]
async fn workflow_show_validate_publish() {
    let (server, root, repo) = setup(V2_FEATURES, me_body()).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/workflows/wf1"))
        .respond_with(ok(json!({
            "id": "wf1", "name": "Professional", "latest_version": 2, "version": 2, "has_draft": true,
            "definition": {"states": [
                {"id": "ready", "name": "Ready", "category": "ready"},
                {"id": "doing", "name": "Doing", "category": "in-progress", "role": "DEV", "auto_assign": true, "wip": 3},
                {"id": "qc", "name": "QC", "category": "qc", "role": "QA"}],
              "transitions": [{"id": "t1", "from": "ready", "to": "doing"},
                              {"id": "t2", "from": ["doing"], "to": "qc", "guard": {"tests_pass": true}, "rules": [{"slug": "r"}]}]},
            "draft": {"definition": {"states": [], "transitions": []}}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/workflows/wf1/validate"))
        .respond_with(ok(
            json!({"issues": [{"field": "states", "code": "no-done", "message": "needs a done status"}]}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/workflows/wf1/diff"))
        .respond_with(ok(
            json!({"states": {"added": [{"id": "review", "name": "Review"}], "removed": [], "changed": []}}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/workflows/wf1/publish"))
        .and(body_partial_json(
            json!({"note": "Review step", "base_version": 2}),
        ))
        .respond_with(ok(json!({"version": 3, "notified_projects": 2})))
        .expect(1)
        .mount(&server)
        .await;
    let host = server.uri();
    let (text, _, _) = run(root.path(), &repo, &host, &["workflow", "show"], &[]).await;
    assert!(text.starts_with("╭─ Professional · v2"), "{text}");
    assert!(text.contains("⚡1") && text.contains("tests pass"), "{text}");
    insta::assert_snapshot!("workflow_show", text);
    let (text, _, code) = run(root.path(), &repo, &host, &["workflow", "validate"], &[]).await;
    assert_eq!(code, 1);
    assert!(text.contains("✕ no-done states: needs a done status"), "{text}");
    let (text, _, code) = run(
        root.path(),
        &repo,
        &host,
        &["workflow", "publish", "--note", "Review step", "--yes"],
        &[],
    )
    .await;
    assert_eq!(code, 0);
    assert!(
        text.contains("+ status Review") && text.contains("✓ Published Professional v3"),
        "{text}"
    );
}

#[tokio::test]
async fn automation_list_show_test() {
    let (server, root, repo) = setup(V2_FEATURES, me_body()).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/automation/rules"))
        .and(query_param("project_id", "p1"))
        .respond_with(ok(json!([{"id": "r1", "slug": "auto-qa", "name": "Send to QA", "enabled": true,
            "trigger": {"type": "moved", "to": "qc"}, "conditions": {"match": "all", "filters": [{"type": "label", "value": "web"}]},
            "actions": [{"type": "assign", "role": "QA"}], "runs_7d": 4}])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/automation/rules/dry-run"))
        .respond_with(ok(
            json!({"matched": true, "actions": [{"type": "assign", "role": "QA", "will_run": true}]}),
        ))
        .mount(&server)
        .await;
    let host = server.uri();
    let (text, _, _) = run(root.path(), &repo, &host, &["automation", "list"], &[]).await;
    assert!(text.starts_with("AUR · 1 rule(s), 1 on"), "{text}");
    insta::assert_snapshot!("automation_list", text);
    let (text, _, _) = run(root.path(), &repo, &host, &["automation", "show", "auto-qa"], &[]).await;
    assert!(
        text.contains("When moved (to=qc)") && text.contains("└─ label (value=web)"),
        "{text}"
    );
    let (text, _, _) = run(
        root.path(),
        &repo,
        &host,
        &["automation", "test", "auto-qa", "--task", "AUR-1"],
        &[],
    )
    .await;
    assert!(text.starts_with("✓ Send to QA would run on AUR-1"), "{text}");
}

#[tokio::test]
async fn skill_push_creates_then_reports_unchanged() {
    let (server, root, repo) = setup(V2_FEATURES, me_body()).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/skills"))
        .respond_with(ok(json!([])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/skills"))
        .and(body_partial_json(
            json!({"workspace_id": "w2", "slug": "tdd", "name": "TDD"}),
        ))
        .respond_with(ok(json!({"id": "s9", "slug": "tdd", "name": "TDD"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/skills/s9/versions"))
        .and(header("content-type", "application/zip"))
        .respond_with(ok(
            json!({"version": 1, "sha256": "abc", "review_status": "pending"}),
        ))
        .mount(&server)
        .await;
    let dir = root.path().join("tdd");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        "---\nname: TDD\ndescription: Test first.\n---\n# TDD\n",
    )
    .unwrap();
    let host = server.uri();
    let dir_s = dir.display().to_string();
    let out = ttm_env(
        root.path(),
        &repo,
        &host,
        &["skill", "push", &dir_s, "--yes"],
        None,
        &[],
    )
    .await;
    let text = human(&out, root.path(), &host);
    assert!(text.contains("✓ tdd v1 pending review"), "{text}");
    assert!(text.contains("Next: ttm skill review approve tdd@1"), "{text}");
    // A broken skill never reaches the server.
    std::fs::write(dir.join("SKILL.md"), "no front matter").unwrap();
    let out = ttm_env(
        root.path(),
        &repo,
        &host,
        &["skill", "push", &dir_s, "--yes"],
        None,
        &[],
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("front_matter"));
}

#[tokio::test]
async fn mcp_server_add_reads_the_secret_from_the_environment() {
    let (server, root, repo) = setup(V2_FEATURES, me_body()).await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/mcp-servers"))
        .and(body_partial_json(json!({"name": "linear", "auth": "workspace",
            "static_secret": {"header": "Authorization", "value": "Bearer s3cret"}})))
        .respond_with(ok(json!({"id": "m1", "name": "linear", "url": "https://mcp.linear.app/mcp", "tools": ["create_issue"]})))
        .expect(1)
        .mount(&server)
        .await;
    let host = server.uri();
    let args = [
        "mcp-server",
        "add",
        "--name",
        "linear",
        "--url",
        "https://mcp.linear.app/mcp",
        "--auth",
        "workspace",
        "--secret-header",
        "Authorization",
        "--secret-env",
        "LINEAR_SECRET",
        "--yes",
    ];
    let (text, _, code) = run(
        root.path(),
        &repo,
        &host,
        &args,
        &[("LINEAR_SECRET", "Bearer s3cret")],
    )
    .await;
    assert_eq!(code, 0);
    assert!(
        text.starts_with("✓ Added MCP server linear") && !text.contains("s3cret"),
        "{text}"
    );
    let (_, err, code) = run(root.path(), &repo, &host, &args, &[]).await;
    assert_eq!(code, 2, "an unset variable is a missing flag: {err}");
}

#[tokio::test]
async fn cr_inbox_template_members() {
    let (server, root, repo) = setup(V2_FEATURES, me_body()).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/change-requests"))
        .respond_with(ok(json!([{"id": "cr1-long-id", "status": "pending", "created_at": "2026-11-01T00:00:00Z", "changes": [{"key": "AUR-1"}]}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/workspaces/w2/config-requests"))
        .respond_with(ok(json!([{"id": "cfg1", "status": "pending", "kind": "agent", "target": "dev", "created_at": "2026-11-02T00:00:00Z",
            "author": {"kind": "user", "id": "u1", "name": "An", "membership": {"tier": "mei-er"}}}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/notifications"))
        .respond_with(ok(
            json!([{"id": "n1", "kind": "factory", "text": "Skill ttm-tdd v3 waiting for review",
            "created_at": "2026-11-05T14:02:00Z", "data": {"reason": "skill_review"}, "url": "https://x/f"}]),
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/catalog"))
        .respond_with(ok(json!([{"id": "i1", "slug": "web-todo", "kind": "workflow", "name": "Web todo", "tagline": "Ship a todo app", "price": 0}])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/workspaces/w2/members"))
        .respond_with(ok(json!([{"user": {"id": "u1", "name": "An", "email": "an@example.com"}, "role": "owner", "membership": {"tier": "sponsor"}},
            {"user": {"id": "u2", "name": "Bình", "email": "binh@example.com"}, "role": "member"}])))
        .mount(&server)
        .await;
    let host = server.uri();
    let (text, _, _) = run(root.path(), &repo, &host, &["cr", "list"], &[]).await;
    assert!(text.starts_with("AUR · 2 change request(s)"), "{text}");
    assert!(text.contains("👤 An ✦ Mei-er"), "{text}");
    insta::assert_snapshot!("cr_list", text);
    let (text, _, _) = run(root.path(), &repo, &host, &["inbox"], &[]).await;
    assert!(
        text.starts_with("1 notification(s), 1 unread") && text.contains("skill_review"),
        "{text}"
    );
    let (text, _, _) = run(root.path(), &repo, &host, &["template", "list"], &[]).await;
    assert!(text.contains("web-todo") && text.contains("free"), "{text}");
    let (text, _, _) = run(root.path(), &repo, &host, &["workspace", "members"], &[]).await;
    assert!(
        text.contains("👤 An ✦✦ Sponsor") && text.contains("👤 Bình"),
        "{text}"
    );
    insta::assert_snapshot!("workspace_members", text);
    let (text, _, _) = run(
        root.path(),
        &repo,
        &host,
        &["workspace", "members"],
        &[("TTM_PLAIN", "1")],
    )
    .await;
    assert!(text.contains("[person] An [Sponsor]"), "{text}");
}

#[tokio::test]
async fn sync_checks_as_code_files_and_lists_config_requests() {
    let (server, root, repo) = setup(V2_FEATURES, me_body()).await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/export"))
        .and(query_param("include", "config,skills"))
        .respond_with(ok(
            json!({"rev": 5, "project_id": "p1", "key": "AUR", "config_base": {"agents": 1},
            "files": {"plan/index.md": "# A\n", "agents/dev.yaml": "name: DEV\n"}}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects/p1/change-requests"))
        .and(body_partial_json(json!({"config_base": {"agents": 1}, "files": {"agents/dev.yaml": "name: DEV\nmodel: opus\n"}})))
        .respond_with(ok(json!({"change_request": null, "config_requests": [{"id": "cfg1", "kind": "agent", "target": "dev"}], "skipped": [], "conflicts": []})))
        .mount(&server)
        .await;
    std::fs::create_dir_all(repo.join(".ttm/base-config/agents")).unwrap();
    std::fs::create_dir_all(repo.join(".ttm/agents")).unwrap();
    std::fs::write(repo.join(".ttm/base-config/agents/dev.yaml"), "name: DEV\n").unwrap();
    let host = server.uri();
    // A secret in an agent file stops the sync.
    std::fs::write(
        repo.join(".ttm/agents/dev.yaml"),
        "name: DEV\ngithub_token: ghp_x\n",
    )
    .unwrap();
    let (_, err, code) = run(root.path(), &repo, &host, &["sync"], &[]).await;
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("secret_in_file agents/dev.yaml:2"), "{err}");
    std::fs::write(repo.join(".ttm/agents/dev.yaml"), "name: DEV\nmodel: opus\n").unwrap();
    let (text, err, code) = run(root.path(), &repo, &host, &["sync"], &[]).await;
    assert_eq!(code, 0, "{err}");
    assert!(
        text.contains("1 configuration request(s) created") && text.contains("agent  dev"),
        "{text}"
    );
    assert!(text.contains("Next: ttm cr show cfg1"), "{text}");
    insta::assert_snapshot!("sync_config_requests", text);
}

#[tokio::test]
async fn project_init_without_a_terminal_works_like_0_1_6() {
    let (server, root, _) = setup(&["notee"], me_body()).await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects"))
        .and(body_partial_json(json!({"key": "WEB", "name": "web-todo"})))
        .respond_with(ok(
            json!({"id": "p2", "key": "WEB", "name": "web-todo", "rev": 1}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p2/export"))
        .respond_with(ok(json!({"rev": 1, "files": {"plan/index.md": "# W\n"}})))
        .mount(&server)
        .await;
    let repo = root.path().join("web-todo");
    std::fs::create_dir_all(&repo).unwrap();
    let host = server.uri();
    let (text, err, code) = run(
        root.path(),
        &repo,
        &host,
        &["project", "init", "--key", "web"],
        &[],
    )
    .await;
    assert_eq!(code, 0, "{err}");
    assert!(text.starts_with("✓ Created project WEB (web-todo)"), "{text}");
    // v2 flags on a server without workflow v2: refused before any call.
    let (_, err, code) = run(
        root.path(),
        &repo,
        &host,
        &[
            "project",
            "init",
            "--key",
            "WEB",
            "--workflow",
            "balanced",
            "--yes",
        ],
        &[],
    )
    .await;
    assert_eq!(code, 2, "{err}");
}
