//! Worktrees of jobs and their retention: finished worktrees are kept `keep_days` (3 by default) for inspection, then
//! removed by the runner. The ledger lives in `<cache>/runner-worktrees.json`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;

pub use ttm_core::worktree::{add, remove, worktree_path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub repo: PathBuf,
    pub path: PathBuf,
    pub run_id: String,
    /// Unix seconds when the job finished.
    pub finished_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default)]
    pub entries: Vec<Entry>,
}

impl Ledger {
    pub fn path(cache_dir: &Path) -> PathBuf {
        cache_dir.join("runner-worktrees.json")
    }

    pub fn load(cache_dir: &Path) -> Self {
        std::fs::read(Self::path(cache_dir))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, cache_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(cache_dir)?;
        std::fs::write(
            Self::path(cache_dir),
            serde_json::to_vec_pretty(self).unwrap_or_default(),
        )
    }

    pub fn record(&mut self, repo: &Path, path: &Path, run_id: &str) {
        self.entries.retain(|e| e.path != path);
        self.entries.push(Entry {
            repo: repo.to_path_buf(),
            path: path.to_path_buf(),
            run_id: run_id.to_string(),
            finished_at: OffsetDateTime::now_utc().unix_timestamp(),
        });
    }

    /// Splits off entries older than `keep_days` at `now`.
    pub fn expired(&mut self, keep_days: u32, now: i64) -> Vec<Entry> {
        let cutoff = now - i64::from(keep_days) * 86_400;
        let (old, keep): (Vec<_>, Vec<_>) = self.entries.drain(..).partition(|e| e.finished_at < cutoff);
        self.entries = keep;
        old
    }
}

/// Removes expired worktrees; returns how many were removed.
pub fn prune(cache_dir: &Path, keep_days: u32) -> usize {
    let mut ledger = Ledger::load(cache_dir);
    let old = ledger.expired(keep_days, OffsetDateTime::now_utc().unix_timestamp());
    let mut removed = 0;
    for e in &old {
        match remove(&e.repo, &e.path) {
            Ok(()) => removed += 1,
            Err(err) => tracing::debug!("could not remove {}: {err}", e.path.display()),
        }
    }
    let _ = ledger.save(cache_dir);
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry() {
        let mut l = Ledger::default();
        l.record(Path::new("/r"), Path::new("/w/a"), "r1");
        l.entries[0].finished_at = 0;
        l.record(Path::new("/r"), Path::new("/w/b"), "r2");
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let old = l.expired(3, now);
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].run_id, "r1");
        assert_eq!(l.entries.len(), 1);
    }

    #[test]
    fn ledger_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut l = Ledger::default();
        l.record(Path::new("/r"), Path::new("/w/a"), "r1");
        l.save(dir.path()).unwrap();
        assert_eq!(Ledger::load(dir.path()), l);
    }
}
