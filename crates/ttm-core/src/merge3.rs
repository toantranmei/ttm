//! Three-way comparison of plan bundles (path → contents): what changed locally, on the server, and where both sides
//! changed the same file differently.

use std::collections::{BTreeMap, BTreeSet};

pub type Files = BTreeMap<String, String>;

/// One file changed relative to the base: new contents, or `None` when deleted.
pub type Changes = BTreeMap<String, Option<String>>;

/// Files of `side` that differ from `base` (added, changed, or deleted).
pub fn changes(base: &Files, side: &Files) -> Changes {
    let paths: BTreeSet<&String> = base.keys().chain(side.keys()).collect();
    paths
        .into_iter()
        .filter(|p| base.get(*p) != side.get(*p))
        .map(|p| (p.clone(), side.get(p).cloned()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merge {
    /// Result where the sides do not conflict; conflicting paths keep the local version.
    pub files: Files,
    /// Paths changed on both sides differently.
    pub conflicts: Vec<String>,
    /// Paths only changed locally (to send as a change request).
    pub local_only: Vec<String>,
}

/// File-level 3-way merge.
pub fn merge(base: &Files, local: &Files, remote: &Files) -> Merge {
    let local_changes = changes(base, local);
    let remote_changes = changes(base, remote);
    let mut files = remote.clone();
    let mut conflicts = Vec::new();
    let mut local_only = Vec::new();
    for (path, local_value) in &local_changes {
        match remote_changes.get(path) {
            Some(remote_value) if remote_value != local_value => conflicts.push(path.clone()),
            Some(_) => {}
            None => local_only.push(path.clone()),
        }
        match local_value {
            Some(text) => {
                files.insert(path.clone(), text.clone());
            }
            None => {
                files.remove(path);
            }
        }
    }
    Merge {
        files,
        conflicts,
        local_only,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(pairs: &[(&str, &str)]) -> Files {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn detects_changes_additions_and_deletions() {
        let base = files(&[("a", "1"), ("b", "2")]);
        let side = files(&[("a", "1"), ("c", "3")]);
        let c = changes(&base, &side);
        assert_eq!(c.get("b"), Some(&None));
        assert_eq!(c.get("c"), Some(&Some("3".into())));
        assert!(!c.contains_key("a"));
    }

    #[test]
    fn merges_independent_changes_and_flags_conflicts() {
        let base = files(&[("a", "1"), ("b", "2"), ("c", "3")]);
        let local = files(&[("a", "1 local"), ("b", "2 local"), ("c", "3")]);
        let remote = files(&[("a", "1"), ("b", "2 server"), ("c", "3 server"), ("d", "new")]);
        let m = merge(&base, &local, &remote);
        assert_eq!(m.conflicts, ["b"]);
        assert_eq!(m.local_only, ["a"]);
        assert_eq!(m.files["a"], "1 local");
        assert_eq!(m.files["c"], "3 server");
        assert_eq!(m.files["d"], "new");
    }

    #[test]
    fn same_change_on_both_sides_is_not_a_conflict() {
        let base = files(&[("a", "1")]);
        let both = files(&[("a", "2")]);
        let m = merge(&base, &both, &both);
        assert!(m.conflicts.is_empty() && m.local_only.is_empty());
    }
}
