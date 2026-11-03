//! Background service definitions for `ttm runner install-service`: launchd (macOS), systemd `--user` (Linux),
//! Task Scheduler (Windows).

use std::path::{Path, PathBuf};

pub const LABEL: &str = "com.toantranmei.ttm-runner";
pub const SYSTEMD_UNIT: &str = "ttm-runner.service";
pub const WINDOWS_TASK: &str = "ttm-runner";

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// `~/Library/LaunchAgents/com.toantranmei.ttm-runner.plist`.
pub fn launchd_plist(ttm: &Path, log_dir: &Path, profile: Option<&str>) -> String {
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

/// `~/.config/systemd/user/ttm-runner.service`.
pub fn systemd_unit(ttm: &Path, profile: Option<&str>) -> String {
    let profile = profile.map(|p| format!(" --profile {p}")).unwrap_or_default();
    format!(
        "[Unit]\nDescription=ttm runner (agent jobs from toantranmei.com)\nAfter=network-online.target\n\n[Service]\nExecStart={}{profile} runner up\nRestart=on-failure\nRestartSec=10\n\n[Install]\nWantedBy=default.target\n",
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
        );
        assert!(plist.contains("<string>/usr/local/bin/ttm</string>"));
        assert!(plist.contains("<string>--profile</string>\n    <string>work</string>"));
        assert!(plist.contains("<string>runner</string>\n    <string>up</string>"));
        let unit = systemd_unit(Path::new("/home/a/.cargo/bin/ttm"), None);
        assert!(unit.contains("ExecStart=/home/a/.cargo/bin/ttm runner up"));
        let args = schtasks_args(Path::new(r"C:\ttm\ttm.exe"), None);
        assert_eq!(args.last().unwrap(), r#""C:\ttm\ttm.exe" runner up"#);
    }
}
