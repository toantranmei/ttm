//! `ttm project init | link | open | status`.

use crate::app::App;
use crate::cli::ProjectCommand;
use crate::output::emit;
use crate::util::open_browser;
use anyhow::{Context as _, Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use ttm_api::Client;
use ttm_api::factory::{NewProject, Project};
use ttm_core::project_file::{Linked, ProjectFile, write_tree};
use ttm_core::worktree;

/// The linked repository around the current directory.
pub fn linked() -> Result<Linked> {
    let cwd = std::env::current_dir()?;
    Linked::find(&cwd)?
        .ok_or_else(|| anyhow!("this directory is not linked to a project; run `ttm project link <KEY>`"))
}

/// The git top level of the current directory (or the directory itself).
pub fn repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    Ok(worktree::git(&cwd, &["rev-parse", "--show-toplevel"])
        .map(PathBuf::from)
        .unwrap_or(cwd))
}

/// `git@github.com:a/b.git` / `https://gitlab.com/a/b` → provider.
pub fn provider_of(url: &str) -> &'static str {
    if url.contains("github.com") {
        "github"
    } else if url.contains("gitlab") {
        "gitlab"
    } else {
        "local"
    }
}

pub async fn run(app: App, command: ProjectCommand) -> Result<ExitCode> {
    match command {
        ProjectCommand::Init {
            key,
            name,
            repo_url,
            workspace,
        } => {
            let (client, _) = app.api().await?;
            let root = repo_root()?;
            let repo_url = repo_url.or_else(|| worktree::git(&root, &["remote", "get-url", "origin"]).ok());
            let name = name.unwrap_or_else(|| {
                root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Project")
                    .to_string()
            });
            let project = client
                .create_project(&NewProject {
                    key: key.to_uppercase(),
                    name,
                    workspace_id: workspace.or_else(|| app.ctx.workspace()),
                    repo_provider: Some(
                        repo_url
                            .as_deref()
                            .map(provider_of)
                            .unwrap_or("local")
                            .to_string(),
                    ),
                    repo_url,
                    default_branch: Some(worktree::default_branch(&root)),
                })
                .await?;
            let linked = link(&app, &client, &root, &project).await?;
            emit(app.json, &linked.file, || {
                println!(
                    "Created project {} ({}) and linked {}.",
                    project.key,
                    project.name,
                    root.display()
                )
            });
        }
        ProjectCommand::Link { project } => {
            let (client, _) = app.api().await?;
            let found = client
                .find_project(&project)
                .await?
                .ok_or_else(|| anyhow!("no project `{project}` in your workspaces"))?;
            // List items lack the repository settings; the single project has them.
            let found = client.project(&found.id).await.unwrap_or(found);
            let root = repo_root()?;
            let linked = link(&app, &client, &root, &found).await?;
            emit(app.json, &linked.file, || {
                println!(
                    "Linked {} to {} ({}), plan rev {}.",
                    root.display(),
                    found.key,
                    found.name,
                    linked.file.synced_rev
                );
                println!(
                    "Commit .ttm/project.toml and .ttm/plan; run `ttm rules apply` to install the project rules."
                );
            });
        }
        ProjectCommand::Open => {
            let linked = linked()?;
            let url = format!("{}/factory?project={}", linked.file.host, linked.file.project_id);
            open_browser(&url, false);
            emit(app.json, &serde_json::json!({ "url": url }), || {});
        }
        ProjectCommand::Status => {
            let linked = linked()?;
            emit(app.json, &linked.file, || {
                println!("project: {} ({})", linked.file.key, linked.file.project_id);
                println!("host:    {}", linked.file.host);
                println!("rev:     {}", linked.file.synced_rev);
                if let Some(cr) = &linked.file.pending_change_request {
                    println!("pending change request: {cr}");
                }
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Writes `.ttm/project.toml` and pulls the plan into `.ttm/plan` and `.ttm/base`.
async fn link(app: &App, client: &Client, root: &Path, project: &Project) -> Result<Linked> {
    let bundle = client
        .export_plan(&project.id, None)
        .await
        .context("downloading the plan")?;
    let linked = Linked {
        root: root.to_path_buf(),
        file: ProjectFile {
            host: app.host().to_string(),
            project_id: project.id.clone(),
            key: project.key.clone(),
            synced_rev: bundle.rev,
            default_branch: project.default_branch.clone(),
            pending_change_request: None,
        },
    };
    let files = bundle.plan_files();
    write_tree(&linked.plan_dir(), &files)?;
    write_tree(&linked.base_dir(), &files)?;
    linked.save()?;
    Ok(linked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers() {
        assert_eq!(provider_of("git@github.com:toantranmei/ttm.git"), "github");
        assert_eq!(provider_of("https://gitlab.example.com/a/b"), "gitlab");
        assert_eq!(provider_of("/srv/repo.git"), "local");
    }
}
