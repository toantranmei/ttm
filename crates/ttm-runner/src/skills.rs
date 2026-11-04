//! Skills of a job: download (or reuse from the cache by SHA-256), check, unpack into
//! `<worktree>/.claude/skills/ttm-<slug>/`, keep them out of git (`info/exclude`), and remove them after the run.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use ttm_api::factory::{JobSkill, JobSpec, RunEvent};
use ttm_core::skill_zip;
use ttm_core::worktree::git;

/// Error code of a skill whose bytes do not match the job's SHA-256 (the job is refused).
pub const SKILL_HASH_MISMATCH: &str = "skill_hash_mismatch";
/// Line added once to `<git-common-dir>/info/exclude`.
pub const EXCLUDE_LINE: &str = "/.claude/skills/ttm-*/";
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// The download does not match `sha256` (or is larger than announced): refuse the job.
    #[error("skill {slug}: the download does not match its SHA-256")]
    HashMismatch { slug: String },
    #[error("skill {slug}: {message}")]
    Other { slug: String, message: String },
}

impl SkillError {
    pub fn code(&self) -> &'static str {
        match self {
            SkillError::HashMismatch { .. } => SKILL_HASH_MISMATCH,
            SkillError::Other { .. } => "skill_install_failed",
        }
    }
}

/// The skills installed for a run; removed by [`Installed::cleanup`] or when dropped.
#[derive(Debug, Default)]
pub struct Installed {
    pub dirs: Vec<PathBuf>,
    /// `ttm-tdd v3`.
    pub names: Vec<String>,
}

impl Installed {
    pub fn cleanup(&mut self) {
        for dir in self.dirs.drain(..) {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

impl Drop for Installed {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// A slug safe as a directory name.
pub fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 64
        && slug
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    skill_zip::hex(&sha2::Sha256::digest(bytes))
}

/// Bytes of a skill: the cached zip when its hash matches, else a download capped at `min(size, 5 MB)`.
pub async fn fetch(
    http: &reqwest::Client,
    skill: &JobSkill,
    cache_dir: &Path,
) -> Result<Vec<u8>, SkillError> {
    let want = skill.sha256.to_ascii_lowercase();
    let cached = cache_dir.join("skills").join(format!("{want}.zip"));
    if let Ok(bytes) = std::fs::read(&cached)
        && sha256_hex(&bytes) == want
    {
        return Ok(bytes);
    }
    let other = |message: String| SkillError::Other {
        slug: skill.slug.clone(),
        message,
    };
    let cap = if skill.size > 0 {
        skill.size.min(skill_zip::MAX_SKILL_BYTES)
    } else {
        skill_zip::MAX_SKILL_BYTES
    } as usize;
    let mut response = http
        .get(&skill.url)
        .timeout(DOWNLOAD_TIMEOUT)
        .send()
        .await
        .map_err(|e| other(format!("download failed: {e}")))?;
    if !response.status().is_success() {
        return Err(other(format!("download failed: HTTP {}", response.status())));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| other(format!("download failed: {e}")))?
    {
        bytes.extend_from_slice(&chunk);
        if bytes.len() > cap {
            return Err(SkillError::HashMismatch {
                slug: skill.slug.clone(),
            });
        }
    }
    if sha256_hex(&bytes) != want {
        return Err(SkillError::HashMismatch {
            slug: skill.slug.clone(),
        });
    }
    if let Some(parent) = cached.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cached, &bytes);
    Ok(bytes)
}

/// Adds [`EXCLUDE_LINE`] to `<git-common-dir>/info/exclude` unless it is there.
pub fn ensure_excluded(worktree: &Path) -> std::io::Result<()> {
    let Ok(common) = git(worktree, &["rev-parse", "--git-common-dir"]) else {
        return Ok(());
    };
    let common = PathBuf::from(common);
    let common = if common.is_absolute() {
        common
    } else {
        worktree.join(common)
    };
    let file = common.join("info").join("exclude");
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    if text.lines().any(|l| l.trim() == EXCLUDE_LINE) {
        return Ok(());
    }
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = text;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(EXCLUDE_LINE);
    out.push('\n');
    std::fs::write(&file, out)
}

/// Whether git tracks anything under `rel` in the worktree.
fn tracked(worktree: &Path, rel: &str) -> bool {
    git(worktree, &["ls-files", "--", rel]).is_ok_and(|out| !out.is_empty())
}

/// Refuses `rel` (`.claude/skills/ttm-<slug>`) when any existing component is a symbolic link or resolves outside
/// the worktree, so a repository cannot point skill files at the rest of the machine.
pub fn inside_worktree(worktree: &Path, rel: &str) -> Result<(), String> {
    let root = worktree
        .canonicalize()
        .map_err(|e| format!("cannot resolve the worktree: {e}"))?;
    let mut current = root.clone();
    for part in rel.split('/') {
        current = current.join(part);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!(
                    "{} is a symbolic link; refusing to install skills there",
                    current.display()
                ));
            }
            Ok(_) => {
                let real = current.canonicalize().map_err(|e| e.to_string())?;
                if !real.starts_with(&root) {
                    return Err(format!("{} is outside the worktree", current.display()));
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

/// Installs every skill of `job` into the worktree. A hash mismatch refuses the job; a skill whose directory git
/// already tracks is skipped with a warning.
pub async fn install(
    http: &reqwest::Client,
    job: &JobSpec,
    worktree: &Path,
    cache_dir: &Path,
    events: &mpsc::UnboundedSender<RunEvent>,
) -> Result<Installed, SkillError> {
    let mut installed = Installed::default();
    if job.skills.is_empty() {
        return Ok(installed);
    }
    for skill in &job.skills {
        if !valid_slug(&skill.slug) {
            let _ = events.send(RunEvent::now(
                "log",
                format!("skill `{}` skipped: invalid name", skill.slug),
                None,
            ));
            continue;
        }
        let rel = format!(".claude/skills/ttm-{}", skill.slug);
        if tracked(worktree, &rel) {
            let _ = events.send(RunEvent::now(
                "log",
                format!(
                    "skill {} skipped: {rel} is tracked by git in this repository",
                    skill.slug
                ),
                None,
            ));
            continue;
        }
        let dest = worktree.join(&rel);
        inside_worktree(worktree, &rel).map_err(|message| SkillError::Other {
            slug: skill.slug.clone(),
            message,
        })?;
        let bytes = fetch(http, skill, cache_dir).await?;
        let _ = std::fs::remove_dir_all(&dest);
        skill_zip::safe_extract(&bytes, &dest).map_err(|e| SkillError::Other {
            slug: skill.slug.clone(),
            message: e.to_string(),
        })?;
        installed.dirs.push(dest);
        installed
            .names
            .push(format!("ttm-{} v{}", skill.slug, skill.version));
    }
    if !installed.dirs.is_empty() {
        ensure_excluded(worktree).map_err(|e| SkillError::Other {
            slug: "-".into(),
            message: format!("cannot update info/exclude: {e}"),
        })?;
        let _ = events.send(RunEvent::now(
            "log",
            format!("skills: {}", installed.names.join(", ")),
            None,
        ));
    }
    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert!(valid_slug("ttm-tdd"));
        assert!(!valid_slug("../x") && !valid_slug("") && !valid_slug("A"));
    }
}
