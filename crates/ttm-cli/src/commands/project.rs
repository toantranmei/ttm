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
use ttm_ui::{Ask, Callout, Cell, Choice, Col, KeyValue, Status, Table, Token, next_line};

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

/// `--provider` when given, else guessed from the URL ([`provider_of`]), `local` without a URL.
pub fn repo_provider(flag: Option<&str>, url: Option<&str>) -> String {
    flag.map(str::to_string)
        .unwrap_or_else(|| url.map(provider_of).unwrap_or("local").to_string())
}

pub async fn run(app: App, command: ProjectCommand) -> Result<ExitCode> {
    match command {
        ProjectCommand::Init(args) => init(&app, args).await?,
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
            emit(&app, &linked.file, |ui| {
                ui.print(
                    &Callout::success(format!(
                        "Linked {} to {} ({})",
                        root.display(),
                        found.key,
                        found.name
                    ))
                    .detail(format!("plan rev {}", linked.file.synced_rev))
                    .detail("Commit .ttm/project.toml and .ttm/plan.")
                    .next("ttm rules apply"),
                );
            });
        }
        ProjectCommand::Open => {
            let linked = linked()?;
            let url = format!("{}/factory?project={}", linked.file.host, linked.file.project_id);
            open_browser(&app.ui, &url, false);
            emit(&app, &serde_json::json!({ "url": url }), |ui| {
                ui.print(&Callout::success(format!(
                    "Opened {} on the web",
                    linked.file.key
                )));
            });
        }
        ProjectCommand::Status => {
            let linked = linked()?;
            emit(&app, &linked.file, |ui| {
                ui.line(&format!(
                    "{} · linked at {}",
                    ui.paint(Token::Ink, &linked.file.key),
                    linked.root.display()
                ));
                let kv = KeyValue::new()
                    .row(
                        "project",
                        format!("{} ({})", linked.file.key, linked.file.project_id),
                    )
                    .row("host", linked.file.host.as_str())
                    .row("rev", linked.file.synced_rev.to_string())
                    .opt("branch", linked.file.default_branch.as_deref())
                    .row(
                        "sync",
                        match &linked.file.pending_change_request {
                            Some(cr) => format!(
                                "{} change request {cr} waits for approval",
                                ui.render(&Status::of_category("approval"))
                            ),
                            None => format!("{} no pending change request", ui.render(&Status::ok())),
                        },
                    );
                ui.print(&kv);
                ui.line(&next_line(ui.mode(), "ttm sync --check"));
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// `AUR` from `Aurora web`: initials of the words (or the first letters), 2 to 4 capitals.
pub fn suggest_key(name: &str) -> String {
    let words: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            w.chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect();
    let mut key: String = if words.len() >= 2 {
        words.iter().filter_map(|w| w.chars().next()).take(4).collect()
    } else {
        words
            .first()
            .map(|w| w.chars().take(3).collect())
            .unwrap_or_default()
    };
    key = key.to_ascii_uppercase();
    if !key.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        key.insert(0, 'P');
    }
    while key.len() < 2 {
        key.push('X');
    }
    key
}

#[allow(clippy::ptr_arg)] // validators take `&T`
fn valid_key(key: &String) -> std::result::Result<(), String> {
    let b = key.as_bytes();
    let ok = (2..=10).contains(&b.len())
        && b[0].is_ascii_uppercase()
        && b.iter().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err("2 to 10 capitals or digits, starting with a letter, e.g. AUR".into())
    }
}

#[allow(clippy::ptr_arg)]
fn not_empty(v: &String) -> std::result::Result<(), String> {
    if v.trim().is_empty() {
        Err("cannot be empty".into())
    } else {
        Ok(())
    }
}

/// The workflow templates offered by the wizard.
fn workflow_choices() -> Vec<Choice<String>> {
    [
        ("professional", "Professional", "9 statuses, 6 agents"),
        (
            "balanced",
            "Balanced",
            "5 statuses, 3 agents, about 40% fewer tokens",
        ),
        ("kanban", "Kanban", "a simple board"),
        ("scrum", "Scrum", "sprints and review"),
        ("autopilot", "Autopilot", "agents take tasks to done"),
    ]
    .map(|(v, l, h)| Choice::new(v.to_string(), l).hint(h))
    .to_vec()
}

/// `ttm project init`: a wizard where every question has a flag (`--yes` takes defaults). Without a terminal it
/// works like 0.1.6: `--key` is the only flag needed.
async fn init(app: &App, args: crate::cli::ProjectInitArgs) -> Result<()> {
    use crate::cli::{AgentsChoice, RunnerChoice};
    let (client, discovery) = app.api().await?;
    let v2 = discovery.has_feature("workflow_v2");
    if !v2 && (args.workflow.is_some() || args.agents.is_some()) {
        return Err(ttm_ui::UsageError::unsupported_feature("workflow_v2", "project templates").into());
    }
    let root = repo_root()?;
    let mut wizard = app.wizard();
    let dir_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Project")
        .to_string();
    // No terminal and no `--yes`: the 0.1.6 form, where `--key` is the only flag needed and every other answer is
    // its 0.1.6 default (the folder name, no template, no agents, no runner).
    let legacy = !wizard.interactive() && !wizard.yes();
    // `--agents` picks the agents of a template: without a terminal there is no template unless `--workflow` names it.
    if legacy && args.agents.is_some() && args.workflow.is_none() {
        return Err(ttm_ui::UsageError::missing_flag("--workflow", false)
            .fix("--agents needs a template: pass --workflow <template> (e.g. --workflow balanced), or --yes for the default")
            .into());
    }
    let name = wizard.text(
        Ask::new("Project name", "--name")
            .value(args.name.clone().or_else(|| legacy.then(|| dir_name.clone())))
            .default(dir_name)
            .validate(not_empty),
    )?;
    let key = wizard.text(
        Ask::new("Task key prefix", "--key")
            .value(args.key.clone().map(|k| k.to_uppercase()))
            .default(suggest_key(&name))
            .validate(valid_key),
    )?;
    // Workspace: asked only at a terminal; otherwise the flag or the current one (as 0.1.6).
    let workspace = match (&args.workspace, wizard.interactive() && !wizard.yes()) {
        (Some(w), _) => Some(w.clone()),
        (None, true) => {
            let me = client.me().await?;
            let choices: Vec<Choice<String>> = me
                .workspaces
                .iter()
                .map(|w| Choice::new(w.id.clone(), w.name.clone()).hint(format!("{} · {}", w.kind, w.role)))
                .collect();
            Some(wizard.select(
                Ask::new("Workspace", "--workspace").default_opt(app.ctx.workspace()),
                &choices,
            )?)
        }
        (None, false) => app.ctx.workspace(),
    };
    let origin = worktree::git(&root, &["remote", "get-url", "origin"]).ok();
    let repo_url = match (&args.repo_url, wizard.interactive() && !wizard.yes()) {
        (Some(r), _) => Some(r.clone()),
        (None, true) => {
            let r = wizard.text(
                Ask::new("Repository URL (empty for none)", "--repo")
                    .default(origin.clone().unwrap_or_default()),
            )?;
            (!r.trim().is_empty()).then_some(r)
        }
        (None, false) => origin,
    };
    let mut template = None;
    let mut agents = std::collections::BTreeMap::new();
    let mut presets = Vec::new();
    if v2 && !(legacy && args.workflow.is_none()) {
        let wf = wizard.select(
            Ask::new("Workflow", "--workflow")
                .value(args.workflow.clone())
                .default("professional".to_string()),
            &workflow_choices(),
        )?;
        presets = client.agent_presets(Some(&wf)).await.unwrap_or_default();
        if !presets.is_empty() {
            let mut preview = Table::new(&[
                Col::new("AGENT").flex(1).min(8),
                Col::new("ROLE"),
                Col::new("MODEL"),
                Col::new("SKILLS").flex(1).min(6),
            ]);
            for p in &presets {
                preview.push(vec![
                    Cell::new(format!("{} {}", app.ui.render(&ttm_ui::Badge::agent()), p.name)),
                    Cell::new(p.role.as_deref().unwrap_or("-")),
                    Cell::new(p.model.as_deref().unwrap_or("-")),
                    Cell::new(if p.skills.is_empty() {
                        "-".to_string()
                    } else {
                        p.skills.join(", ")
                    }),
                ]);
            }
            if wizard.interactive() && args.agents.is_none() && !wizard.yes() {
                app.ui.eprint(&preview);
            }
            let default = if wizard.interactive() && !wizard.yes() {
                AgentsChoice::Create
            } else {
                AgentsChoice::None
            };
            let choice = wizard.select(
                Ask::new("Agents of the template", "--agents")
                    .value(
                        args.agents
                            .map(AgentsWord)
                            .or(legacy.then_some(AgentsWord(AgentsChoice::None))),
                    )
                    .default(AgentsWord(default)),
                &[
                    Choice::new(AgentsWord(AgentsChoice::Create), "Create them"),
                    Choice::new(
                        AgentsWord(AgentsChoice::Reuse),
                        "Reuse agents with the same role, create the others",
                    ),
                    Choice::new(AgentsWord(AgentsChoice::None), "No agents"),
                ],
            )?;
            let existing = match (choice.0, &workspace) {
                (AgentsChoice::Reuse, Some(ws)) => client.agents(ws).await.unwrap_or_default(),
                _ => vec![],
            };
            for p in &presets {
                let value = match choice.0 {
                    AgentsChoice::None => "skip".to_string(),
                    AgentsChoice::Create => "new".to_string(),
                    AgentsChoice::Reuse => existing
                        .iter()
                        .find(|a| {
                            a.status.as_deref() != Some("archived") && a.role.is_some() && a.role == p.role
                        })
                        .map(|a| format!("reuse:{}", a.id))
                        .unwrap_or_else(|| "new".into()),
                };
                agents.insert(p.code.clone(), value);
            }
        }
        template = Some(wf);
    }
    let runner = if wizard.interactive() || args.runner.is_some() || wizard.yes() {
        wizard
            .select(
                Ask::new("Run agents on this machine", "--runner")
                    .value(args.runner.map(RunnerWord))
                    .default(RunnerWord(RunnerChoice::None)),
                &[
                    Choice::new(
                        RunnerWord(RunnerChoice::This),
                        "Yes: this machine is a runner for the project",
                    ),
                    Choice::new(RunnerWord(RunnerChoice::None), "Not now"),
                ],
            )?
            .0
    } else {
        RunnerChoice::None
    };
    // 0.1.6 scripts run `ttm project init --key AUR` without a terminal: no confirmation there.
    if wizard.interactive() && !wizard.confirm(&format!("Create project {key} ({name})?"))? {
        anyhow::bail!("cancelled");
    }
    wizard.print_equivalent("project init");
    let project = client
        .create_project(&NewProject {
            key: key.clone(),
            name,
            workspace_id: workspace,
            repo_provider: Some(repo_provider(args.provider.as_deref(), repo_url.as_deref())),
            repo_url,
            default_branch: Some(worktree::default_branch(&root)),
            template_slug: template,
            agents,
        })
        .await?;
    let linked = link(app, &client, &root, &project).await?;
    if runner == RunnerChoice::This {
        let mut config = ttm_runner::RunnerConfig::load(&app.ctx.paths.config_dir)?;
        config.repos.insert(project.key.clone(), root.clone());
        config.save(&app.ctx.paths.config_dir)?;
    }
    emit(app, &linked.file, |ui| {
        ui.print(
            &Callout::success(format!("Created project {} ({})", project.key, project.name)).detail(format!(
                "linked {} · plan rev {}",
                root.display(),
                linked.file.synced_rev
            )),
        );
        if !project.agents.is_empty() {
            let mut t = Table::new(&[Col::new("PRESET"), Col::new("AGENT").flex(1).min(8), Col::new("")]);
            for a in &project.agents {
                let name = presets
                    .iter()
                    .find(|p| p.code == a.code)
                    .map(|p| p.name.as_str())
                    .unwrap_or(&a.code);
                t.push(vec![
                    Cell::new(a.code.as_str()),
                    Cell::new(format!("{} {name}", ui.render(&ttm_ui::Badge::agent()))),
                    Cell::token(if a.created { "created" } else { "reused" }, Token::Muted),
                ]);
            }
            ui.print(&t);
        }
        let next = if runner == RunnerChoice::This {
            "ttm runner up"
        } else {
            "ttm rules apply"
        };
        ui.line(&next_line(ui.mode(), next));
    });
    Ok(())
}

/// `create`/`reuse`/`none` as a flag value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AgentsWord(crate::cli::AgentsChoice);

impl std::fmt::Display for AgentsWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::cli::AgentsChoice::*;
        f.write_str(match self.0 {
            Create => "create",
            Reuse => "reuse",
            None => "none",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RunnerWord(crate::cli::RunnerChoice);

impl std::fmt::Display for RunnerWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            crate::cli::RunnerChoice::This => "this",
            crate::cli::RunnerChoice::None => "none",
        })
    }
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
    fn key_suggestions() {
        assert_eq!(suggest_key("Aurora web app"), "AWA");
        assert_eq!(suggest_key("aurora"), "AUR");
        assert_eq!(suggest_key("42"), "P42");
        assert!(valid_key(&"AUR".to_string()).is_ok());
        assert!(valid_key(&"aur".to_string()).is_err());
        assert!(valid_key(&"A".to_string()).is_err());
    }

    #[test]
    fn providers() {
        assert_eq!(provider_of("git@github.com:toantranmei/ttm.git"), "github");
        assert_eq!(provider_of("https://gitlab.example.com/a/b"), "gitlab");
        assert_eq!(provider_of("/srv/repo.git"), "local");
        assert_eq!(
            repo_provider(Some("github"), Some("http://127.0.0.1:4600/sim/todo-web.git")),
            "github"
        );
        assert_eq!(repo_provider(None, Some("https://github.com/a/b")), "github");
        assert_eq!(repo_provider(None, None), "local");
    }
}
