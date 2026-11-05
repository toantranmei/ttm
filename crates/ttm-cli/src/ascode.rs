//! Checks of the as-code files under `.ttm/` before `ttm sync` sends them (S5): YAML that parses, no secrets in
//! agent files, skills that pass the bundle checks. Problems stop the sync; nothing is sent.

use std::collections::BTreeMap;
use std::path::Path;
use ttm_core::skill_zip;

/// One problem in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub rule: &'static str,
    pub path: String,
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "{} {}:{l} {}", self.rule, self.path, self.message),
            None => write!(f, "{} {} {}", self.rule, self.path, self.message),
        }
    }
}

/// Key names that must never be in an agent file, at any depth.
pub fn is_secret_key(key: &str) -> bool {
    let k = key.to_ascii_lowercase();
    k == "token"
        || k.ends_with("_token")
        || k.contains("secret")
        || k.contains("password")
        || k == "api_key"
        || k.ends_with("_api_key")
        || k == "apikey"
}

fn secret_keys(v: &serde_yaml_ng::Value, prefix: &str, out: &mut Vec<String>) {
    match v {
        serde_yaml_ng::Value::Mapping(m) => {
            for (k, child) in m {
                let name = k.as_str().unwrap_or_default();
                let path = if prefix.is_empty() {
                    name.to_string()
                } else {
                    format!("{prefix}.{name}")
                };
                if is_secret_key(name) {
                    out.push(path.clone());
                }
                secret_keys(child, &path, out);
            }
        }
        serde_yaml_ng::Value::Sequence(items) => {
            for (i, child) in items.iter().enumerate() {
                secret_keys(child, &format!("{prefix}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// Checks the changed configuration files (`path` → text, bundle paths) and the skills they touch (read from
/// `ttm_dir/skills/<slug>/`).
pub fn check(changed: &BTreeMap<String, String>, ttm_dir: &Path) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut skills = std::collections::BTreeSet::new();
    for (path, text) in changed {
        if let Some(rest) = path.strip_prefix("skills/") {
            if let Some((slug, _)) = rest.split_once('/') {
                skills.insert(slug.to_string());
            }
            continue;
        }
        let is_yaml = path == "workflow.yaml"
            || ((path.starts_with("rules/") || path.starts_with("agents/"))
                && (path.ends_with(".yaml") || path.ends_with(".yml")));
        if !is_yaml {
            continue;
        }
        match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(text) {
            Err(e) => issues.push(Issue {
                rule: "yaml",
                path: path.clone(),
                line: e.location().map(|l| l.line()),
                message: e.to_string().lines().next().unwrap_or("invalid YAML").to_string(),
            }),
            Ok(value) if path.starts_with("agents/") => {
                let mut found = Vec::new();
                secret_keys(&value, "", &mut found);
                for key in found {
                    issues.push(Issue {
                        rule: "secret_in_file",
                        path: path.clone(),
                        line: text
                            .lines()
                            .position(|l| l.trim_start().starts_with(key.rsplit('.').next().unwrap_or(&key)))
                            .map(|i| i + 1),
                        message: format!(
                            "`{key}` looks like a secret; keep secrets on the server, never in files"
                        ),
                    });
                }
            }
            Ok(_) => {}
        }
    }
    for slug in skills {
        let dir = ttm_dir.join("skills").join(&slug);
        if !dir.is_dir() {
            continue; // removed skill
        }
        if let Err(e) = skill_zip::read_dir(&dir).and_then(|files| skill_zip::check(&files).map(|_| ())) {
            issues.push(Issue {
                rule: e.rule,
                path: format!("skills/{slug}/"),
                line: None,
                message: e.message,
            });
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_and_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let mut files = BTreeMap::new();
        files.insert(
            "agents/dev.yaml".to_string(),
            "name: DEV\nmax_budget_tokens: 10\nmcp:\n  - server: linear\n    token: abc\n".to_string(),
        );
        files.insert("rules/r.yaml".to_string(), "when: [unclosed\n".to_string());
        files.insert("workflow.yaml".to_string(), "name: Aurora\n".to_string());
        let issues = check(&files, dir.path());
        assert_eq!(issues.len(), 2, "{issues:?}");
        assert_eq!(
            (issues[0].rule, issues[0].path.as_str()),
            ("secret_in_file", "agents/dev.yaml")
        );
        assert!(issues[0].message.contains("mcp[0].token"));
        assert_eq!(issues[1].rule, "yaml");
        assert!(issues[1].line.is_some());
        assert!(
            !is_secret_key("max_budget_tokens")
                && is_secret_key("client_secret")
                && is_secret_key("GITHUB_TOKEN")
        );
    }

    #[test]
    fn skills_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("skills/tdd")).unwrap();
        std::fs::write(dir.path().join("skills/tdd/README.md"), "x").unwrap();
        let files = BTreeMap::from([("skills/tdd/README.md".to_string(), "x".to_string())]);
        let issues = check(&files, dir.path());
        assert_eq!(issues[0].rule, "skill_md_missing");
    }
}
