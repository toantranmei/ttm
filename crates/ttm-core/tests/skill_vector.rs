//! Cross-check with the server (S3 `skill_bundle::pack`): the vector directory packs to `expected.sha256`.
//! The server keeps a byte-identical copy at `ai-factory/crates/domain-factory/tests/fixtures/skill-vector/`.

use ttm_core::skill_zip;

// Unix only: the vector has an executable script, and Windows has no executable bit (and may check the files out
// with CRLF), so the packed bytes, and the SHA, differ there by design.
#[cfg(unix)]
#[test]
fn vector_packs_to_the_expected_sha() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/skill-vector");
    let expected = std::fs::read_to_string(dir.join("expected.sha256")).unwrap();
    let (zip, sha, manifest) = skill_zip::pack_dir(&dir.join("skill")).unwrap();
    assert_eq!(sha, expected.trim());
    assert_eq!(manifest.front_matter.name, "ttm-vector");
    let paths: Vec<_> = manifest.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["SKILL.md", "notes/ghi-chú.md", "scripts/run.sh"]);
    assert!(manifest.files[2].executable);
    // Unpacking gives the same files back.
    let out = tempfile::tempdir().unwrap();
    skill_zip::safe_extract(&zip, out.path()).unwrap();
    assert_eq!(skill_zip::pack_dir(out.path()).unwrap().1, sha);
}

#[test]
fn zips_with_parent_paths_are_refused() {
    // A zip written by another tool with `../evil`.
    let files = vec![skill_zip::BundleFile {
        path: "../evil".into(),
        bytes: b"x".to_vec(),
        executable: false,
    }];
    let (zip, _) = skill_zip::pack(&files);
    let out = tempfile::tempdir().unwrap();
    assert_eq!(
        skill_zip::safe_extract(&zip, out.path()).unwrap_err().rule,
        "path_unsafe"
    );
}
