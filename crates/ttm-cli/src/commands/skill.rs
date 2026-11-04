//! `ttm skill list | push | pull | show | review [list|approve|reject] | import` (S3 skills).

use super::project::linked;
use crate::app::App;
use crate::cli::{SkillCommand, SkillReviewCommand};
use crate::output::emit;
use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::skills::{NewSkill, Skill, SkillImport};
use ttm_core::skill_zip;
use ttm_ui::{Callout, Cell, Col, Diff, Header, KeyValue, Status, Table, Token, Ui, UsageError, next_line};

const FEATURE: &str = "agent_v2";
const WHAT: &str = "skills";

/// `slug@3` → (`slug`, 3).
pub fn split_target(target: &str) -> Result<(String, i64)> {
    let (slug, v) = target.split_once('@').ok_or_else(|| {
        anyhow::Error::from(UsageError::new(
            "invalid_flag",
            format!("`{target}`: use <slug>@<version>"),
        ))
    })?;
    Ok((
        slug.to_string(),
        v.trim_start_matches('v')
            .parse()
            .map_err(|_| anyhow!("bad version in `{target}`"))?,
    ))
}

async fn find(client: &Client, ws: &str, slug: &str) -> Result<Option<Skill>> {
    Ok(client
        .skills(ws, None, None, true)
        .await?
        .into_iter()
        .find(|s| s.slug == slug || s.id == slug))
}

async fn must_find(client: &Client, ws: &str, slug: &str) -> Result<Skill> {
    find(client, ws, slug)
        .await?
        .ok_or_else(|| anyhow!("no skill `{slug}`; see `ttm skill list --builtin`"))
}

fn review_cell(state: Option<&str>) -> Cell {
    match state {
        Some("pending") => Cell::token("pending", Token::Yellow),
        Some("approved") => Cell::token("approved", Token::Teal),
        Some("exempt") => Cell::token("exempt", Token::Teal),
        Some("rejected") => Cell::token("rejected", Token::Coral),
        Some(other) => Cell::new(other),
        None => Cell::new("-"),
    }
}

fn human_size(n: u64) -> String {
    if n >= 1024 * 1024 {
        format!("{:.1} MB", n as f64 / (1024.0 * 1024.0))
    } else if n >= 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{n} B")
    }
}

