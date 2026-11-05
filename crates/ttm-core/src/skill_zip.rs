//! Skill bundles: a directory with a `SKILL.md`, packed into a **deterministic** zip so the same content always has
//! the same SHA-256 on the CLI (`ttm skill push`), on the server (S3 `skill_bundle::pack`) and on the runner.
//!
//! Byte format (shared contract with the server; see `tests/fixtures/skill-vector/`):
//! - entries sorted by path (UTF-8 bytes), files only (no directory entries), `/` separators;
//! - method **Stored** (0), no data descriptor, general purpose flags `0x0800` (UTF-8 names);
//! - DOS time `0x0000`, date `0x0021` (1980-01-01 00:00);
//! - "version made by" `0x031E` (Unix, 3.0), "version needed" `0x000A`; external attributes `(0o100000 | mode) << 16`
//!   with mode `0o755` when the file is executable, else `0o644`;
//! - no extra fields, no file or archive comment, no ZIP64 (bundles are ≤ 5 MB and ≤ 200 files).

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Largest unpacked bundle.
pub const MAX_SKILL_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_FILES: usize = 200;
pub const MAX_PATH: usize = 200;
/// Largest compression ratio accepted when reading a zip (zip bombs).
pub const MAX_RATIO: u64 = 100;

/// One file of a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFile {
    pub path: String,
    pub bytes: Vec<u8>,
    pub executable: bool,
}

/// Front matter of `SKILL.md`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrontMatter {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub executable: bool,
}

/// What `check` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub files: Vec<ManifestFile>,
    pub front_matter: FrontMatter,
    pub total_size: u64,
}

/// Why a bundle is refused; `rule` matches the server's (`skill_md_missing`, `front_matter`, `path_unsafe`,
/// `too_large`, `too_many_files`, `zip_invalid`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message} ({rule})")]
pub struct BundleError {
    pub rule: &'static str,
    pub message: String,
}

fn err(rule: &'static str, message: impl Into<String>) -> BundleError {
    BundleError {
        rule,
        message: message.into(),
    }
}

/// Whether `path` is a safe relative bundle path.
pub fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains('\0')
        && !path.contains(':')
        && path
            .split('/')
            .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
        && !path.split('/').any(|seg| seg == ".git")
}

/// Checks a bundle like the server does.
pub fn check(files: &[BundleFile]) -> Result<Manifest, BundleError> {
    if files.len() > MAX_FILES {
        return Err(err(
            "too_many_files",
            format!("{} files (at most {MAX_FILES})", files.len()),
        ));
    }
    let mut total = 0u64;
    let mut out = Vec::with_capacity(files.len());
    for f in files {
        if !safe_path(&f.path) {
            return Err(err("path_unsafe", format!("unsafe path `{}`", f.path)));
        }
        total += f.bytes.len() as u64;
        out.push(ManifestFile {
            path: f.path.clone(),
            size: f.bytes.len() as u64,
            sha256: hex(&Sha256::digest(&f.bytes)),
            executable: f.executable,
        });
    }
    if total > MAX_SKILL_BYTES {
        return Err(err(
            "too_large",
            format!("{total} bytes (at most {MAX_SKILL_BYTES})"),
        ));
    }
    let skill_md = files
        .iter()
        .find(|f| f.path == "SKILL.md")
        .ok_or_else(|| err("skill_md_missing", "no SKILL.md at the root of the skill"))?;
    let front_matter = front_matter(&String::from_utf8_lossy(&skill_md.bytes))?;
    out.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    Ok(Manifest {
        files: out,
        front_matter,
        total_size: total,
    })
}

