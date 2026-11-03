//! `ttm mcp install --client …`: points an MCP client at `ttm-mcp`.
//!
//! | Client | How |
//! | --- | --- |
//! | Claude Code | `claude mcp add --scope user <name> -- <ttm-mcp> [args]` |
//! | Claude Desktop | `mcpServers.<name>` in `claude_desktop_config.json` |
//! | Cursor | `mcpServers.<name>` in `~/.cursor/mcp.json` |
//! | Codex | `[mcp_servers.<name>]` in `~/.codex/config.toml` |
//!
//! Files are merged (other servers and settings stay as they are, TOML comments included) and the previous file is
//! copied to `<file>.bak-<timestamp>` before any change. An invalid file is left untouched.

use crate::app::App;
use crate::cli::{McpClient, McpCommand, McpInstallArgs};
use crate::output::emit;
use crate::util::which;
use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize)]
struct InstallOutput {
    client: &'static str,
    file: Option<String>,
    backup: Option<String>,
    changed: bool,
    dry_run: bool,
    entry: ServerEntry,
}

pub async fn run(app: App, command: McpCommand) -> Result<ExitCode> {
    let McpCommand::Install(args) = command;
    let entry = entry_for(&app, &args);
    let home = ttm_core::paths::home_dir().context("no home directory")?;
    let mut out = InstallOutput {
        client: client_name(args.client),
        file: None,
        backup: None,
        changed: false,
        dry_run: args.dry_run,
        entry: entry.clone(),
    };
    match args.client {
        McpClient::ClaudeCode => {
            let cmd = claude_code_command(&entry);
            if args.dry_run {
                eprintln!("would run: claude {}", cmd.join(" "));
            } else {
                run_claude(&cmd, &entry.name)?;
                out.changed = true;
            }
        }
        client => {
            let file = config_path(client, &home);
            let current = match std::fs::read_to_string(&file) {
                Ok(text) => Some(text),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                Err(err) => return Err(err).with_context(|| format!("reading {}", file.display())),
            };
            let merged = match client {
                McpClient::Codex => merge_codex(current.as_deref(), &entry)?,
                _ => merge_json(current.as_deref(), &entry)?,
            };
            out.file = Some(file.display().to_string());
            out.changed = current.as_deref() != Some(merged.as_str());
            if out.changed && !args.dry_run {
                if current.is_some() {
                    out.backup = Some(backup(&file)?.display().to_string());
                }
                write(&file, &merged)?;
            } else if args.dry_run {
                eprintln!("--- {} (dry run)\n{merged}", file.display());
            }
        }
    }
    emit(app.json, &out, || {
        if out.dry_run {
            println!("Dry run: nothing written.");
        } else if !out.changed {
            println!(
                "{} already uses {} for `{}`.",
                out.client, out.entry.command, out.entry.name
            );
        } else {
            println!(
                "{} now starts `{}` as MCP server `{}`.",
                out.client, out.entry.command, out.entry.name
            );
            if let Some(file) = &out.file {
                println!("Updated {file}");
            }
            if let Some(backup) = &out.backup {
                println!("Backup: {backup}");
            }
            println!("Restart the client to load it. Sign in first with `ttm login` if you have not.");
        }
    });
    Ok(ExitCode::SUCCESS)
}

fn client_name(client: McpClient) -> &'static str {
    match client {
        McpClient::ClaudeCode => "Claude Code",
        McpClient::ClaudeDesktop => "Claude Desktop",
        McpClient::Cursor => "Cursor",
        McpClient::Codex => "Codex",
    }
}

/// The command: `--command`, else `ttm-mcp` next to this `ttm` (absolute, since GUI apps do not see the shell PATH),
/// else `ttm-mcp` on PATH. Host and profile are pinned only when given explicitly.
fn entry_for(app: &App, args: &McpInstallArgs) -> ServerEntry {
    let command = args.command.clone().unwrap_or_else(|| {
        let exe = format!("ttm-mcp{}", std::env::consts::EXE_SUFFIX);
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(&exe)))
            .filter(|p| p.exists())
            .or_else(|| which("ttm-mcp"))
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "ttm-mcp".into())
    });
    let mut cmd_args = Vec::new();
    if app.host_flag {
        cmd_args.extend(["--host".to_string(), app.host().to_string()]);
    }
    if app.profile_flag {
        cmd_args.extend(["--profile".to_string(), app.ctx.profile_name.clone()]);
    }
    if let Some(toolsets) = &args.toolsets {
        cmd_args.extend(["--toolsets".to_string(), toolsets.clone()]);
    }
    ServerEntry {
        name: args.name.clone(),
        command,
        args: cmd_args,
    }
}

