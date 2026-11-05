//! `.ttm/project.toml`: which server project this repository is linked to, and what was synced last.
//!
//! ```text
//! .ttm/
//!   project.toml   committed
//!   plan/          committed: the plan as Markdown (web is the source of truth)
//!   workflow.yaml  committed: the project's workflow (as code, S5)
//!   rules/         committed: automation rules `<slug>.yaml` (not the AGENTS.md rules of `ttm rules`)
//!   agents/        committed: agents `<handle>.yaml` (never secrets)
//!   skills/        committed: skills `<slug>/SKILL.md …`
//!   base/          ignored: the plan as of the last sync (common ancestor for 3-way comparisons)
//!   base-config/   ignored: the configuration as of the last sync
//!   cache/         ignored: rules, scratch
//!   resolve/       ignored: conflict resolution inputs
//! ```

use crate::paths::write_atomic;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const DIR: &str = ".ttm";
pub const FILE: &str = "project.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub host: String,
    pub project_id: String,
    pub key: String,
    #[serde(default)]
    pub synced_rev: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
    /// Change request sent by `ttm sync` and not yet decided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_change_request: Option<String>,
}

/// A linked repository: its root and its project file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linked {
    pub root: PathBuf,
    pub file: ProjectFile,
}

impl Linked {
    pub fn dir(&self) -> PathBuf {
        self.root.join(DIR)
    }
    pub fn plan_dir(&self) -> PathBuf {
        self.dir().join("plan")
    }
    pub fn base_dir(&self) -> PathBuf {
        self.dir().join("base")
    }
    pub fn cache_dir(&self) -> PathBuf {
        self.dir().join("cache")
    }
    pub fn resolve_dir(&self) -> PathBuf {
        self.dir().join("resolve")
    }
    /// The configuration as of the last sync.
    pub fn config_base_dir(&self) -> PathBuf {
        self.dir().join("base-config")
    }

    /// What `ttm sync` manages: the plan and the as-code configuration.
    pub fn managed() -> &'static [&'static str] {
        &MANAGED
    }

    /// `.ttm/workflow.yaml`, `.ttm/rules/**`, `.ttm/agents/**`, `.ttm/skills/**` (text files), keyed by their bundle
    /// path (`agents/dev.yaml`).
    pub fn read_config(&self) -> Result<BTreeMap<String, String>> {
        read_config_in(&self.dir())
    }

    pub fn write_config(&self, files: &BTreeMap<String, String>) -> Result<()> {
        write_config_in(&self.dir(), files)
    }

    /// Looks for `.ttm/project.toml` in `start` and its parents.
    pub fn find(start: &Path) -> Result<Option<Self>> {
        for dir in start.ancestors() {
            let path = dir.join(DIR).join(FILE);
            if path.is_file() {
                let text = std::fs::read_to_string(&path).map_err(|source| Error::Read {
                    path: path.clone(),
                    source,
                })?;
                let file = toml::from_str(&text).map_err(|e| Error::Toml {
                    path: path.clone(),
                    message: e.to_string(),
                })?;
                return Ok(Some(Self {
                    root: dir.to_path_buf(),
                    file,
                }));
            }
        }
        Ok(None)
    }

    /// Writes `project.toml` and `.ttm/.gitignore`.
    pub fn save(&self) -> Result<()> {
        let path = self.dir().join(FILE);
        let text = toml::to_string_pretty(&self.file).map_err(|e| Error::Toml {
            path: path.clone(),
            message: e.to_string(),
        })?;
        write_atomic(&path, text.as_bytes(), None)?;
        let ignore = self.dir().join(".gitignore");
        if !ignore.exists() {
            write_atomic(&ignore, b"base/\ncache/\nresolve/\n", None)?;
        }
        Ok(())
    }
}

/// Paths managed by `ttm sync`, relative to `.ttm/` (directories end with `/`).
pub const MANAGED: [&str; 5] = ["plan/", "workflow.yaml", "rules/", "agents/", "skills/"];

/// The configuration part of [`MANAGED`].
const CONFIG_DIRS: [&str; 3] = ["rules", "agents", "skills"];
const WORKFLOW_FILE: &str = "workflow.yaml";

/// Configuration files under `dir` (a `.ttm` or a `base-config` directory), keyed `workflow.yaml`, `rules/x.yaml`…
pub fn read_config_in(dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if let Ok(text) = std::fs::read_to_string(dir.join(WORKFLOW_FILE)) {
        out.insert(WORKFLOW_FILE.to_string(), text);
    }
    for sub in CONFIG_DIRS {
        for (k, v) in read_tree(&dir.join(sub))? {
            out.insert(format!("{sub}/{k}"), v);
        }
    }
    Ok(out)
}