pub async fn run(app: App, command: SkillCommand) -> Result<ExitCode> {
    let (client, _) = app.api_for(FEATURE, WHAT).await?;
    match command {
        SkillCommand::List { tag, builtin, ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let list = client.skills(&ws, None, tag.as_deref(), builtin).await?;
            emit(&app, &list, |ui| {
                ui.line(&format!("{} skill(s)", list.len()));
                let mut t = Table::new(&[
                    Col::new("SLUG"),
                    Col::new("NAME").flex(1).min(6),
                    Col::new("VERSION"),
                    Col::new("REVIEW"),
                    Col::new("TAGS").flex(1).min(4),
                ])
                .empty_text("No skills.");
                for s in &list {
                    let pending = s
                        .pending_versions
                        .as_ref()
                        .map(|p| {
                            p.as_array()
                                .map(Vec::len)
                                .unwrap_or_else(|| p.as_u64().unwrap_or(0) as usize)
                        })
                        .unwrap_or(0);
                    let version = match (s.current_version, s.latest_version) {
                        (Some(c), Some(l)) if l > c => format!("v{c} (v{l} new)"),
                        (Some(c), _) => format!("v{c}"),
                        (None, Some(l)) => format!("v{l} (none approved)"),
                        _ => "-".into(),
                    };
                    let review = if pending > 0 {
                        Cell::token(
                            format!("{} {pending} pending", ui.render(&Status::of_category("review"))),
                            Token::Yellow,
                        )
                    } else if s.builtin {
                        Cell::token("built-in", Token::Muted)
                    } else {
                        Cell::token("ok", Token::Teal)
                    };
                    t.push(vec![
                        Cell::token(s.slug.as_str(), Token::Ink),
                        Cell::new(s.name.as_str()),
                        Cell::new(version),
                        review,
                        Cell::new(s.tags.join(", ")),
                    ]);
                }
                ui.print(&t);
                ui.line(&next_line(ui.mode(), "ttm skill push <dir>"));
            });
        }
        SkillCommand::Push { dir, slug, note, ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let (zip, sha, manifest) = skill_zip::pack_dir(&dir).map_err(|e| {
                anyhow::Error::from(UsageError::new("invalid_file", e.message.clone()).field(e.rule))
            })?;
            let slug = slug.unwrap_or_else(|| {
                dir.canonicalize()
                    .ok()
                    .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "skill".into())
            });
            let mut t = Table::new(&[
                Col::new("FILE").flex(1).min(8),
                Col::new("SIZE").right(),
                Col::new(""),
            ]);
            for f in &manifest.files {
                t.push(vec![
                    Cell::new(f.path.as_str()),
                    Cell::new(human_size(f.size)),
                    Cell::token(if f.executable { "exec" } else { "" }, Token::Muted),
                ]);
            }
            if !app.json {
                app.ui.print(
                    &Header::new(format!("{slug} · {}", manifest.front_matter.name)).summary(format!(
                        "{} file(s) · {} · sha256 {}",
                        manifest.files.len(),
                        human_size(manifest.total_size),
                        &sha[..12]
                    )),
                );
                app.ui.print(&t);
            }
            let existing = find(&client, &ws, &slug).await?;
            let mut wizard = app.wizard();
            let question = match &existing {
                Some(_) => format!("Upload a new version of {slug}?"),
                None => format!("Create skill {slug} and upload version 1?"),
            };
            if !wizard.confirm(&question)? {
                bail!("cancelled");
            }
            let skill = match existing {
                Some(s) => s,
                None => {
                    client
                        .create_skill(&NewSkill {
                            workspace_id: ws.clone(),
                            slug: slug.clone(),
                            name: manifest.front_matter.name.clone(),
                            description: Some(manifest.front_matter.description.clone()),
                            tags: manifest.front_matter.tags.clone(),
                        })
                        .await?
                }
            };
            let version = match client.skill_push(&skill.id, zip, note.as_deref()).await {
                Ok(v) => v,
                Err(err) if err.rule() == Some("skill_unchanged") || err.code() == "skill_unchanged" => {
                    emit(
                        &app,
                        &serde_json::json!({ "unchanged": true, "sha256": sha }),
                        |ui| {
                            ui.print(
                                &Callout::info(format!("No changes: {slug} already has this content"))
                                    .detail(format!("sha256 {sha}")),
                            )
                        },
                    );
                    return Ok(ExitCode::SUCCESS);
                }
                Err(err) => return Err(err.into()),
            };
            emit(&app, &version, |ui| {
                let pending = version.review_status.as_deref() == Some("pending");
                let head = if pending {
                    Callout::success(format!("{slug} v{} pending review", version.version))
                        .next(format!("ttm skill review approve {slug}@{}", version.version))
                } else {
                    Callout::success(format!("{slug} v{} ready", version.version))
                        .next(format!("ttm agent skills add <agent> {slug}"))
                };
                ui.print(&head.detail(format!("sha256 {}", version.sha256.as_deref().unwrap_or(&sha))));
            });
        }
        SkillCommand::Pull {
            slug,
            version,
            dir,
            ws,
        } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let skill = must_find(&client, &ws, &slug).await?;
            let v = version
                .or(skill.current_version)
                .ok_or_else(|| anyhow!("{slug} has no approved version yet; pass --version"))?;
            let info = client.skill_version(&skill.id, v).await?;
            let bytes = client.skill_download(&skill.id, v).await?;
            let sha = skill_zip::hex(&<sha2::Sha256 as sha2::Digest>::digest(&bytes));
            if let Some(expected) = &info.sha256
                && expected != &sha
            {
                return Err(UsageError::new(
                    "skill_hash_mismatch",
                    format!("the download of {slug} v{v} has another SHA-256"),
                )
                .into());
            }
            let dest = dir.unwrap_or_else(|| default_pull_dir(&slug));
            let written = skill_zip::safe_extract(&bytes, &dest).map_err(|e| anyhow!("{e}"))?;
            emit(
                &app,
                &serde_json::json!({ "slug": slug, "version": v, "sha256": sha, "dir": dest, "files": written }),
                |ui| {
                    ui.print(
                        &Callout::success(format!("Pulled {slug} v{v} into {}", dest.display()))
                            .detail(format!("{} file(s) · sha256 {sha}", written.len()))
                            .next(format!("ttm skill push {}", dest.display())),
                    )
                },
            );
        }
        SkillCommand::Show { slug, version, ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let found = must_find(&client, &ws, &slug).await?;
            let skill = client.skill(&found.id).await?;
            let detail = match version {
                Some(v) => Some(client.skill_version(&skill.id, v).await?),
                None => None,
            };
            emit(&app, &skill, |ui| show(ui, &skill, detail.as_ref()));
        }
        SkillCommand::Review(cmd) => review(&app, &client, cmd).await?,
        SkillCommand::Import {
            git,
            path,
            git_ref,
            slug,
            ws,
        } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            if !app
                .wizard()
                .confirm(&format!("Import {path} of {git} ({git_ref})?"))?
            {
                bail!("cancelled");
            }
            let out = client
                .skill_import_git(&SkillImport {
                    workspace_id: ws,
                    url: git.clone(),
                    path,
                    git_ref,
                    slug,
                })
                .await?;
            emit(&app, &out, |ui| {
                let slug = out
                    .pointer("/skill/slug")
                    .or_else(|| out.get("slug"))
                    .and_then(Value::as_str)
                    .unwrap_or("skill");
                ui.print(
                    &Callout::success(format!("Imported {slug} from {git}"))
                        .next(format!("ttm skill show {slug}")),
                )
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `.ttm/skills/<slug>` in a linked repository, else `./<slug>`.
fn default_pull_dir(slug: &str) -> PathBuf {
    match linked() {
        Ok(l) => l.root.join(".ttm").join("skills").join(slug),
        Err(_) => Path::new(".").join(slug),
    }
}

fn show(ui: &Ui, s: &Skill, detail: Option<&ttm_api::skills::SkillVersion>) {
    ui.print(
        &Header::new(format!("{} · {}", s.slug, s.name)).summary(s.description.clone().unwrap_or_default()),
    );
    ui.print(
        &KeyValue::new()
            .row("id", s.id.as_str())
            .row("status", s.status.as_deref().unwrap_or("-"))
            .row(
                "current",
                s.current_version
                    .map(|v| format!("v{v}"))
                    .unwrap_or_else(|| "none".into()),
            )
            .row(
                "source",
                s.source
                    .as_ref()
                    .map(ttm_api::workflows::item_label)
                    .unwrap_or_else(|| "-".into()),
            )
            .opt("tags", (!s.tags.is_empty()).then(|| s.tags.join(", ")))
            .row("built-in", if s.builtin { "yes" } else { "no" }),
    );
    ui.blank();
    let mut t = Table::new(&[
        Col::new("VERSION"),
        Col::new("REVIEW"),
        Col::new("SIZE").right(),
        Col::new("SHA256"),
        Col::new("NOTE").flex(1).min(4),
    ])
    .empty_text("No versions.");
    for v in &s.versions {
        t.push(vec![
            Cell::new(format!("v{}", v.version)),
            review_cell(v.review_status.as_deref()),
            Cell::new(v.size.map(|n| human_size(n as u64)).unwrap_or_else(|| "-".into())),
            Cell::token(
                v.sha256.as_deref().map(|h| &h[..h.len().min(12)]).unwrap_or("-"),
                Token::Muted,
            ),
            Cell::new(v.note.as_deref().unwrap_or("")),
        ]);
    }
    ui.print(&t);
    if let Some(d) = detail
        && let Some(files) = d
            .manifest
            .as_ref()
            .and_then(|m| m.get("files"))
            .and_then(Value::as_array)
    {
        ui.blank();
        let mut f = Table::new(&[Col::new("FILE").flex(1).min(6), Col::new("SIZE").right()]);
        for file in files {
            f.push(vec![
                Cell::new(file.get("path").and_then(Value::as_str).unwrap_or("?")),
                Cell::new(
                    file.get("size")
                        .and_then(Value::as_u64)
                        .map(human_size)
                        .unwrap_or_default(),
                ),
            ]);
        }
        ui.print(&f);
    }
    ui.line(&next_line(ui.mode(), &format!("ttm skill pull {}", s.slug)));
}

async fn review(app: &App, client: &Client, cmd: SkillReviewCommand) -> Result<()> {
    match cmd {
        SkillReviewCommand::List { ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let queue = client.skill_reviews(&ws).await?;
            emit(app, &queue, |ui| {
                ui.line(&format!("{} version(s) waiting for review", queue.len()));
                let mut t = Table::new(&[
                    Col::new("SKILL"),
                    Col::new("VERSION"),
                    Col::new("BY").flex(1).min(4),
                    Col::new("WHEN"),
                ])
                .empty_text("Nothing to review.");
                for q in &queue {
                    let slug = q
                        .get("slug")
                        .or_else(|| q.pointer("/skill/slug"))
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let by = q
                        .pointer("/created_by/name")
                        .or_else(|| q.get("author"))
                        .and_then(Value::as_str)
                        .unwrap_or("-");
                    t.push(vec![
                        Cell::token(slug, Token::Ink),
                        Cell::new(format!(
                            "v{}",
                            q.get("version").and_then(Value::as_i64).unwrap_or(0)
                        )),
                        Cell::new(by),
                        Cell::token(
                            q.get("created_at")
                                .and_then(Value::as_str)
                                .map(super::automation::short_time)
                                .unwrap_or("-"),
                            Token::Muted,
                        ),
                    ]);
                }
                ui.print(&t);
                if let Some(q) = queue.first() {
                    let slug = q
                        .get("slug")
                        .or_else(|| q.pointer("/skill/slug"))
                        .and_then(Value::as_str)
                        .unwrap_or("?");
                    let v = q.get("version").and_then(Value::as_i64).unwrap_or(0);
                    ui.line(&next_line(
                        ui.mode(),
                        &format!("ttm skill review approve {slug}@{v}"),
                    ));
                }
            });
        }
        SkillReviewCommand::Approve { target, note, ws } => {
            decide(app, client, &target, true, note, ws.workspace).await?
        }
        SkillReviewCommand::Reject { target, note, ws } => {
            decide(app, client, &target, false, Some(note), ws.workspace).await?
        }
    }
    Ok(())
}

async fn decide(
    app: &App,
    client: &Client,
    target: &str,
    approve: bool,
    note: Option<String>,
    ws: Option<String>,
) -> Result<()> {
    let ws = app.workspace_id(ws.as_deref())?;
    let (slug, version) = split_target(target)?;
    let skill = must_find(client, &ws, &slug).await?;
    // The diff with the version in use, before deciding.
    if let Ok(diff) = client.skill_diff(&skill.id, version, None).await
        && !app.json
    {
        app.ui
            .print(&Header::new(format!("{slug} v{version}")).summary(format!(
            "against {}",
            skill.current_version.map(|v| format!("v{v}")).unwrap_or_else(|| "nothing (first version)".into())
        )));
        for f in &diff.files {
            match (&f.unified, f.status.as_str()) {
                (_, "same") => {}
                (Some(u), _) => {
                    app.ui
                        .line(&app.ui.paint(Token::Ink, &format!("{} ({})", f.path, f.status)));
                    for line in u.lines() {
                        let token = if line.starts_with('+') {
                            Some(Token::Teal)
                        } else if line.starts_with('-') {
                            Some(Token::Coral)
                        } else if line.starts_with("@@") {
                            Some(Token::Purple)
                        } else {
                            None
                        };
                        app.ui.line(&match token {
                            Some(t) => app.ui.paint(t, line),
                            None => line.to_string(),
                        });
                    }
                }
                (None, status) => app.ui.print(&Diff::changes(&[
                    ttm_ui::Change::changed(f.path.clone()).detail(status.to_string())
                ])),
            }
        }
    }
    let verb = if approve { "Approve" } else { "Reject" };
    if !app.wizard().confirm(&format!("{verb} {slug} v{version}?"))? {
        bail!("cancelled");
    }
    let out = client
        .skill_review(
            &skill.id,
            version,
            if approve { "approve" } else { "reject" },
            note.as_deref(),
        )
        .await?;
    emit(app, &out, |ui| {
        let done = if approve { "approved" } else { "rejected" };
        ui.print(
            &Callout::success(format!("{slug} v{version} {done}")).next(format!("ttm skill show {slug}")),
        )
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_and_sizes() {
        assert_eq!(split_target("ttm-tdd@3").unwrap(), ("ttm-tdd".to_string(), 3));
        assert_eq!(split_target("ttm-tdd@v2").unwrap().1, 2);
        assert!(split_target("ttm-tdd").is_err());
        assert_eq!(human_size(2048), "2.0 KB");
    }
}
