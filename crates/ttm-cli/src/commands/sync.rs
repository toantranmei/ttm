//! `ttm sync [--check] [--resolve]`: the web is the source of truth.
//!
//! 1. Download the latest plan bundle.
//! 2. Local edits (`.ttm/plan` vs `.ttm/base`) go up as a change request against `synced_rev`, never as overwrites.
//! 3. Server changes are merged into `.ttm/plan` (file level; files edited locally keep the local version until the
//!    change request is decided) and `.ttm/base` becomes the server version.
//! 4. A `409` with conflicts stops, or with `--resolve` runs a local agent in a separate worktree to merge base, local
//!    and server versions and sends the result as a new change request.

use super::project::linked;
use crate::app::App;
use crate::cli::SyncArgs;
use crate::output::emit;
use crate::util::which;
use anyhow::{Context as _, Result, anyhow, bail};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::factory::{ChangeRequestOutcome, Conflict, bundle_path};
use ttm_core::merge3::{self, Changes, Files};
use ttm_core::project_file::{Linked, read_tree, write_tree};
use ttm_core::worktree;

#[derive(Debug, Serialize, Default)]
struct SyncOutput {
    local_rev: i64,
    server_rev: i64,
    local_changes: Vec<String>,
    server_changes: Vec<String>,
    change_request: Option<String>,
    /// Files the server ignored, with the reason (deletions, `plan/index.md`, unreadable files).
    skipped: Vec<String>,
    /// The server found nothing to change in the local edits.
    nothing_to_send: bool,
    conflicts: Vec<Conflict>,
    in_sync: bool,
}

/// Local plan changes keyed by their bundle path (`plan/tasks/A.md`), as change requests expect.
pub fn as_bundle(changes: &Changes) -> Changes {
    changes.iter().map(|(k, v)| (bundle_path(k), v.clone())).collect()
}

