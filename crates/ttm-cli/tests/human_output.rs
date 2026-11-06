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

/// The `profile` row of `ttm status` holds the config file path, cut at 80 columns where the temporary root's length
/// decides: replaced by `[CONFIG]` so the snapshot is the same on every machine.
fn config_path_out(text: &str) -> String {
    text.lines()
        .map(|l| match l.find("profile") {
            Some(i) if l[..i].trim().is_empty() && l.contains(" (") => {
                format!("{}[CONFIG])", &l[..=l.find(" (").unwrap() + 1])
            }
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if text.ends_with('\n') { "\n" } else { "" }
}

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
    insta::assert_snapshot!("status", config_path_out(&text));
    let (text, _, _) = run(root.path(), &host, &["status"], PLAIN).await;
    assert!(text.contains("token: ok ttm_pat_"), "{text}");
    insta::assert_snapshot!("status_plain", config_path_out(&text));

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

/// A repository linked to project `p1` (key `AUR`) at rev 5, with an in-sync plan.
fn linked_repo(root: &Path, host: &str) -> std::path::PathBuf {
    let repo = root.join("repo");
    std::fs::create_dir_all(repo.join(".ttm/plan")).unwrap();
    std::fs::create_dir_all(repo.join(".ttm/base")).unwrap();
    std::fs::write(
        repo.join(".ttm/project.toml"),
        format!("host = \"{host}\"\nproject_id = \"p1\"\nkey = \"AUR\"\nsynced_rev = 5\n"),
    )
    .unwrap();
    std::fs::write(repo.join(".ttm/base/index.md"), "# A\n").unwrap();
    std::fs::write(repo.join(".ttm/plan/index.md"), "# A\n").unwrap();
    repo
}

async fn factory(server: &MockServer) {
    use serde_json::json;
    let ok = |body: serde_json::Value| ResponseTemplate::new(200).set_body_json(body);
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/view"))
        .respond_with(ok(json!({"tasks": [
            {"id": "AUR-12", "key": "AUR-12", "title": "Đăng nhập bằng Google cho người dùng mới, kèm xác minh email",
             "state": "Doing", "category": "in-progress", "role": "DEV", "estimate": 3,
             "assigneeMember": {"kind": "agent", "id": "a1", "name": "Mei", "ownerMembership": {"tier": "mei-er"}}},
            {"id": "AUR-13", "key": "AUR-13", "title": "登录页面", "state": "QC", "category": "qc", "role": "QA",
             "assigneeMember": {"kind": "user", "id": "u1", "name": "An"}},
            {"id": "AUR-14", "key": "AUR-14", "title": "Release 🚀", "state": "Approval", "category": "approval"}
        ]})))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/tasks/AUR-12"))
        .respond_with(ok(json!({
            "id": "AUR-12", "key": "AUR-12", "title": "Login with Google", "state": "Doing", "category": "in-progress",
            "role": "DEV", "estimate": 3, "description": "Use the Google button.",
            "assigneeMember": {"kind": "agent", "id": "a1", "name": "Mei"},
            "createdBy": {"kind": "user", "id": "u1", "name": "An", "membership": {"tier": "sponsor"}},
            "sprint": {"name": "Sprint 4"}, "labels": ["auth", "web"],
            "acceptanceCriteria": [{"text": "Button shows", "done": true}, "Email is verified"],
            "comments": [{"author": {"kind": "user", "id": "u1", "name": "An"}, "body": "Ngày nào xong?"}]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/export"))
        .respond_with(ok(json!({"rev": 6, "project_id": "p1", "key": "AUR", "files": {"plan/index.md": "# A\n", "plan/tasks/AUR-12.md": "x\n"}})))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/runs/r1/events"))
        .respond_with(ok(json!([
            {"id": 1, "kind": "thought", "message": "Reading the login page", "at": "2026-11-05T10:00:01Z"},
            {"id": 2, "kind": "action", "message": "edited src/login.rs", "at": "2026-11-05T10:00:02Z"},
            {"id": 3, "kind": "progress", "message": "tests", "data": {"percent": 50}, "at": "2026-11-05T10:00:03Z"},
            {"id": 4, "kind": "error", "message": "cargo test failed", "at": "2026-11-05T10:00:04Z"}
        ])))
        .mount(server)
        .await;
}

#[tokio::test]
async fn factory_tasks_sync_project_and_logs() {
    let (server, root) = signed_in().await;
    factory(&server).await;
    let host = server.uri();
    let repo = linked_repo(root.path(), &host);
    let run_in = |args: &'static [&'static str], envs: &'static [(&'static str, &'static str)]| {
        let (root, repo, host) = (root.path().to_path_buf(), repo.clone(), host.clone());
        async move {
            let out = ttm_env(&root, &repo, &host, args, None, envs).await;
            (
                human(&out, &root, &host),
                human_err(&out, &root, &host),
                out.status.code(),
            )
        }
    };

    let (text, _, _) = run_in(&["task", "list"], &[]).await;
    assert!(text.starts_with("AUR · 3 task(s)"), "{text}");
    assert!(
        text.contains("●  AUR-12") && text.contains("🤖 Mei ✦ Mei-er"),
        "{text}"
    );
    assert!(text.contains("◐  AUR-14"), "{text}");
    insta::assert_snapshot!("task_list", text);
    let (text, _, _) = run_in(&["task", "list"], PLAIN).await;
    assert!(
        text.contains("doing   AUR-12") && text.contains("[agent] Mei [Mei-er]"),
        "{text}"
    );
    insta::assert_snapshot!("task_list_plain", text);

    let (text, _, _) = run_in(&["task", "show", "AUR-12"], &[]).await;
    assert!(text.starts_with("╭─ AUR-12 · Login with Google"), "{text}");
    assert!(text.contains("delegator  👤 An ✦✦ Sponsor"), "{text}");
    assert!(
        text.contains("✓ Button shows") && text.contains("○ Email is verified"),
        "{text}"
    );
    assert!(text.trim_end().ends_with("Next: ttm agent run AUR-12"), "{text}");
    insta::assert_snapshot!("task_show", text);

    let (text, _, code) = run_in(&["sync", "--check"], &[]).await;
    assert_eq!(code, Some(1));
    assert!(
        text.starts_with("! Plan out of sync: local rev 5, server rev 6"),
        "{text}"
    );
    assert!(text.contains("server  tasks/AUR-12.md"), "{text}");
    insta::assert_snapshot!("sync_check", text);

    let (text, _, _) = run_in(&["project", "status"], &[]).await;
    assert!(text.starts_with("AUR · linked at [ROOT]/repo"), "{text}");
    insta::assert_snapshot!("project_status", text);

    let (text, _, _) = run_in(&["agent", "logs", "r1"], &[]).await;
    assert_eq!(
        text,
        "10:00:01 · Reading the login page\n10:00:02 › edited src/login.rs\n10:00:03  50% tests\n10:00:04 ✕ cargo test failed\n"
    );
    let (text, _, _) = run_in(&["agent", "logs", "r1", "--json"], &[]).await;
    assert_eq!(text.lines().count(), 4, "NDJSON: {text}");
    assert!(
        text.lines()
            .all(|l| serde_json::from_str::<serde_json::Value>(l).is_ok()),
        "{text}"
    );
}

#[tokio::test]
async fn hooks_always_print_plain() {
    let (server, root) = signed_in().await;
    let host = server.uri();
    let repo = linked_repo(root.path(), &host);
    // serde_json escapes the path (Windows paths have backslashes, which a hand-written JSON string would not).
    let input = serde_json::json!({"tool_input": {"command": "git push --no-verify"}, "cwd": repo.display().to_string()})
        .to_string();
    let out = ttm_env(
        root.path(),
        &repo,
        &host,
        &["hook", "pre-bash"],
        Some(&input),
        &[("CLICOLOR_FORCE", "1")],
    )
    .await;
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.starts_with("Blocked by project rule `") && err.contains("no_verify"),
        "{err}"
    );
    assert!(!err.contains('\u{1b}'), "never coloured: {err}");
}