/// Parses the YAML front matter of a `SKILL.md`: `name` (≤ 64), `description` (≤ 1024), `ttm.tags` (≤ 12,
/// `^[a-z0-9][a-z0-9-]{0,31}$`).
pub fn front_matter(text: &str) -> Result<FrontMatter, BundleError> {
    let text = text.trim_start_matches('\u{feff}');
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
        .ok_or_else(|| err("front_matter", "SKILL.md must start with a `---` front matter"))?;
    let end = rest.find("\n---").ok_or_else(|| {
        err(
            "front_matter",
            "the front matter of SKILL.md is not closed by `---`",
        )
    })?;
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&rest[..end])
        .map_err(|e| err("front_matter", format!("front matter is not valid YAML: {e}")))?;
    let get = |k: &str| {
        yaml.get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .unwrap_or("")
            .to_string()
    };
    let name = get("name");
    let description = get("description");
    if name.is_empty() || name.chars().count() > 64 {
        return Err(err("front_matter", "`name` is required (at most 64 characters)"));
    }
    if description.is_empty() || description.chars().count() > 1024 {
        return Err(err(
            "front_matter",
            "`description` is required (at most 1024 characters)",
        ));
    }
    let tags: Vec<String> = yaml
        .get("ttm")
        .and_then(|t| t.get("tags"))
        .and_then(|t| t.as_sequence())
        .map(|s| s.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    if tags.len() > 12 || !tags.iter().all(|t| valid_tag(t)) {
        return Err(err(
            "front_matter",
            "`ttm.tags`: at most 12 tags like `testing` or `open-pr`",
        ));
    }
    Ok(FrontMatter {
        name,
        description,
        tags,
    })
}

fn valid_tag(tag: &str) -> bool {
    let b = tag.as_bytes();
    !b.is_empty()
        && b.len() <= 32
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// CRC-32 (IEEE) of `bytes`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for b in bytes {
        crc = table[((crc ^ *b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Lower-case hex.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The deterministic zip of `files` and its SHA-256 (hex). Input order does not matter.
pub fn pack(files: &[BundleFile]) -> (Vec<u8>, String) {
    let mut sorted: Vec<&BundleFile> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    for f in &sorted {
        let name = f.path.as_bytes();
        let crc = crc32(&f.bytes);
        let size = f.bytes.len() as u32;
        let offset = out.len() as u32;
        // Local file header.
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 10); // version needed
        le16(&mut out, 0x0800); // UTF-8 names
        le16(&mut out, 0); // Stored
        le16(&mut out, 0x0000); // time 00:00
        le16(&mut out, 0x0021); // date 1980-01-01
        le32(&mut out, crc);
        le32(&mut out, size);
        le32(&mut out, size);
        le16(&mut out, name.len() as u16);
        le16(&mut out, 0); // extra
        out.extend_from_slice(name);
        out.extend_from_slice(&f.bytes);
        // Central directory entry.
        let mode: u32 = 0o100000 | if f.executable { 0o755 } else { 0o644 };
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 0x031E); // made by: Unix, 3.0
        le16(&mut central, 10);
        le16(&mut central, 0x0800);
        le16(&mut central, 0);
        le16(&mut central, 0x0000);
        le16(&mut central, 0x0021);
        le32(&mut central, crc);
        le32(&mut central, size);
        le32(&mut central, size);
        le16(&mut central, name.len() as u16);
        le16(&mut central, 0); // extra
        le16(&mut central, 0); // comment
        le16(&mut central, 0); // disk
        le16(&mut central, 0); // internal attributes
        le32(&mut central, mode << 16);
        le32(&mut central, offset);
        central.extend_from_slice(name);
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, sorted.len() as u16);
    le16(&mut out, sorted.len() as u16);
    le32(&mut out, cd_size);
    le32(&mut out, cd_offset);
    le16(&mut out, 0);
    let sha = hex(&Sha256::digest(&out));
    (out, sha)
}

/// One `.gitignore` pattern (the common subset: `#` comments, `!` is not supported and ignored, trailing `/` for
/// directories, leading `/` anchors, `*` and `?` inside a segment, `**/` prefix).
#[derive(Debug, Clone)]
struct Ignore {
    pattern: String,
    anchored: bool,
    dir_only: bool,
}

fn parse_gitignore(text: &str) -> Vec<Ignore> {
    text.lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with('!'))
        .map(|l| {
            let dir_only = l.ends_with('/');
            let l = l.trim_end_matches('/');
            let l = l.strip_prefix("**/").unwrap_or(l);
            let anchored = l.starts_with('/') || l.contains('/');
            Ignore {
                pattern: l.trim_start_matches('/').to_string(),
                anchored,
                dir_only,
            }
        })
        .collect()
}

fn glob(pattern: &[u8], text: &[u8]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            glob(&pattern[1..], text) || (!text.is_empty() && text[0] != b'/' && glob(pattern, &text[1..]))
        }
        (Some(b'?'), Some(c)) if *c != b'/' => glob(&pattern[1..], &text[1..]),
        (Some(p), Some(c)) if p == c => glob(&pattern[1..], &text[1..]),
        _ => false,
    }
}

