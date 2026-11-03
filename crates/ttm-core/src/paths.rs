//! Where ttm keeps its files.
//!
//! | | Unix (macOS, Linux) | Windows |
//! | --- | --- | --- |
//! | config | `$XDG_CONFIG_HOME/ttm` or `~/.config/ttm` | `%APPDATA%\ttm` |
//! | cache | `$XDG_CACHE_HOME/ttm` or `~/.cache/ttm` | `%LOCALAPPDATA%\ttm\cache` |
//!
//! `TTM_CONFIG_DIR` and `TTM_CACHE_DIR` override both (tests, portable installs).

use crate::{Error, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl Paths {
    /// Directories for this user, honouring `TTM_CONFIG_DIR` / `TTM_CACHE_DIR`.
    pub fn detect() -> Result<Self> {
        let env = |name: &str| {
            std::env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let base = directories::BaseDirs::new();
        let config_dir = match env("TTM_CONFIG_DIR") {
            Some(dir) => dir,
            None => default_config_dir(base.as_ref(), env("XDG_CONFIG_HOME"))?,
        };
        let cache_dir = match env("TTM_CACHE_DIR") {
            Some(dir) => dir,
            None => default_cache_dir(base.as_ref(), env("XDG_CACHE_HOME"))?,
        };
        Ok(Self {
            config_dir,
            cache_dir,
        })
    }

    /// Everything under one root (`<root>/config`, `<root>/cache`); used by tests.
    pub fn under(root: &Path) -> Self {
        Self {
            config_dir: root.join("config"),
            cache_dir: root.join("cache"),
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn credentials_file(&self) -> PathBuf {
        self.config_dir.join("credentials.toml")
    }
}

fn default_config_dir(base: Option<&directories::BaseDirs>, xdg: Option<PathBuf>) -> Result<PathBuf> {
    let base = base.ok_or(Error::NoHome)?;
    if cfg!(windows) {
        return Ok(base.config_dir().join("ttm"));
    }
    Ok(xdg.unwrap_or_else(|| base.home_dir().join(".config")).join("ttm"))
}

fn default_cache_dir(base: Option<&directories::BaseDirs>, xdg: Option<PathBuf>) -> Result<PathBuf> {
    let base = base.ok_or(Error::NoHome)?;
    if cfg!(windows) {
        return Ok(base.data_local_dir().join("ttm").join("cache"));
    }
    Ok(xdg.unwrap_or_else(|| base.home_dir().join(".cache")).join("ttm"))
}

/// Writes `contents` to `path` atomically (temp file + rename), creating parent directories. On Unix the file gets
/// `mode` before it is moved into place, so a secret is never readable by others even briefly.
/// The user's home directory: `TTM_HOME` when set (tests, portable installs), else the platform's (on Windows the
/// profile folder from the Known Folder API, which ignores `HOME`).
pub fn home_dir() -> Option<PathBuf> {
    match std::env::var_os("TTM_HOME").filter(|value| !value.is_empty()) {
        Some(home) => Some(PathBuf::from(home)),
        None => directories::BaseDirs::new().map(|base| base.home_dir().to_path_buf()),
    }
}

pub fn write_atomic(path: &Path, contents: &[u8], mode: Option<u32>) -> Result<()> {
    let write_err = |source| Error::Write {
        path: path.to_path_buf(),
        source,
    };
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).map_err(write_err)?;
    let tmp = dir.join(format!(
        ".{}.tmp-{}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("ttm"),
        std::process::id()
    ));
    std::fs::write(&tmp, contents).map_err(write_err)?;
    #[cfg(unix)]
    if let Some(mode) = mode {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode)).map_err(write_err)?;
    }
    #[cfg(not(unix))]
    let _ = mode;
    std::fs::rename(&tmp, path).map_err(write_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_puts_config_and_cache_side_by_side() {
        let paths = Paths::under(Path::new("/tmp/x"));
        assert_eq!(paths.config_file(), PathBuf::from("/tmp/x/config/config.toml"));
        assert_eq!(
            paths.credentials_file(),
            PathBuf::from("/tmp/x/config/credentials.toml")
        );
        assert_eq!(paths.cache_dir, PathBuf::from("/tmp/x/cache"));
    }

    #[cfg(unix)]
    #[test]
    fn xdg_dirs_win_over_home() {
        let base = directories::BaseDirs::new().unwrap();
        let dir = default_config_dir(Some(&base), Some(PathBuf::from("/xdg/config"))).unwrap();
        assert_eq!(dir, PathBuf::from("/xdg/config/ttm"));
        let dir = default_cache_dir(Some(&base), None).unwrap();
        assert_eq!(dir, base.home_dir().join(".cache").join("ttm"));
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_sets_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("secret.toml");
        write_atomic(&path, b"x = 1", Some(0o600)).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x = 1");
    }
}
