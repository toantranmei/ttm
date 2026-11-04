//! `ttm self-update`: download the release archive for this platform from GitHub Releases, check its SHA-256 (and the
//! GitHub artifact attestation when the `gh` CLI is installed), then replace `ttm` (and `ttm-mcp` next to it).

use crate::app::App;
use crate::cli::SelfUpdateArgs;
use crate::output::emit;
use crate::util::which;
use anyhow::{Context as _, Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Target triple this binary was built for.
pub const TARGET: &str = env!("TTM_TARGET");

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Serialize)]
struct UpdateOutput {
    current: &'static str,
    latest: String,
    update_available: bool,
    updated: Vec<String>,
    attestation: &'static str,
}

pub async fn run(app: App, args: SelfUpdateArgs) -> Result<ExitCode> {
    let http = reqwest::Client::builder()
        .user_agent(ttm_api::user_agent())
        .build()?;
    let release = fetch_release(&http, &args.repo, args.version.as_deref()).await?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let newer = is_newer(&latest, ttm_core::VERSION);
    let mut out = UpdateOutput {
        current: ttm_core::VERSION,
        latest: latest.clone(),
        update_available: newer,
        updated: vec![],
        attestation: "skipped",
    };
    if args.check || (!newer && args.version.is_none()) {
        emit(&app, &out, |_ui| {
            if newer {
                println!(
                    "ttm {latest} is available (this is {}). Run `ttm self-update`.",
                    ttm_core::VERSION
                );
            } else {
                println!("ttm {} is up to date.", ttm_core::VERSION);
            }
        });
        return Ok(ExitCode::SUCCESS);
    }

    let current_exe = std::env::current_exe().context("cannot locate the running ttm")?;
    let dir = current_exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut binaries = vec![("ttm-cli", "ttm", current_exe.clone())];
    let mcp = dir.join(exe_name("ttm-mcp"));
    if mcp.exists() {
        binaries.push(("ttm-mcp", "ttm-mcp", mcp));
    }
    let work = tempfile::tempdir()?;
    let mut attested = true;
    for (package, bin, installed) in binaries {
        let archive_name = archive_name(package, TARGET);
        let asset = find_asset(&release, &archive_name)
            .ok_or_else(|| anyhow!("release {} has no {archive_name}", release.tag_name))?;
        let bytes = download(&http, &asset.browser_download_url).await?;
        let checksum = match find_asset(&release, &format!("{archive_name}.sha256")) {
            Some(sum) => {
                String::from_utf8_lossy(&download(&http, &sum.browser_download_url).await?).to_string()
            }
            None => bail!(
                "release {} has no checksum for {archive_name}; refusing to install",
                release.tag_name
            ),
        };
        verify_sha256(&bytes, &checksum)
            .with_context(|| format!("{archive_name} failed the SHA-256 check"))?;
        let archive = work.path().join(&archive_name);
        std::fs::write(&archive, &bytes)?;
        attested &= verify_attestation(&archive, &args.repo)?;
        let extracted = work.path().join(package);
        std::fs::create_dir_all(&extracted)?;
        extract(&archive, &extracted)?;
        let new_bin = find_file(&extracted, &exe_name(bin))
            .ok_or_else(|| anyhow!("{archive_name} does not contain {}", exe_name(bin)))?;
        replace_exe(&installed, &new_bin)?;
        out.updated.push(installed.display().to_string());
    }
    out.attestation = if attested {
        "verified"
    } else {
        "skipped (install the gh CLI to verify)"
    };
    emit(&app, &out, |_ui| {
        println!("Updated to ttm {latest}: {}", out.updated.join(", "));
        println!("SHA-256 verified; attestation {}.", out.attestation);
    });
    Ok(ExitCode::SUCCESS)
}

async fn fetch_release(http: &reqwest::Client, repo: &str, version: Option<&str>) -> Result<Release> {
    let url = match version {
        Some(v) => format!(
            "https://api.github.com/repos/{repo}/releases/tags/v{}",
            v.trim_start_matches('v')
        ),
        None => format!("https://api.github.com/repos/{repo}/releases/latest"),
    };
    let response = http
        .get(&url)
        .header("accept", "application/vnd.github+json")
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("GitHub answered {} for {url}", response.status());
    }
    Ok(response.json().await?)
}

async fn download(http: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let response = http.get(url).send().await?;
    if !response.status().is_success() {
        bail!("download failed ({}) for {url}", response.status());
    }
    Ok(response.bytes().await?.to_vec())
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (semver::Version::parse(candidate), semver::Version::parse(current)) {
        (Ok(c), Ok(cur)) => c > cur,
        _ => false,
    }
}