fn ignored(rules: &[Ignore], rel: &str, is_dir: bool) -> bool {
    rules.iter().any(|r| {
        if r.dir_only && !is_dir {
            return false;
        }
        if r.anchored {
            glob(r.pattern.as_bytes(), rel.as_bytes())
        } else {
            let name = rel.rsplit('/').next().unwrap_or(rel);
            glob(r.pattern.as_bytes(), name.as_bytes())
        }
    })
}

/// Reads a skill directory: skips `.git/`, `.DS_Store` and what the directory's `.gitignore` lists (the
/// `.gitignore` itself is kept); never follows symlinks (they are refused).
pub fn read_dir(dir: &Path) -> Result<Vec<BundleFile>, BundleError> {
    let rules = std::fs::read_to_string(dir.join(".gitignore"))
        .map(|t| parse_gitignore(&t))
        .unwrap_or_default();
    let mut files = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current)
            .map_err(|e| err("zip_invalid", format!("cannot read {}: {e}", current.display())))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path
                .strip_prefix(dir)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = std::fs::symlink_metadata(&path)
                .map_err(|e| err("zip_invalid", format!("cannot read {rel}: {e}")))?;
            if meta.file_type().is_symlink() {
                return Err(err("path_unsafe", format!("`{rel}` is a symbolic link")));
            }
            if name == ".git" || name == ".DS_Store" || ignored(&rules, &rel, meta.is_dir()) {
                continue;
            }
            if meta.is_dir() {
                stack.push(path);
                continue;
            }
            let bytes =
                std::fs::read(&path).map_err(|e| err("zip_invalid", format!("cannot read {rel}: {e}")))?;
            files.push(BundleFile {
                path: rel,
                bytes,
                executable: is_executable(&meta),
            });
        }
    }
    files.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    Ok(files)
}

#[cfg(unix)]
fn is_executable(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_meta: &std::fs::Metadata) -> bool {
    false
}

/// `read_dir` + `check` + `pack`: the zip, its SHA-256 and the manifest.
pub fn pack_dir(dir: &Path) -> Result<(Vec<u8>, String, Manifest), BundleError> {
    let files = read_dir(dir)?;
    let manifest = check(&files)?;
    let (zip, sha) = pack(&files);
    Ok((zip, sha, manifest))
}

/// Reads a zip (any compression) into files, refusing unsafe paths, symlinks, too many or too large files and zip
/// bombs. A single top-level directory shared by every file (zips from GitHub) is removed.
pub fn from_zip(bytes: &[u8]) -> Result<Vec<BundleFile>, BundleError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| err("zip_invalid", format!("not a zip: {e}")))?;
    let mut files = Vec::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| err("zip_invalid", format!("entry {i}: {e}")))?;
        let name = entry.name().replace('\\', "/");
        if entry.is_dir() {
            continue;
        }
        let mode = entry.unix_mode().unwrap_or(0o644);
        if mode & 0o170000 == 0o120000 {
            return Err(err("path_unsafe", format!("`{name}` is a symbolic link")));
        }
        if !safe_path(&name) {
            return Err(err("path_unsafe", format!("unsafe path `{name}`")));
        }
        let declared = entry.size();
        if entry.compressed_size() > 0 && declared / entry.compressed_size().max(1) > MAX_RATIO {
            return Err(err(
                "too_large",
                format!("`{name}` expands more than {MAX_RATIO}×"),
            ));
        }
        total += declared;
        if total > MAX_SKILL_BYTES {
            return Err(err(
                "too_large",
                format!("more than {MAX_SKILL_BYTES} bytes unpacked"),
            ));
        }
        let mut data = Vec::with_capacity(declared as usize);
        entry
            .by_ref()
            .take(MAX_SKILL_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|e| err("zip_invalid", format!("`{name}`: {e}")))?;
        if data.len() as u64 != declared {
            return Err(err("zip_invalid", format!("`{name}`: size does not match")));
        }
        files.push(BundleFile {
            path: name,
            bytes: data,
            executable: mode & 0o111 != 0,
        });
        if files.len() > MAX_FILES {
            return Err(err("too_many_files", format!("more than {MAX_FILES} files")));
        }
    }
    // Strip a shared top-level directory.
    let first: Option<String> = files
        .first()
        .and_then(|f| f.path.split_once('/').map(|(d, _)| d.to_string()));
    if let Some(root) = first
        && files.iter().all(|f| f.path.starts_with(&format!("{root}/")))
    {
        for f in &mut files {
            f.path = f.path[root.len() + 1..].to_string();
        }
    }
    files.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    Ok(files)
}

