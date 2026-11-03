//! `ttm project link`, `ttm sync`, `ttm hook pre-bash` against a mock server.

mod common;

use common::*;
use serde_json::json;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn link_edit_sync_and_check() {
    let server = server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"id": "p1", "workspaceId": "w1", "name": "Aurora", "code": "AUR", "key": "AUR", "health": "ok",
             "sprint": "", "activeAgents": 0, "rev": 7, "canWrite": true}
        ])))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "p1", "workspace_id": "w1", "key": "AUR", "name": "Aurora", "repo_provider": "github",
            "repo_url": null, "default_branch": "main", "settings": {}, "human_gate": true, "paused": false,
            "workflow_id": null, "owner_id": null, "rev": 7
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/export"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "rev": 7,
            "project_id": "p1", "key": "AUR",
            "files": {"plan/index.md": "# Aurora\n", "plan/tasks/AUR-1.md": "---\nkey: \"AUR-1\"\nstate: \"todo\"\n---\nLogin\n"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects/p1/change-requests"))
        .and(body_partial_json(json!({"base_rev": 7, "files": {"plan/tasks/AUR-1.md": "---\nkey: \"AUR-1\"\nstate: \"doing\"\n---\nLogin\n"}})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "cr1", "status": "pending", "base_rev": 7, "changes": [], "skipped": []})))
        .expect(1)
        .mount(&server)
        .await;

    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let host = server.uri();
    let login = ttm(root.path(), &host, &["login", "--token", "ttm_pat_good"]).await;
    assert!(
        login.status.success(),
        "{}",
        String::from_utf8_lossy(&login.stderr)
    );

    let out = ttm_in(
        root.path(),
        &repo,
        &host,
        &["project", "link", "aur", "--json"],
        None,
    )
    .await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(json(&out)["synced_rev"], 7);
    assert_eq!(
        json(&out)["default_branch"],
        "main",
        "single project read for the branch"
    );
    assert!(repo.join(".ttm/plan/tasks/AUR-1.md").exists());
    assert!(repo.join(".ttm/base/index.md").exists());

    let check = ttm_in(root.path(), &repo, &host, &["sync", "--check"], None).await;
    assert!(check.status.success(), "in sync right after linking");

    std::fs::write(
        repo.join(".ttm/plan/tasks/AUR-1.md"),
        "---\nkey: \"AUR-1\"\nstate: \"doing\"\n---\nLogin\n",
    )
    .unwrap();
    let check = ttm_in(root.path(), &repo, &host, &["sync", "--check"], None).await;
    assert!(!check.status.success(), "local edit is reported");

    let out = ttm_in(root.path(), &repo, &host, &["sync", "--json"], None).await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(json(&out)["change_request"], "cr1");
    let project = std::fs::read_to_string(repo.join(".ttm/project.toml")).unwrap();
    assert!(project.contains("pending_change_request = \"cr1\""), "{project}");
}

#[tokio::test]
async fn empty_change_request_means_up_to_date() {
    let server = server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/export"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "rev": 5, "project_id": "p1", "key": "A", "files": {"plan/index.md": "# A\n"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects/p1/change-requests"))
        .and(body_partial_json(
            json!({"files": {"plan/index.md": "# A edited\n"}}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "", "status": "empty", "base_rev": 5, "changes": [], "skipped": []
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join(".ttm/plan")).unwrap();
    std::fs::create_dir_all(repo.join(".ttm/base")).unwrap();
    std::fs::write(
        repo.join(".ttm/project.toml"),
        format!(
            "host = \"{}\"\nproject_id = \"p1\"\nkey = \"A\"\nsynced_rev = 5\n",
            server.uri()
        ),
    )
    .unwrap();
    std::fs::write(repo.join(".ttm/base/index.md"), "# A\n").unwrap();
    std::fs::write(repo.join(".ttm/plan/index.md"), "# A edited\n").unwrap();
    let host = server.uri();
    ttm(root.path(), &host, &["login", "--token", "ttm_pat_good"]).await;
    let out = ttm_in(root.path(), &repo, &host, &["sync", "--json"], None).await;
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let v = json(&out);
    assert_eq!(v["nothing_to_send"], true);
    assert!(v["change_request"].is_null());
    assert_eq!(
        std::fs::read_to_string(repo.join(".ttm/plan/index.md")).unwrap(),
        "# A\n"
    );
    assert!(repo.join(".ttm/cache/dropped/index.md").exists());
    let check = ttm_in(root.path(), &repo, &host, &["sync", "--check"], None).await;
    assert!(check.status.success());
}

#[tokio::test]
async fn conflicts_stop_the_sync() {
    let server = server().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/factory/projects/p1/export"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"rev": 9, "project_id": "p1", "key": "A", "files": {"plan/tasks/A.md": "server"}}),
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v1/factory/projects/p1/change-requests"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "code": "conflict", "rule": "plan-conflict", "title": "1 field(s) changed both locally and on the server", "status": 409,
            "conflicts": [{"task": "A", "field": "state", "base": "todo", "local": "doing", "server": "done"}]
        })))
        .mount(&server)
        .await;
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join(".ttm/plan/tasks")).unwrap();
    std::fs::create_dir_all(repo.join(".ttm/base/tasks")).unwrap();
    std::fs::write(
        repo.join(".ttm/project.toml"),
        format!(
            "host = \"{}\"\nproject_id = \"p1\"\nkey = \"A\"\nsynced_rev = 8\n",
            server.uri()
        ),
    )
    .unwrap();
    std::fs::write(repo.join(".ttm/base/tasks/A.md"), "base").unwrap();
    std::fs::write(repo.join(".ttm/plan/tasks/A.md"), "local").unwrap();
    let host = server.uri();
    ttm(root.path(), &host, &["login", "--token", "ttm_pat_good"]).await;
    let out = ttm_in(root.path(), &repo, &host, &["sync", "--json"], None).await;
    assert!(!out.status.success());
    assert_eq!(json(&out)["conflicts"][0]["server"], "done");
    assert_eq!(
        std::fs::read_to_string(repo.join(".ttm/plan/tasks/A.md")).unwrap(),
        "local",
        "nothing overwritten"
    );
}

#[tokio::test]
async fn pre_bash_hook_blocks_push_to_main() {
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    std::fs::create_dir_all(repo.join(".ttm")).unwrap();
    std::fs::write(
        repo.join(".ttm/project.toml"),
        "host = \"http://127.0.0.1:9\"\nproject_id = \"p1\"\nkey = \"A\"\n",
    )
    .unwrap();
    let input = json!({"tool_name": "Bash", "tool_input": {"command": "git push origin main"}, "cwd": repo})
        .to_string();
    let out = ttm_in(
        root.path(),
        &repo,
        "http://127.0.0.1:9",
        &["hook", "pre-bash"],
        Some(&input),
    )
    .await;
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no_push_main"));

    let ok = json!({"tool_name": "Bash", "tool_input": {"command": "git status"}, "cwd": repo}).to_string();
    let out = ttm_in(
        root.path(),
        &repo,
        "http://127.0.0.1:9",
        &["hook", "pre-bash"],
        Some(&ok),
    )
    .await;
    assert_eq!(out.status.code(), Some(0));
}
