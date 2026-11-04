//! What people see: every command through `ttm-ui`, without colour at 80 columns (`NO_COLOR`, `TTM_COLUMNS=80`)
//! and with `--plain`. Snapshots live in `tests/snapshots/`; review changes with `cargo insta review`.

mod common;

use common::*;
use std::path::Path;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Runs `ttm` in `root` and returns (stdout, stderr), normalized.
async fn run(root: &Path, host: &str, args: &[&str], envs: &[(&str, &str)]) -> (String, String, i32) {
    let out = ttm_env(root, root, host, args, None, envs).await;
    (
        human(&out, root, host),
        human_err(&out, root, host),
        out.status.code().unwrap_or(-1),
    )
}

async fn signed_in() -> (MockServer, tempfile::TempDir) {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let out = ttm(root.path(), &server.uri(), &["login", "--token", "ttm_pat_good"]).await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    (server, root)
}

const PLAIN: &[(&str, &str)] = &[("TTM_PLAIN", "1")];

#[tokio::test]
async fn login_whoami_logout() {
    let server = server().await;
    Mock::given(method("POST"))
        .and(path("/api/v1/oauth/revoke"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();
    let out = ttm_env(
        root.path(),
        root.path(),
        &host,
        &["login", "--token", "-"],
        Some("ttm_pat_good\n"),
        &[],
    )
    .await;
    let text = human(&out, root.path(), &host);
    assert!(text.starts_with("✓ Signed in as An <an@example.com>"), "{text}");
    assert!(text.trim_end().ends_with("Next: ttm project init"), "{text}");
    insta::assert_snapshot!("login", text);

    let (text, _, _) = run(root.path(), &host, &["whoami"], &[]).await;
    assert!(text.starts_with("An <an@example.com>"), "{text}");
    assert!(text.contains("Next: ttm workspace switch <name>"), "{text}");
    insta::assert_snapshot!("whoami", text);
    let (text, _, _) = run(root.path(), &host, &["whoami"], PLAIN).await;
    assert!(text.contains("user: An"), "{text}");
    insta::assert_snapshot!("whoami_plain", text);

    let (text, _, _) = run(root.path(), &host, &["logout"], &[]).await;
    assert!(text.starts_with("✓ Signed out of [HOST]"), "{text}");
    insta::assert_snapshot!("logout", text);
}

#[tokio::test]
async fn errors_follow_the_template() {
    let server = server().await;
    let root = tempfile::tempdir().unwrap();
    let host = server.uri();
    let (out, err, code) = run(root.path(), &host, &["whoami"], &[]).await;
    assert_eq!((out.as_str(), code), ("", 1));
    assert!(err.starts_with("✕ not signed in"), "{err}");
    insta::assert_snapshot!("error_not_signed_in", err);
    let (_, err, _) = run(root.path(), &host, &["whoami"], PLAIN).await;
    assert!(err.starts_with("ERROR: not signed in"), "{err}");
}

#[tokio::test]
async fn status_and_doctor() {
    let (server, root) = signed_in().await;
    let host = server.uri();
    let (text, _, _) = run(root.path(), &host, &["status"], &[]).await;
    assert!(text.starts_with("✓ Signed in to [HOST]"), "{text}");
    assert!(text.contains("token  ✓ ttm_pat_"), "{text}");
    insta::assert_snapshot!("status", text);
    let (text, _, _) = run(root.path(), &host, &["status"], PLAIN).await;
    assert!(text.contains("token: ok ttm_pat_"), "{text}");
    insta::assert_snapshot!("status_plain", text);

    // Doctor depends on the tools installed on this machine: check the shape, not a snapshot.
    let (text, _, _) = run(root.path(), &host, &["doctor"], &[]).await;
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[0].contains("check") || lines[0].contains("passed") || lines[0].contains("warning"),
        "{text}"
    );
    assert!(lines[1].starts_with("CHECK"), "{text}");
    let host_row = |text: &str| {
        text.lines()
            .find(|l| l.starts_with("host "))
            .unwrap_or_default()
            .to_string()
    };
    assert!(host_row(&text).contains(" ✓ "), "{text}");
    let (text, _, _) = run(root.path(), &host, &["doctor"], PLAIN).await;
    assert!(host_row(&text).contains(" ok "), "{text}");
    assert!(!text.contains('─'), "plain has no rules: {text}");
}

#[tokio::test]
async fn workspace_list_switch_current() {
    let (server, root) = signed_in().await;
    let host = server.uri();
    let (text, _, _) = run(root.path(), &host, &["workspace", "list"], &[]).await;
    assert!(text.starts_with("2 workspace(s), none chosen"), "{text}");
    insta::assert_snapshot!("workspace_list_none", text);
    let (text, _, _) = run(root.path(), &host, &["workspace", "switch", "aurora"], &[]).await;
    assert!(text.starts_with("✓ Current workspace: Team Aurora"), "{text}");
    insta::assert_snapshot!("workspace_switch", text);
    let (text, _, _) = run(root.path(), &host, &["workspace", "list"], &[]).await;
    assert!(text.contains("● "), "{text}");
    insta::assert_snapshot!("workspace_list", text);
    let (text, _, _) = run(root.path(), &host, &["workspace", "list"], PLAIN).await;
    assert!(text.contains("* "), "{text}");
    insta::assert_snapshot!("workspace_list_plain", text);
    let (text, _, _) = run(root.path(), &host, &["workspace", "current"], &[]).await;
    assert!(text.starts_with("Team Aurora"), "{text}");
    insta::assert_snapshot!("workspace_current", text);
}

#[tokio::test]
async fn token_pages_and_mcp_dry_run() {
    let (server, root) = signed_in().await;
    let host = server.uri();
    let (text, err, _) = run(root.path(), &host, &["token", "list"], &[]).await;
    assert!(text.starts_with("› Tokens are managed on the web"), "{text}");
    assert!(err.contains("Open this URL in your browser:"), "{err}");
    insta::assert_snapshot!("token_list", text);

    std::fs::create_dir_all(root.path().join(".cursor")).unwrap();
    std::fs::write(
        root.path().join(".cursor/mcp.json"),
        "{\n  \"mcpServers\": {}\n}\n",
    )
    .unwrap();
    let (text, _, _) = run(
        root.path(),
        &host,
        &[
            "mcp",
            "install",
            "--client",
            "cursor",
            "--command",
            "ttm-mcp",
            "--dry-run",
        ],
        &[],
    )
    .await;
    assert!(
        text.starts_with("› Dry run for Cursor: nothing written"),
        "{text}"
    );
    assert!(text.contains("+++ b/[ROOT]/.cursor/mcp.json"), "{text}");
    assert!(
        text.trim_end().ends_with("Next: ttm mcp install --client cursor"),
        "{text}"
    );
    insta::assert_snapshot!("mcp_install_dry_run", text);
}

/// Notee endpoints with Vietnamese, CJK and emoji content.
async fn notee(server: &MockServer) {
    use serde_json::json;
    let ok = |body: serde_json::Value| ResponseTemplate::new(200).set_body_json(body);
    Mock::given(method("GET"))
        .and(path("/api/v1/search"))
        .respond_with(ok(json!([
            {"id": "pg1", "title": "Kế hoạch quý 4", "icon": "📝", "snippet": "Mục tiêu: ra mắt bản 0.2.0\nvà thêm nhiều thứ khác nữa cho đủ dài để bị cắt"},
            {"id": "pg2", "title": "会议记录", "snippet": "第一次会议"}
        ])))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/resources/pg1/summary"))
        .respond_with(ok(
            json!({"id": "pg1", "kind": "page", "title": "Kế hoạch quý 4"}),
        ))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pages/pg1/markdown"))
        .respond_with(ok(json!({"id": "pg1", "title": "Kế hoạch quý 4", "markdown": "# Kế hoạch quý 4\n\n- [ ] Ra mắt\n", "truncated": false})))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/databases/db1/query"))
        .respond_with(ok(json!({
            "columns": [{"name": "Name"}, {"name": "Status"}, {"name": "Tags"}],
            "rows": [
                {"id": "r1", "values": {"Name": "Đăng nhập bằng Google cho người dùng mới", "Status": "Done", "Tags": ["auth", "web"]}},
                {"id": "r2", "values": {"Name": "登录页面", "Status": null, "Tags": []}}
            ],
            "next_cursor": "c2"
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/resources/pg1/comments"))
        .respond_with(ok(json!([
            {"id": "t1", "anchor": {"kind": "text"}, "quote": {"exact": "Ra mắt"}, "resolved_at": null,
             "comments": [{"id": "c1", "author": {"id": "u1", "name": "An"}, "body": {"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Ngày nào?"}]}]}}]},
            {"id": "t2", "anchor": {"kind": "page"}, "resolved_at": "2026-11-01T00:00:00Z",
             "comments": [{"id": "c2", "author": {"id": "u2", "name": "Lan"}, "body": {"type": "doc", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Xong"}]}]}}]}
        ])))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/pages/pg1/suggestions"))
        .respond_with(ok(json!([
            {"id": "s1", "author": {"id": "u1", "name": "An"}, "via_agent": "claude", "status": "pending",
             "changes": [{"id": "ch1", "kind": "replace", "anchor": {}, "before_nfm": "Ra mắt", "after_nfm": "Ra mắt 0.2.0"}]}
        ])))
        .mount(server)
        .await;
}

#[tokio::test]
async fn notes_search_get_query_comments_suggestions() {
    let (server, root) = signed_in().await;
    notee(&server).await;
    let host = server.uri();

    let (text, _, _) = run(root.path(), &host, &["notes", "search", "kế hoạch"], &[]).await;
    assert!(text.starts_with("2 page(s) match"), "{text}");
    assert!(text.contains('…'), "long snippets are cut: {text}");
    assert!(text.trim_end().ends_with("Next: ttm notes get pg1"), "{text}");
    insta::assert_snapshot!("notes_search", text);
    let (text, _, _) = run(root.path(), &host, &["notes", "search", "kế hoạch"], PLAIN).await;
    assert!(!text.contains("📝"), "plain has no emoji: {text}");
    insta::assert_snapshot!("notes_search_plain", text);

    // Markdown goes out untouched, so it can be piped.
    let (text, err, _) = run(root.path(), &host, &["notes", "get", "pg1"], &[]).await;
    assert_eq!(text, "# Kế hoạch quý 4\n\n- [ ] Ra mắt\n");
    assert_eq!(err, "", "no meta line when not at a terminal");

    let (text, _, _) = run(
        root.path(),
        &host,
        &["notes", "query", "db1", "--sql", "SELECT * FROM rows"],
        &[],
    )
    .await;
    assert!(text.contains("NAME") && text.contains("auth, web"), "{text}");
    assert!(text.contains("2 row(s) · more rows"), "{text}");
    assert!(
        text.contains("Next: ttm notes query db1 --sql 'SELECT * FROM rows' --cursor c2"),
        "{text}"
    );
    insta::assert_snapshot!("notes_query", text);

    let (text, _, _) = run(root.path(), &host, &["notes", "comments", "pg1", "--all"], &[]).await;
    assert!(text.starts_with("2 thread(s)"), "{text}");
    assert!(
        text.contains("> Ra mắt") && text.contains("👤 An: Ngày nào?"),
        "{text}"
    );
    assert!(text.contains("Thread t2 ✓ resolved"), "{text}");
    insta::assert_snapshot!("notes_comments", text);
    let (text, _, _) = run(root.path(), &host, &["notes", "comments", "pg1", "--all"], PLAIN).await;
    assert!(
        text.contains("[person] An: Ngày nào?") && text.contains("Thread t2 done resolved"),
        "{text}"
    );

    let (text, _, _) = run(root.path(), &host, &["notes", "suggestions", "pg1"], &[]).await;
    assert!(text.starts_with("1 suggestion(s)"), "{text}");
    assert!(text.contains("🤖 An via claude"), "{text}");
    assert!(text.contains("-Ra mắt\n+Ra mắt 0.2.0"), "{text}");
    insta::assert_snapshot!("notes_suggestions", text);
}
