//! Small helpers: executables on PATH, opening the browser.

use std::path::{Path, PathBuf};

/// Finds `name` on `PATH` (with `PATHEXT` on Windows).
pub fn which(name: &str) -> Option<PathBuf> {
    which_in(name, std::env::var_os("PATH")?.as_os_str())
}

pub fn which_in(name: &str, path: &std::ffi::OsStr) -> Option<PathBuf> {
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".into())
            .split(';')
            .map(str::to_ascii_lowercase)
            .chain(std::iter::once(String::new()))
            .collect()
    } else {
        vec![String::new()]
    };
    std::env::split_paths(path).find_map(|dir| {
        exts.iter()
            .map(|ext| dir.join(format!("{name}{ext}")))
            .find(|p| is_executable(p))
    })
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    true
}

/// Opens `url` in the default browser unless `no_browser`; always prints it so it can be copied.
pub fn open_browser(url: &str, no_browser: bool) {
    eprintln!("Open this URL in your browser / Mở URL này trong trình duyệt:\n  {url}");
    if no_browser || std::env::var_os("TTM_NO_BROWSER").is_some() {
        return;
    }
    if let Err(err) = open::that_detached(url) {
        tracing::debug!("could not open a browser: {err}");
    }
}

/// First line of `<program> --version`, if it runs.
pub fn version_of(program: &Path) -> Option<String> {
    let out = std::process::Command::new(program)
        .arg("--version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .next()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn which_finds_executables_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("tool");
        std::fs::write(&exe, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(dir.path().join("plain"), "x").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        assert_eq!(which_in("tool", &path), Some(exe));
        assert_eq!(which_in("plain", &path), None);
        assert_eq!(which_in("missing", &path), None);
    }
}