pub async fn run(app: App, args: SyncArgs) -> Result<ExitCode> {
    let mut linked = linked()?;
    let (client, _) = app.api().await?;
    let bundle = client
        .export_plan(&linked.file.project_id, None)
        .await
        .context("downloading the plan")?;
    let remote = bundle.plan_files();
    let base = read_tree(&linked.base_dir())?;
    let local = read_tree(&linked.plan_dir())?;
    let local_changes = merge3::changes(&base, &local);
    let server_changes = merge3::changes(&base, &remote);

    let mut out = SyncOutput {
        local_rev: linked.file.synced_rev,
        server_rev: bundle.rev,
        local_changes: local_changes.keys().cloned().collect(),
        server_changes: server_changes.keys().cloned().collect(),
        ..SyncOutput::default()
    };
    out.in_sync =
        local_changes.is_empty() && server_changes.is_empty() && bundle.rev == linked.file.synced_rev;

    if args.check {
        emit(&app, &out, |_ui| print_check(&out));
        return Ok(if out.in_sync {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }

    // A change request still waiting: do not send the same edits twice.
    let mut send = !local_changes.is_empty();
    if let Some(pending) = linked.file.pending_change_request.clone() {
        match client.change_request(&pending).await {
            Ok(cr) if cr.status == "pending" => {
                send = false;
                out.change_request = Some(pending);
            }
            _ => linked.file.pending_change_request = None,
        }
    }

    if send {
        match client
            .create_change_request(
                &linked.file.project_id,
                linked.file.synced_rev,
                &as_bundle(&local_changes),
            )
            .await?
        {
            ChangeRequestOutcome::Created(cr) => {
                out.skipped = cr.skipped.clone();
                if cr.is_empty() {
                    out.nothing_to_send = true;
                } else {
                    linked.file.pending_change_request = (cr.status == "pending").then(|| cr.id.clone());
                    out.change_request = Some(cr.id);
                }
            }
            ChangeRequestOutcome::Conflicts(conflicts) => {
                if !args.resolve {
                    out.conflicts = conflicts;
                    emit(&app, &out, |_ui| {
                        eprintln!("Conflicts with the server (both sides changed the same fields):");
                        for c in &out.conflicts {
                            eprintln!(
                                "  {} {}: local {} / server {}",
                                c.task, c.field, c.local, c.server
                            );
                        }
                        eprintln!(
                            "Run `ttm sync --resolve` to merge them with a local agent, or edit .ttm/plan and retry."
                        );
                    });
                    return Ok(ExitCode::FAILURE);
                }
                let cr = resolve(&app, &client, &linked, &base, &local, &remote, bundle.rev).await?;
                linked.file.pending_change_request = Some(cr.clone());
                out.change_request = Some(cr);
            }
        }
    }

    // Bring server changes in, keeping local edits that are not decided yet. When the server found nothing to change
    // in them, the plan is up to date: take the server version and keep a copy of what is replaced.
    let next = if out.nothing_to_send {
        let dropped: Files = local_changes
            .keys()
            .filter_map(|k| local.get(k).map(|v| (k.clone(), v.clone())))
            .collect();
        if !dropped.is_empty() {
            write_tree(&linked.cache_dir().join("dropped"), &dropped)?;
        }
        remote.clone()
    } else {
        merge3::merge(&base, &local, &remote).files
    };
    write_tree(&linked.plan_dir(), &next)?;
    write_tree(&linked.base_dir(), &remote)?;
    linked.file.synced_rev = bundle.rev;
    linked.save()?;

    emit(&app, &out, |_ui| {
        println!(
            "Plan at rev {} ({} file(s) updated from the server).",
            bundle.rev,
            out.server_changes.len()
        );
        if let Some(cr) = &out.change_request {
            println!("Local edits sent as change request {cr}; they apply once approved on the web.");
        }
        if out.nothing_to_send {
            println!(
                "Up to date: the server found nothing to change in the local edits (copies in .ttm/cache/dropped)."
            );
        }
        for s in &out.skipped {
            eprintln!("skipped: {s}");
        }
    });
    Ok(ExitCode::SUCCESS)
}

fn print_check(out: &SyncOutput) {
    if out.in_sync {
        println!("Plan in sync (rev {}).", out.server_rev);
        return;
    }
    println!(
        "Plan out of sync: local rev {}, server rev {}.",
        out.local_rev, out.server_rev
    );
    for f in &out.local_changes {
        println!("  local:  {f}");
    }
    for f in &out.server_changes {
        println!("  server: {f}");
    }
}

/// Instruction appended to the server's prompt.
pub const RESOLVE_INSTRUCTIONS: &str = "\n\nThe three versions of the plan are in .ttm/resolve/base, .ttm/resolve/local and \
.ttm/resolve/server (same relative paths). Write the merged version of every file that differs into \
.ttm/resolve/merged with the same relative path. Keep both sides' intent; do not edit anything else.";

/// Creates `ttm/resolve-<rev>` in its own worktree, lets `claude -p` merge, and sends the merge as a change request.
async fn resolve(
    app: &App,
    client: &Client,
    linked: &Linked,
    base: &Files,
    local: &Files,
    remote: &Files,
    server_rev: i64,
) -> Result<String> {
    let claude = which("claude")
        .ok_or_else(|| anyhow!("`claude` (Claude Code) is needed for --resolve and is not on PATH"))?;
    let root = app
        .ctx
        .config
        .worktree_root()
        .ok_or_else(|| anyhow!("no home directory for worktrees"))?;
    let branch = format!("ttm/resolve-{server_rev}");
    let path = worktree::worktree_path(&root, &linked.file.key, &branch);
    let main = linked
        .file
        .default_branch
        .clone()
        .unwrap_or_else(|| worktree::default_branch(&linked.root));
    worktree::add(&linked.root, &path, &branch, &main).context("creating the resolve worktree")?;

    let result = async {
        let dir = path.join(".ttm").join("resolve");
        let paths: Vec<String> = merge3::changes(base, local).into_keys().collect();
        let pick = |files: &Files| -> BTreeMap<String, String> {
            paths
                .iter()
                .filter_map(|p| files.get(p).map(|v| (p.clone(), v.clone())))
                .collect()
        };
        write_tree(&dir.join("base"), &pick(base))?;
        write_tree(&dir.join("local"), &pick(local))?;
        write_tree(&dir.join("server"), &pick(remote))?;
        let prompt = client
            .resolve_prompt(&linked.file.project_id, server_rev)
            .await
            .unwrap_or_default();
        let prompt = if prompt.is_empty() {
            "Merge the conflicting plan changes.".to_string()
        } else {
            prompt
        };
        run_agent(&claude, &path, &format!("{prompt}{RESOLVE_INSTRUCTIONS}"))?;
        let merged = read_tree(&dir.join("merged"))?;
        if merged.is_empty() {
            bail!(
                "the agent did not write any merged file in {}",
                dir.join("merged").display()
            );
        }
        let files: Changes = merged
            .into_iter()
            .map(|(k, v)| (bundle_path(&k), Some(v)))
            .collect();
        match client
            .create_change_request(&linked.file.project_id, server_rev, &files)
            .await?
        {
            ChangeRequestOutcome::Created(cr) if cr.is_empty() => {
                bail!("the merged plan changes nothing on the server")
            }
            ChangeRequestOutcome::Created(cr) => Ok(cr.id),
            ChangeRequestOutcome::Conflicts(c) => bail!(
                "the merged plan still conflicts on {} field(s); run sync again",
                c.len()
            ),
        }
    }
    .await;
    if let Err(err) = worktree::remove(&linked.root, &path) {
        tracing::warn!("could not remove {}: {err}", path.display());
    }
    result
}

fn run_agent(claude: &Path, cwd: &Path, prompt: &str) -> Result<()> {
    let status = std::process::Command::new(claude)
        .current_dir(cwd)
        .args([
            "-p",
            prompt,
            "--output-format",
            "json",
            "--allowedTools",
            "Read,Write,Edit,Glob,Grep",
        ])
        .stdout(std::process::Stdio::null())
        .status()
        .context("running the resolve agent")?;
    if !status.success() {
        bail!("the resolve agent exited with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_request_paths_are_bundle_paths() {
        let mut c = Changes::new();
        c.insert("tasks/AUR-1.md".into(), Some("x".into()));
        c.insert("index.md".into(), None);
        let b = as_bundle(&c);
        assert!(b.contains_key("plan/tasks/AUR-1.md") && b.contains_key("plan/index.md"));
    }
}