pub fn claude_code_command(entry: &ServerEntry) -> Vec<String> {
    let mut cmd = vec![
        "mcp".into(),
        "add".into(),
        "--scope".into(),
        "user".into(),
        entry.name.clone(),
        "--".into(),
    ];
    cmd.push(entry.command.clone());
    cmd.extend(entry.args.iter().cloned());
    cmd
}

fn run_claude(cmd: &[String], name: &str) -> Result<()> {
    let Some(claude) = which("claude") else {
        bail!(
            "Claude Code (`claude`) is not on PATH. Install it, or run by hand: claude {}",
            cmd.join(" ")
        );
    };
    let output = std::process::Command::new(&claude)
        .args(cmd)
        .output()
        .context("running claude mcp add")?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("already exists") {
        // Our own entry from an earlier install: replace it.
        let _ = std::process::Command::new(&claude)
            .args(["mcp", "remove", "--scope", "user", name])
            .output();
        let retry = std::process::Command::new(&claude).args(cmd).output()?;
        if retry.status.success() {
            return Ok(());
        }
        bail!(
            "claude mcp add failed: {}",
            String::from_utf8_lossy(&retry.stderr).trim()
        );
    }
    bail!("claude mcp add failed: {}", stderr.trim());
}

/// Config file of a client, under `home`.
pub fn config_path(client: McpClient, home: &Path) -> PathBuf {
    match client {
        McpClient::ClaudeDesktop => claude_desktop_dir(home).join("claude_desktop_config.json"),
        McpClient::Cursor => home.join(".cursor").join("mcp.json"),
        McpClient::Codex => std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"))
            .join("config.toml"),
        McpClient::ClaudeCode => home.join(".claude.json"),
    }
}

fn claude_desktop_dir(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library").join("Application Support").join("Claude")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"))
            .join("Claude")
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
            .join("Claude")
    }
}

/// Sets `mcpServers.<name>` and keeps everything else (key order included).
pub fn merge_json(current: Option<&str>, entry: &ServerEntry) -> Result<String> {
    let mut root: Value = match current.map(str::trim).filter(|t| !t.is_empty()) {
        Some(text) => {
            serde_json::from_str(text).context("the existing config is not valid JSON; fix it first")?
        }
        None => Value::Object(Map::new()),
    };
    let Some(obj) = root.as_object_mut() else {
        bail!("the existing config is not a JSON object")
    };
    let servers = obj
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(servers) = servers.as_object_mut() else {
        bail!("`mcpServers` is not an object")
    };
    let mut server = servers
        .get(&entry.name)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    server.insert("command".into(), json!(entry.command));
    server.insert("args".into(), json!(entry.args));
    servers.insert(entry.name.clone(), Value::Object(server));
    let mut text = serde_json::to_string_pretty(&root)?;
    text.push('\n');
    // Unchanged content compares equal so no backup is made.
    if let Some(cur) = current
        && serde_json::from_str::<Value>(cur).ok().as_ref() == Some(&root)
    {
        return Ok(cur.to_string());
    }
    Ok(text)
}

/// Sets `[mcp_servers.<name>] command, args`, preserving comments and other tables.
pub fn merge_codex(current: Option<&str>, entry: &ServerEntry) -> Result<String> {
    let text = current.unwrap_or_default();
    let mut doc: toml_edit::DocumentMut = text
        .parse()
        .context("the existing config.toml is not valid TOML; fix it first")?;
    let servers = doc.entry("mcp_servers").or_insert_with(|| {
        let mut table = toml_edit::Table::new();
        table.set_implicit(true);
        toml_edit::Item::Table(table)
    });
    let Some(servers) = servers.as_table_mut() else {
        bail!("`mcp_servers` is not a table")
    };
    let server = servers
        .entry(&entry.name)
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
    let Some(server) = server.as_table_mut() else {
        bail!("`mcp_servers.{}` is not a table", entry.name)
    };
    let same_command = server.get("command").and_then(|v| v.as_str()) == Some(entry.command.as_str());
    let current_args: Option<Vec<String>> = server
        .get("args")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect());
    if same_command && current_args.as_deref() == Some(entry.args.as_slice()) {
        return Ok(text.to_string());
    }
    server["command"] = toml_edit::value(entry.command.clone());
    let mut args = toml_edit::Array::new();
    for a in &entry.args {
        args.push(a.as_str());
    }
    server["args"] = toml_edit::value(args);
    Ok(doc.to_string())
}

