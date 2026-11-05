//! Local MCP servers of this machine (`[mcp.local.<name>]` in `runner.toml`): who may use them, how a job gets them,
//! what the server is told (names and permissions only, never commands, arguments or environment), and the edits of
//! `ttm runner config mcp …` (made with `toml_edit`, so comments stay).
//!
//! ```toml
//! [mcp.local.github-local]
//! command = "npx"
//! args = ["-y", "@modelcontextprotocol/server-github"]
//! env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }   # "env:VAR" = read from the runner's environment at run time
//! allow = ["dev", "reviewer"]                    # agent handles
//! team_jobs = false                              # also for jobs delegated by someone else
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ttm_api::factory::LocalMcpAnnounce;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpSection {
    #[serde(default)]
    pub local: BTreeMap<String, LocalMcp>,
}

impl McpSection {
    pub fn is_empty(&self) -> bool {
        self.local.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalMcp {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub team_jobs: bool,
}

/// `^[a-z0-9][a-z0-9-]{0,31}$` and not `ttm` (the runner's own server).
pub fn valid_name(name: &str) -> bool {
    let b = name.as_bytes();
    name != "ttm"
        && !b.is_empty()
        && b.len() <= 32
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// The server's `grant_local` rule: the agent is in `allow`, and the job is not a team job unless `team_jobs`.
pub fn grant<'a>(
    name: &str,
    handle: Option<&str>,
    team_job: bool,
    section: &'a McpSection,
) -> Option<&'a LocalMcp> {
    let mcp = section.local.get(name)?;
    let handle = handle.filter(|h| !h.is_empty())?;
    let allowed = mcp
        .allow
        .iter()
        .any(|a| a.trim_start_matches('@').eq_ignore_ascii_case(handle));
    (allowed && (!team_job || mcp.team_jobs)).then_some(mcp)
}

/// Environment of a local MCP with `env:VAR` values read through `getenv`; `Err(var)` names the first missing one.
pub fn resolve_env(
    env: &BTreeMap<String, String>,
    getenv: &dyn Fn(&str) -> Option<String>,
) -> Result<BTreeMap<String, String>, String> {
    env.iter()
        .map(|(k, v)| match v.strip_prefix("env:") {
            Some(var) => getenv(var)
                .filter(|x| !x.is_empty())
                .map(|x| (k.clone(), x))
                .ok_or_else(|| var.to_string()),
            None => Ok((k.clone(), v.clone())),
        })
        .collect()
}

/// What the server is told: names, allowed agents and `team_jobs`.
pub fn announce(section: &McpSection) -> Vec<LocalMcpAnnounce> {
    section
        .local
        .iter()
        .map(|(name, m)| LocalMcpAnnounce {
            name: name.clone(),
            allow: m.allow.clone(),
            team_jobs: m.team_jobs,
        })
        .collect()
}

/// Edits of `runner.toml` keeping comments and layout.
pub mod edit {
    use super::valid_name;
    use toml_edit::{Array, DocumentMut, InlineTable, Item, Table, value};

    fn local(doc: &mut DocumentMut) -> &mut Table {
        let mcp = doc.entry("mcp").or_insert_with(|| {
            let mut t = Table::new();
            t.set_implicit(true);
            Item::Table(t)
        });
        let mcp = mcp.as_table_mut().expect("[mcp] is a table");
        let local = mcp.entry("local").or_insert_with(|| {
            let mut t = Table::new();
            t.set_implicit(true);
            Item::Table(t)
        });
        local.as_table_mut().expect("[mcp.local] is a table")
    }