/// Makes the configuration under `dir` equal to `files`: writes every file and removes text files that are no longer
/// there. Files that are not text (binary skill assets the bundle skips) are never touched.
pub fn write_config_in(dir: &Path, files: &BTreeMap<String, String>) -> Result<()> {
    let current = read_config_in(dir)?;
    for rel in current.keys().filter(|k| !files.contains_key(*k)) {
        if let Some(path) = safe_join(dir, rel) {
            let _ = std::fs::remove_file(path);
        }
    }
    for (rel, text) in files {
        let path = safe_join(dir, rel).ok_or_else(|| Error::Write {
            path: dir.join(rel),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path escapes the .ttm directory",
            ),
        })?;
        if current.get(rel) != Some(text) {
            write_atomic(&path, text.as_bytes(), None)?;
        }
    }
    Ok(())
}

/// Reads every file under `dir` as `relative/path` → contents (forward slashes). Missing dir = empty.
pub fn read_tree(dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    if dir.is_dir() {
        walk(dir, dir, &mut out)?;
    }
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| Error::Read {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out)?;
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let key = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(key, text);
        }
    }
    Ok(())
}

/// Replaces the contents of `dir` with `files` (paths are checked to stay inside `dir`).
pub fn write_tree(dir: &Path, files: &BTreeMap<String, String>) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(|source| Error::Write {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    std::fs::create_dir_all(dir).map_err(|source| Error::Write {
        path: dir.to_path_buf(),
        source,
    })?;
    for (rel, text) in files {
        let path = safe_join(dir, rel).ok_or_else(|| Error::Write {
            path: dir.join(rel),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "path escapes the plan directory",
            ),
        })?;
        write_atomic(&path, text.as_bytes(), None)?;
    }
    Ok(())
}

/// `dir/rel` when `rel` is relative and has no `..`.
pub fn safe_join(dir: &Path, rel: &str) -> Option<PathBuf> {
    let rel = Path::new(rel);
    let ok = rel
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));
    (ok && !rel.as_os_str().is_empty()).then(|| dir.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_find_and_trees() {
        let dir = tempfile::tempdir().unwrap();
        let linked = Linked {
            root: dir.path().to_path_buf(),
            file: ProjectFile {
                host: "https://a.com".into(),
                project_id: "p1".into(),
                key: "AUR".into(),
                synced_rev: 3,
                ..Default::default()
            },
        };
        linked.save().unwrap();
        let nested = dir.path().join("src/deep");
        std::fs::create_dir_all(&nested).unwrap();
        let found = Linked::find(&nested).unwrap().unwrap();
        assert_eq!(found.file, linked.file);
        assert!(
            std::fs::read_to_string(dir.path().join(".ttm/.gitignore"))
                .unwrap()
                .contains("base/")
        );

        let mut files = BTreeMap::new();
        files.insert("index.md".to_string(), "# Plan".to_string());
        files.insert("tasks/AUR-1.md".to_string(), "---\nkey: AUR-1\n---".to_string());
        write_tree(&found.plan_dir(), &files).unwrap();
        assert_eq!(read_tree(&found.plan_dir()).unwrap(), files);
    }

    #[test]
    fn rejects_escaping_paths() {
        assert!(safe_join(Path::new("/x"), "../etc/passwd").is_none());
        assert!(safe_join(Path::new("/x"), "/abs").is_none());
        assert!(safe_join(Path::new("/x"), "tasks/A.md").is_some());
        let dir = tempfile::tempdir().unwrap();
        let mut files = BTreeMap::new();
        files.insert("../evil.md".to_string(), String::new());
        assert!(write_tree(&dir.path().join("plan"), &files).is_err());
    }

    #[test]
    fn config_round_trip_keeps_binary_files() {
        let dir = tempfile::tempdir().unwrap();
        let ttm = dir.path();
        std::fs::create_dir_all(ttm.join("skills/tdd")).unwrap();
        std::fs::write(ttm.join("skills/tdd/logo.png"), [0xff, 0xfe, 0x00]).unwrap();
        let mut files = BTreeMap::new();
        files.insert("workflow.yaml".to_string(), "name: A\n".to_string());
        files.insert("agents/dev.yaml".to_string(), "name: DEV\n".to_string());
        files.insert("skills/tdd/SKILL.md".to_string(), "---\n".to_string());
        write_config_in(ttm, &files).unwrap();
        assert_eq!(read_config_in(ttm).unwrap(), files);
        files.remove("agents/dev.yaml");
        write_config_in(ttm, &files).unwrap();
        assert!(!ttm.join("agents/dev.yaml").exists());
        assert!(
            ttm.join("skills/tdd/logo.png").exists(),
            "binary files are not touched"
        );
        assert_eq!(Linked::managed().len(), 5);
    }
}