/// Copies `file` to `file.bak-<unix seconds>`.
pub fn backup(file: &Path) -> Result<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!(
        "{}.bak-{stamp}",
        file.file_name().and_then(|n| n.to_str()).unwrap_or("config")
    );
    let target = file.with_file_name(name);
    std::fs::copy(file, &target).with_context(|| format!("backing up {}", file.display()))?;
    Ok(target)
}

fn write(file: &Path, text: &str) -> Result<()> {
    ttm_core::paths::write_atomic(file, text.as_bytes(), None).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> ServerEntry {
        ServerEntry {
            name: "ttm".into(),
            command: "/usr/local/bin/ttm-mcp".into(),
            args: vec![],
        }
    }

    #[test]
    fn json_merge_keeps_other_servers_and_order() {
        let current = r#"{
  "theme": "dark",
  "mcpServers": { "github": { "command": "gh-mcp", "args": ["--x"], "env": {"A": "1"} } },
  "zzz": 1
}"#;
        let merged = merge_json(Some(current), &entry()).unwrap();
        let v: Value = serde_json::from_str(&merged).unwrap();
        assert_eq!(v["mcpServers"]["github"]["env"]["A"], "1");
        assert_eq!(v["mcpServers"]["ttm"]["command"], "/usr/local/bin/ttm-mcp");
        assert_eq!(v["theme"], "dark");
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["theme", "mcpServers", "zzz"]);
    }

    #[test]
    fn json_merge_keeps_env_of_existing_entry_and_is_idempotent() {
        let first = merge_json(None, &entry()).unwrap();
        assert_eq!(merge_json(Some(&first), &entry()).unwrap(), first);
        let with_env = r#"{"mcpServers":{"ttm":{"command":"old","args":[],"env":{"TTM_HOST":"http://localhost:3000"}}}}"#;
        let v: Value = serde_json::from_str(&merge_json(Some(with_env), &entry()).unwrap()).unwrap();
        assert_eq!(v["mcpServers"]["ttm"]["env"]["TTM_HOST"], "http://localhost:3000");
        assert_eq!(v["mcpServers"]["ttm"]["command"], "/usr/local/bin/ttm-mcp");
    }

    #[test]
    fn json_merge_refuses_invalid_files() {
        assert!(merge_json(Some("{ not json"), &entry()).is_err());
        assert!(merge_json(Some("[]"), &entry()).is_err());
    }

    #[test]
    fn codex_merge_preserves_comments() {
        let current = "# my settings\nmodel = \"o3\"\n\n[mcp_servers.other]\ncommand = \"x\"\n";
        let merged = merge_codex(
            Some(current),
            &ServerEntry {
                args: vec!["--toolsets".into(), "notee".into()],
                ..entry()
            },
        )
        .unwrap();
        assert!(merged.contains("# my settings"));
        assert!(merged.contains("[mcp_servers.other]"));
        let parsed: toml::Value = toml::from_str(&merged).unwrap();
        assert_eq!(
            parsed["mcp_servers"]["ttm"]["command"].as_str(),
            Some("/usr/local/bin/ttm-mcp")
        );
        assert_eq!(parsed["mcp_servers"]["ttm"]["args"][1].as_str(), Some("notee"));
        assert_eq!(parsed["model"].as_str(), Some("o3"));
        let again = merge_codex(
            Some(&merged),
            &ServerEntry {
                args: vec!["--toolsets".into(), "notee".into()],
                ..entry()
            },
        )
        .unwrap();
        assert_eq!(again, merged);
    }

    #[test]
    fn codex_from_scratch() {
        let merged = merge_codex(None, &entry()).unwrap();
        let parsed: toml::Value = toml::from_str(&merged).unwrap();
        assert!(
            parsed["mcp_servers"]["ttm"]["args"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn claude_code_command_shape() {
        let cmd = claude_code_command(&ServerEntry {
            args: vec!["--host".into(), "http://localhost:3000".into()],
            ..entry()
        });
        assert_eq!(
            cmd,
            [
                "mcp",
                "add",
                "--scope",
                "user",
                "ttm",
                "--",
                "/usr/local/bin/ttm-mcp",
                "--host",
                "http://localhost:3000"
            ]
        );
    }

    #[test]
    fn paths() {
        let home = Path::new("/home/an");
        assert_eq!(
            config_path(McpClient::Cursor, home),
            PathBuf::from("/home/an/.cursor/mcp.json")
        );
        assert!(config_path(McpClient::ClaudeDesktop, home).ends_with("Claude/claude_desktop_config.json"));
    }

    #[test]
    fn backup_copies() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mcp.json");
        std::fs::write(&file, "{}").unwrap();
        let b = backup(&file).unwrap();
        assert_eq!(std::fs::read_to_string(b).unwrap(), "{}");
    }
}