/// cargo-dist names archives `<package>-<target>.tar.xz` (`.zip` on Windows).
pub fn archive_name(package: &str, target: &str) -> String {
    let ext = if target.contains("windows") {
        "zip"
    } else {
        "tar.xz"
    };
    format!("{package}-{target}.{ext}")
}

pub fn find_asset<'a>(release: &'a Release, name: &str) -> Option<&'a Asset> {
    release.assets.iter().find(|a| a.name == name)
}

fn exe_name(bin: &str) -> String {
    format!("{bin}{}", std::env::consts::EXE_SUFFIX)
}

/// `<hex>  <file>` (or just `<hex>`) must match the SHA-256 of `bytes`.
pub fn verify_sha256(bytes: &[u8], checksum_file: &str) -> Result<()> {
    let expected = checksum_file
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("empty checksum file"))?
        .to_ascii_lowercase();
    let actual: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    if actual != expected {
        bail!("expected {expected}, got {actual}");
    }
    Ok(())
}

/// `gh attestation verify` when `gh` exists. Returns whether it ran; a failed verification is an error.
fn verify_attestation(archive: &Path, repo: &str) -> Result<bool> {
    let Some(gh) = which("gh") else { return Ok(false) };
    let status = std::process::Command::new(gh)
        .args(["attestation", "verify"])
        .arg(archive)
        .args(["--repo", repo])
        .stdout(std::process::Stdio::null())
        .status()
        .context("running gh attestation verify")?;
    if !status.success() {
        bail!("GitHub attestation verification failed for {}", archive.display());
    }
    Ok(true)
}

/// Uses the system `tar` (bsdtar on macOS and Windows 10+, GNU tar with xz on Linux), which reads both formats.
fn extract(archive: &Path, into: &Path) -> Result<()> {
    let status = std::process::Command::new("tar")
        .arg("-xf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .status()
        .context("running tar to unpack the update")?;
    if !status.success() {
        bail!("tar could not unpack {}", archive.display());
    }
    Ok(())
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, name) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(path);
        }
    }
    None
}

/// Swaps the file in place. A running executable can be renamed on every platform, so the old one is moved aside first.
pub fn replace_exe(installed: &Path, new_bin: &Path) -> Result<()> {
    let dir = installed.parent().unwrap_or_else(|| Path::new("."));
    let staged = dir.join(format!(
        ".{}.new",
        installed.file_name().and_then(|n| n.to_str()).unwrap_or("ttm")
    ));
    std::fs::copy(new_bin, &staged).with_context(|| format!("cannot write into {}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    let old = installed.with_extension("old");
    let _ = std::fs::remove_file(&old);
    if installed.exists() {
        std::fs::rename(installed, &old)?;
    }
    if let Err(err) = std::fs::rename(&staged, installed) {
        let _ = std::fs::rename(&old, installed);
        return Err(err.into());
    }
    // On Windows the running .old file cannot be deleted yet; it is removed by the next update.
    let _ = std::fs::remove_file(&old);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn archive_names_follow_cargo_dist() {
        assert_eq!(
            archive_name("ttm-cli", "aarch64-apple-darwin"),
            "ttm-cli-aarch64-apple-darwin.tar.xz"
        );
        assert_eq!(
            archive_name("ttm-mcp", "x86_64-pc-windows-msvc"),
            "ttm-mcp-x86_64-pc-windows-msvc.zip"
        );
        assert!(!TARGET.is_empty());
    }

    #[test]
    fn sha256_check() {
        let sum = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824  hello.tar.xz\n";
        verify_sha256(b"hello", sum).unwrap();
        verify_sha256(b"hello", &sum.to_uppercase()).unwrap();
        assert!(verify_sha256(b"hellO", sum).is_err());
        assert!(verify_sha256(b"hello", "").is_err());
    }

    #[test]
    fn release_json_and_asset_lookup() {
        let release: Release = serde_json::from_value(serde_json::json!({
            "tag_name": "v0.2.0",
            "assets": [{"name": "ttm-cli-x86_64-unknown-linux-gnu.tar.xz", "browser_download_url": "https://x/a"}]
        }))
        .unwrap();
        assert!(find_asset(&release, "ttm-cli-x86_64-unknown-linux-gnu.tar.xz").is_some());
        assert!(find_asset(&release, "nope").is_none());
    }

    #[test]
    fn replace_exe_swaps_content() {
        let dir = tempfile::tempdir().unwrap();
        let installed = dir.path().join("ttm");
        let new_bin = dir.path().join("download");
        std::fs::write(&installed, "old").unwrap();
        std::fs::write(&new_bin, "new").unwrap();
        replace_exe(&installed, &new_bin).unwrap();
        assert_eq!(std::fs::read_to_string(&installed).unwrap(), "new");
    }
}