    fn server<'a>(doc: &'a mut DocumentMut, name: &str) -> anyhow::Result<&'a mut Table> {
        local(doc)
            .get_mut(name)
            .and_then(Item::as_table_mut)
            .ok_or_else(|| anyhow::anyhow!("no local MCP server `{name}` in runner.toml"))
    }

    fn array(items: &[String]) -> Array {
        items.iter().map(String::as_str).collect()
    }

    /// Adds or replaces `[mcp.local.<name>]`. Values of `env` must be `env:VAR` or plain non-secret values.
    pub fn add(
        doc: &mut DocumentMut,
        name: &str,
        command: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> anyhow::Result<()> {
        if !valid_name(name) {
            anyhow::bail!("`{name}` is not a valid local MCP name (a-z, 0-9, `-`, up to 32, not `ttm`)");
        }
        let mut t = Table::new();
        t["command"] = value(command);
        t["args"] = value(array(args));
        let mut e = InlineTable::new();
        for (k, v) in env {
            e.insert(k, v.as_str().into());
        }
        t["env"] = value(e);
        t["allow"] = value(Array::new());
        t["team_jobs"] = value(false);
        local(doc).insert(name, Item::Table(t));
        Ok(())
    }

    /// Adds agent handles to `allow`.
    pub fn allow(doc: &mut DocumentMut, name: &str, handles: &[String]) -> anyhow::Result<Vec<String>> {
        let t = server(doc, name)?;
        let mut current: Vec<String> = t
            .get("allow")
            .and_then(Item::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        for h in handles {
            let h = h.trim_start_matches('@').to_string();
            if !current.contains(&h) {
                current.push(h);
            }
        }
        t["allow"] = value(array(&current));
        Ok(current)
    }

    /// Removes agent handles from `allow`.
    pub fn deny(doc: &mut DocumentMut, name: &str, handles: &[String]) -> anyhow::Result<Vec<String>> {
        let t = server(doc, name)?;
        let drop: Vec<String> = handles
            .iter()
            .map(|h| h.trim_start_matches('@').to_string())
            .collect();
        let current: Vec<String> = t
            .get("allow")
            .and_then(Item::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default()
            .into_iter()
            .filter(|h| !drop.contains(h))
            .collect();
        t["allow"] = value(array(&current));
        Ok(current)
    }

    pub fn team_jobs(doc: &mut DocumentMut, name: &str, on: bool) -> anyhow::Result<()> {
        server(doc, name)?["team_jobs"] = value(on);
        Ok(())
    }

    pub fn remove(doc: &mut DocumentMut, name: &str) -> anyhow::Result<()> {
        local(doc)
            .remove(name)
            .map(drop)
            .ok_or_else(|| anyhow::anyhow!("no local MCP server `{name}` in runner.toml"))
    }

    /// Local MCP names whose `allow` would include `handle` with every name when `names` is empty.
    pub fn names(doc: &DocumentMut) -> Vec<String> {
        doc.get("mcp")
            .and_then(|m| m.get("local"))
            .and_then(Item::as_table)
            .map(|t| t.iter().map(|(k, _)| k.to_string()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section() -> McpSection {
        toml::from_str::<crate::RunnerConfig>(
            r#"
            [mcp.local.github-local]
            command = "npx"
            args = ["-y", "@modelcontextprotocol/server-github"]
            env = { GITHUB_TOKEN = "env:GITHUB_TOKEN", MODE = "ro" }
            allow = ["dev"]
            team_jobs = false
            "#,
        )
        .unwrap()
        .mcp
    }

    #[test]
    fn grants_follow_allow_and_team_jobs() {
        let s = section();
        assert!(grant("github-local", Some("dev"), false, &s).is_some());
        assert!(grant("github-local", Some("qa"), false, &s).is_none());
        assert!(
            grant("github-local", Some("dev"), true, &s).is_none(),
            "team job without team_jobs"
        );
        assert!(
            grant("github-local", None, false, &s).is_none(),
            "a person's task has no agent"
        );
        assert!(grant("nope", Some("dev"), false, &s).is_none());
    }

    #[test]
    fn env_vars_are_read_at_run_time() {
        let s = section();
        let env = &s.local["github-local"].env;
        let ok = resolve_env(env, &|v| (v == "GITHUB_TOKEN").then(|| "ghp_x".to_string())).unwrap();
        assert_eq!(ok["GITHUB_TOKEN"], "ghp_x");
        assert_eq!(ok["MODE"], "ro");
        assert_eq!(resolve_env(env, &|_| None).unwrap_err(), "GITHUB_TOKEN");
    }

    #[test]
    fn announce_has_no_command_or_env() {
        let a = announce(&section());
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("github-local") && json.contains("dev"));
        assert!(!json.contains("npx") && !json.contains("GITHUB_TOKEN"));
    }

    #[test]
    fn names() {
        assert!(valid_name("github-local") && valid_name("a1"));
        assert!(
            !valid_name("ttm") && !valid_name("-x") && !valid_name("Big") && !valid_name(&"a".repeat(33))
        );
    }

    #[test]
    fn edits_keep_comments() {
        let text = "# my runner\nname = \"mac\" # this machine\n\n[repos]\nAUR = \"/code/aurora\"\n";
        let mut doc: toml_edit::DocumentMut = text.parse().unwrap();
        edit::add(
            &mut doc,
            "github-local",
            "npx",
            &["-y".into()],
            &[("GITHUB_TOKEN".into(), "env:GITHUB_TOKEN".into())],
        )
        .unwrap();
        edit::allow(&mut doc, "github-local", &["@dev".into(), "qa".into()]).unwrap();
        edit::deny(&mut doc, "github-local", &["qa".into()]).unwrap();
        edit::team_jobs(&mut doc, "github-local", true).unwrap();
        let out = doc.to_string();
        assert!(
            out.contains("# my runner") && out.contains("# this machine"),
            "{out}"
        );
        let parsed: crate::RunnerConfig = toml::from_str(&out).unwrap();
        let m = &parsed.mcp.local["github-local"];
        assert_eq!((m.allow.clone(), m.team_jobs), (vec!["dev".to_string()], true));
        assert_eq!(m.env["GITHUB_TOKEN"], "env:GITHUB_TOKEN", "never the real value");
        assert!(edit::add(&mut doc, "ttm", "x", &[], &[]).is_err());
        edit::remove(&mut doc, "github-local").unwrap();
        assert!(edit::names(&doc).is_empty());
    }
}