/// Unpacks a bundle into `dest` (created), after the same checks as [`from_zip`] and [`check`]. Returns the files
/// written, relative to `dest`.
pub fn safe_extract(bytes: &[u8], dest: &Path) -> Result<Vec<String>, BundleError> {
    let files = from_zip(bytes)?;
    check(&files)?;
    std::fs::create_dir_all(dest)
        .map_err(|e| err("zip_invalid", format!("cannot create {}: {e}", dest.display())))?;
    // Compare real paths: a symlinked directory inside `dest` must not lead the files elsewhere.
    let root = dest
        .canonicalize()
        .map_err(|e| err("zip_invalid", format!("cannot resolve {}: {e}", dest.display())))?;
    let mut written = Vec::new();
    for f in &files {
        let target = root.join(&f.path);
        if !target.starts_with(&root) {
            return Err(err("path_unsafe", format!("unsafe path `{}`", f.path)));
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| err("zip_invalid", e.to_string()))?;
            let real = parent
                .canonicalize()
                .map_err(|e| err("zip_invalid", e.to_string()))?;
            if !real.starts_with(&root) {
                return Err(err(
                    "path_unsafe",
                    format!("`{}` leads outside the skill directory", f.path),
                ));
            }
        }
        if std::fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(err("path_unsafe", format!("`{}` is a symbolic link", f.path)));
        }
        std::fs::write(&target, &f.bytes).map_err(|e| err("zip_invalid", e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if f.executable { 0o755 } else { 0o644 };
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(mode));
        }
        written.push(f.path.clone());
    }
    Ok(written)
}

/// Files of a bundle as a path → bytes map (diffs).
pub fn as_map(files: &[BundleFile]) -> BTreeMap<String, Vec<u8>> {
    files.iter().map(|f| (f.path.clone(), f.bytes.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str, executable: bool) -> BundleFile {
        BundleFile {
            path: path.into(),
            bytes: text.as_bytes().to_vec(),
            executable,
        }
    }

    const SKILL: &str =
        "---\nname: TDD\ndescription: Write the test first.\nttm:\n  tags: [testing]\n---\n# TDD\n";

    #[test]
    fn pack_is_deterministic_and_order_free() {
        let a = vec![
            file("SKILL.md", SKILL, false),
            file("scripts/run.sh", "#!/bin/sh\n", true),
        ];
        let b = vec![a[1].clone(), a[0].clone()];
        assert_eq!(pack(&a).1, pack(&b).1);
        let (zip, _) = pack(&a);
        let back = from_zip(&zip).unwrap();
        assert_eq!(back, a);
    }

    #[test]
    fn checks() {
        assert_eq!(
            check(&[file("README.md", "x", false)]).unwrap_err().rule,
            "skill_md_missing"
        );
        assert_eq!(
            check(&[file("../x", "x", false)]).unwrap_err().rule,
            "path_unsafe"
        );
        assert_eq!(
            check(&[file("SKILL.md", "---\nname: A\n---\n", false)])
                .unwrap_err()
                .rule,
            "front_matter"
        );
        let m = check(&[file("SKILL.md", SKILL, false)]).unwrap();
        assert_eq!(m.front_matter.tags, ["testing"]);
        assert!(!safe_path(".git/config") && !safe_path("a//b") && !safe_path("/abs") && safe_path("a/b.md"));
    }

    #[test]
    fn crc32_known_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn gitignore_subset() {
        let rules = parse_gitignore("# c\nnode_modules/\n*.log\n/dist\n");
        assert!(ignored(&rules, "node_modules", true));
        assert!(!ignored(&rules, "node_modules", false));
        assert!(ignored(&rules, "a/b/debug.log", false));
        assert!(ignored(&rules, "dist", true));
        assert!(!ignored(&rules, "src/dist", true));
    }
}
