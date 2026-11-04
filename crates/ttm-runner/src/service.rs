//! Background service definitions for `ttm runner install-service` / `uninstall-service`: launchd (macOS), systemd
//! `--user` (Linux), Task Scheduler (Windows).
//!
//! launchd and systemd start services with a bare `PATH` (`/usr/bin:/bin:…`), where `claude`, `gh` and often `git` are
//! missing. The plist and the unit therefore carry a `PATH`: the one of the shell that ran `install-service`, plus
//! the usual install directories (`~/.local/bin`, `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin`, the system
//! directories) when they are not in it yet. A Windows logon task runs with the user's own environment (the `PATH`
//! set in Windows), so it needs nothing extra.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub const LABEL: &str = "com.toantranmei.ttm-runner";
pub const SYSTEMD_UNIT: &str = "ttm-runner.service";
pub const WINDOWS_TASK: &str = "ttm-runner";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Directories added after the installing shell's `PATH` (Unix), in this order, when missing.
pub fn default_path_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![home.join(".local").join("bin"), home.join(".cargo").join("bin")];
    dirs.extend(
        [
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
            "/usr/sbin",
            "/sbin",
        ]
        .map(PathBuf::from),
    );
    dirs
}

/// The `PATH` written into the service: `current` (the installing shell's), without empty or relative entries
/// (`.` would make the service run whatever lies in its working directory) and duplicates, then the
/// [`default_path_dirs`] it lacks. Unix only (`:`-separated).
pub fn service_path(current: Option<&OsStr>, home: &Path) -> String {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let from_shell = current
        .map(|p| std::env::split_paths(p).collect::<Vec<_>>())
        .unwrap_or_default();
    for dir in from_shell.into_iter().chain(default_path_dirs(home)) {
        let text = dir.to_string_lossy();
        if text.is_empty() || !dir.is_absolute() || text.contains(':') || dirs.contains(&dir) {
            continue;
        }
        dirs.push(dir);
    }
    dirs.iter()
        .map(|d| d.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":")
}

/// `~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist`, with `path` as the service's `PATH`.
pub fn launchd_plist(ttm: &Path, log_dir: &Path, profile: Option<&str>, path: &str) -> String {
    let mut args = format!(
        "    <string>{}</string>\n",
        xml_escape(&ttm.display().to_string())
    );
    if let Some(p) = profile {
        args.push_str(&format!(
            "    <string>--profile</string>\n    <string>{}</string>\n",
            xml_escape(p)
        ));
    }
    args.push_str("    <string>runner</string>\n    <string>up</string>\n");
    let log = xml_escape(&log_dir.join("runner.log").display().to_string());
    let path = xml_escape(path);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{LABEL}</string>
  <key>ProgramArguments</key>
  <array>
{args}  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>{path}</string>
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#
    )
}

/// `~/.config/systemd/user/ttm-runner.service`, with `path` as the service's `PATH`.
pub fn systemd_unit(ttm: &Path, profile: Option<&str>, path: &str) -> String {
    let profile = profile.map(|p| format!(" --profile {p}")).unwrap_or_default();
    // Quoted assignment: `\` and `"` escaped, `%` doubled (systemd specifiers).
    let path = path.replace('\\', "\\\\").replace('"', "\\\"").replace('%', "%%");
    format!(
        "[Unit]\nDescription=ttm runner (agent jobs from toantranmei.com)\nAfter=network-online.target\n\n[Service]\nEnvironment=\"PATH={path}\"\nExecStart={}{profile} runner up\nRestart=on-failure\nRestartSec=10\n\n[Install]\nWantedBy=default.target\n",
        ttm.display()
    )
}

/// `schtasks` arguments creating a logon task.
pub fn schtasks_args(ttm: &Path, profile: Option<&str>) -> Vec<String> {
    let profile = profile.map(|p| format!(" --profile {p}")).unwrap_or_default();
    vec![
        "/Create".into(),
        "/F".into(),
        "/SC".into(),
        "ONLOGON".into(),
        "/TN".into(),
        WINDOWS_TASK.into(),
        "/TR".into(),
        format!("\"{}\"{profile} runner up", ttm.display()),
    ]
}

/// `schtasks` arguments ending the running task (errors ignored: it may not run).
pub fn schtasks_end_args() -> Vec<String> {
    ["/End", "/TN", WINDOWS_TASK].map(String::from).to_vec()
}

/// `schtasks` arguments deleting the task.
pub fn schtasks_delete_args() -> Vec<String> {
    ["/Delete", "/F", "/TN", WINDOWS_TASK].map(String::from).to_vec()
}

pub fn launchd_path(home: &Path) -> PathBuf {
    home.join("Library")
        .join("LaunchAgents")
        .join(format!("{LABEL}.plist"))
}

pub fn systemd_path(home: &Path) -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("systemd")
        .join("user")
        .join(SYSTEMD_UNIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions() {
        let plist = launchd_plist(
            Path::new("/usr/local/bin/ttm"),
            Path::new("/tmp/logs"),
            Some("work"),
            "/opt/homebrew/bin:/usr/bin",
        );
        assert!(plist.contains("<string>/usr/local/bin/ttm</string>"));
        assert!(plist.contains("<string>--profile</string>\n    <string>work</string>"));
        assert!(plist.contains("<string>runner</string>\n    <string>up</string>"));
        assert!(plist.contains(
            "<key>EnvironmentVariables</key>\n  <dict>\n    <key>PATH</key>\n    <string>/opt/homebrew/bin:/usr/bin</string>"
        ));
        let unit = systemd_unit(
            Path::new("/home/a/.cargo/bin/ttm"),
            None,
            "/home/a/.local/bin:/usr/bin",
        );
        assert!(unit.contains("ExecStart=/home/a/.cargo/bin/ttm runner up"));
        assert!(unit.contains("Environment=\"PATH=/home/a/.local/bin:/usr/bin\"\n"));
        assert!(systemd_unit(Path::new("/t"), None, "/a%b").contains("PATH=/a%%b"));
        let args = schtasks_args(Path::new(r"C:\ttm\ttm.exe"), None);
        assert_eq!(args.last().unwrap(), r#""C:\ttm\ttm.exe" runner up"#);
        assert_eq!(schtasks_delete_args(), ["/Delete", "/F", "/TN", "ttm-runner"]);
    }

    #[cfg(unix)]
    #[test]
    fn service_path_keeps_the_shell_path_and_adds_defaults() {
        let home = Path::new("/home/a");
        let path = service_path(
            Some(OsStr::new("/home/a/.nvm/bin:.::/usr/bin:/home/a/.nvm/bin")),
            home,
        );
        let dirs: Vec<&str> = path.split(':').collect();
        assert_eq!(&dirs[..3], ["/home/a/.nvm/bin", "/usr/bin", "/home/a/.local/bin"]);
        for want in [
            "/home/a/.cargo/bin",
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/bin",
        ] {
            assert!(dirs.contains(&want), "{path}");
        }
        assert_eq!(dirs.iter().filter(|d| **d == "/usr/bin").count(), 1, "{path}");
        assert!(!dirs.contains(&"."), "{path}");
        assert!(
            service_path(None, home).starts_with("/home/a/.local/bin:/home/a/.cargo/bin:/opt/homebrew/bin")
        );
    }
}
