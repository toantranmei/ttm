//! Skills of a job: installed from a mock server, checked by SHA-256, excluded from git, cleaned up, cached.

#![cfg(unix)]

use tokio::sync::mpsc;
use ttm_api::factory::{JobSkill, JobSpec};
use ttm_core::skill_zip::{self, BundleFile};
use ttm_core::worktree::git;
use ttm_runner::skills;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn bundle() -> (Vec<u8>, String) {
    skill_zip::pack(&[BundleFile {
        path: "SKILL.md".into(),
        bytes: b"---\nname: TDD\ndescription: Tests first.\n---\n# TDD\n".to_vec(),
        executable: false,
    }])
}

fn job(url: String, sha: String, size: u64) -> JobSpec {
    let mut job: JobSpec = serde_json::from_value(serde_json::json!({
        "run_id": "r1", "task_key": "A-1", "branch": "a-1", "prompt": "p"
    }))
    .unwrap();
    job.skills = vec![JobSkill {
        slug: "tdd".into(),
        version: 3,
        url,
        sha256: sha,
        size,
    }];
    job
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    git(dir.path(), &["init", "-q"]).unwrap();
    dir
}

#[tokio::test]
async fn installs_excludes_cleans_and_caches() {
    let server = MockServer::start().await;
    let (zip, sha) = bundle();
    Mock::given(method("GET"))
        .and(path("/s.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip.clone()))
        .expect(1)
        .mount(&server)
        .await;
    let repo = repo();
    let cache = tempfile::tempdir().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let http = reqwest::Client::new();
    let j = job(format!("{}/s.zip", server.uri()), sha.clone(), zip.len() as u64);
    let mut installed = skills::install(&http, &j, repo.path(), cache.path(), &tx)
        .await
        .unwrap();
    let dir = repo.path().join(".claude/skills/ttm-tdd");
    assert!(dir.join("SKILL.md").exists());
    assert_eq!(installed.names, ["ttm-tdd v3"]);
    let exclude = std::fs::read_to_string(repo.path().join(".git/info/exclude")).unwrap();
    assert_eq!(exclude.matches(skills::EXCLUDE_LINE).count(), 1);
    assert!(rx.try_recv().unwrap().message.starts_with("skills: ttm-tdd v3"));
    installed.cleanup();
    assert!(!dir.exists());
    // Second install: the zip comes from the cache (the mock expects one request) and exclude is not repeated.
    let installed = skills::install(&http, &j, repo.path(), cache.path(), &tx)
        .await
        .unwrap();
    assert!(dir.exists());
    let exclude = std::fs::read_to_string(repo.path().join(".git/info/exclude")).unwrap();
    assert_eq!(exclude.matches(skills::EXCLUDE_LINE).count(), 1);
    drop(installed);
    assert!(!dir.exists(), "cleaned on drop");
}

#[tokio::test]
async fn a_wrong_hash_refuses_the_job() {
    let server = MockServer::start().await;
    let (zip, _) = bundle();
    Mock::given(method("GET"))
        .and(path("/s.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip.clone()))
        .mount(&server)
        .await;
    let repo = repo();
    let cache = tempfile::tempdir().unwrap();
    let (tx, _rx) = mpsc::unbounded_channel();
    let j = job(
        format!("{}/s.zip", server.uri()),
        "00".repeat(32),
        zip.len() as u64,
    );
    let err = skills::install(&reqwest::Client::new(), &j, repo.path(), cache.path(), &tx)
        .await
        .unwrap_err();
    assert_eq!(err.code(), skills::SKILL_HASH_MISMATCH);
    assert!(!repo.path().join(".claude/skills/ttm-tdd").exists());
}

#[tokio::test]
async fn unsafe_zips_are_refused() {
    let server = MockServer::start().await;
    let (zip, sha) = skill_zip::pack(&[BundleFile {
        path: "../evil".into(),
        bytes: b"x".to_vec(),
        executable: false,
    }]);
    Mock::given(method("GET"))
        .and(path("/s.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip.clone()))
        .mount(&server)
        .await;
    let repo = repo();
    let cache = tempfile::tempdir().unwrap();
    let (tx, _rx) = mpsc::unbounded_channel();
    let j = job(format!("{}/s.zip", server.uri()), sha, zip.len() as u64);
    let err = skills::install(&reqwest::Client::new(), &j, repo.path(), cache.path(), &tx)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("path_unsafe"), "{err}");
    assert!(!repo.path().parent().unwrap().join("evil").exists());
}

#[tokio::test]
async fn symlinked_skill_dirs_are_refused() {
    let outside = tempfile::tempdir().unwrap();
    let repo = repo();
    std::os::unix::fs::symlink(outside.path(), repo.path().join(".claude")).unwrap();
    assert!(skills::inside_worktree(repo.path(), ".claude/skills/ttm-tdd").is_err());
    let server = MockServer::start().await;
    let (zip, sha) = skill_zip::pack(&[BundleFile {
        path: "SKILL.md".into(),
        bytes: b"---\nname: TDD\ndescription: Tests first.\n---\n".to_vec(),
        executable: false,
    }]);
    Mock::given(method("GET"))
        .and(path("/s.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip.clone()))
        .mount(&server)
        .await;
    let cache = tempfile::tempdir().unwrap();
    let (tx, _rx) = mpsc::unbounded_channel();
    let j = job(format!("{}/s.zip", server.uri()), sha, zip.len() as u64);
    assert!(
        skills::install(&reqwest::Client::new(), &j, repo.path(), cache.path(), &tx)
            .await
            .is_err()
    );
    assert!(
        std::fs::read_dir(outside.path()).unwrap().next().is_none(),
        "nothing written outside"
    );
    // A plain directory is fine.
    let clean = crate::repo();
    assert!(skills::inside_worktree(clean.path(), ".claude/skills/ttm-tdd").is_ok());
}
