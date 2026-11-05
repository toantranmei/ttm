//! `ttm template list | publish | install` (S5 public template library).

use crate::app::App;
use crate::cli::{PublishFrom, TemplateCommand};
use crate::output::emit;
use anyhow::{Result, anyhow, bail};
use serde_json::{Value, json};
use std::process::ExitCode;
use ttm_api::templates::TemplateSource;
use ttm_ui::{Callout, Cell, Col, Header, Table, Token, next_line};

fn price(v: &Option<Value>) -> String {
    match v {
        None | Some(Value::Null) => "free".into(),
        Some(Value::Number(n)) if n.as_u64() == Some(0) => "free".into(),
        Some(Value::Number(n)) => format!("{n} VND"),
        Some(Value::Object(o)) => o
            .get("amount")
            .and_then(Value::as_u64)
            .map(|a| {
                if a == 0 {
                    "free".to_string()
                } else {
                    format!("{a} VND")
                }
            })
            .unwrap_or_else(|| "-".into()),
        Some(other) => other.to_string(),
    }
}

pub async fn run(app: App, command: TemplateCommand) -> Result<ExitCode> {
    let (client, _) = app.api_for("templates", "the template library").await?;
    match command {
        TemplateCommand::List { kind, query } => {
            let kinds: Vec<&str> = match kind {
                Some(k) => vec![k.as_str()],
                None => vec!["workflow", "agent", "skill", "rule_pack", "bundle"],
            };
            let list = client.catalog(&kinds, query.as_deref()).await?;
            emit(&app, &list, |ui| {
                ui.line(&format!("{} template(s)", list.len()));
                let mut t = Table::new(&[
                    Col::new("SLUG"),
                    Col::new("KIND"),
                    Col::new("NAME").flex(1).min(6),
                    Col::new("TAGLINE").flex(2).min(6),
                    Col::new("PRICE").right(),
                ])
                .empty_text("No templates.");
                for i in &list {
                    t.push(vec![
                        Cell::token(i.slug.as_str(), Token::Ink),
                        Cell::new(i.kind.as_str()),
                        Cell::new(i.name.as_str()),
                        Cell::token(i.tagline.as_deref().unwrap_or(""), Token::Muted),
                        Cell::new(price(&i.price)),
                    ]);
                }
                ui.print(&t);
                if let Some(i) = list.first() {
                    ui.line(&next_line(ui.mode(), &format!("ttm template install {}", i.slug)));
                }
            });
        }
        TemplateCommand::Publish {
            from,
            source,
            price: amount,
            slug,
            name,
            ws,
        } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let (kind, id, default_name) = match from {
                PublishFrom::Workflow => {
                    let (pid, _) = app
                        .project_ref(&client, Some(&source))
                        .await
                        .or_else(|_| Ok::<_, anyhow::Error>((source.clone(), source.clone())))?;
                    let wf_id = client
                        .project(&pid)
                        .await
                        .ok()
                        .and_then(|p| p.workflow_id)
                        .unwrap_or(source.clone());
                    let wf = client.workflow(&wf_id).await?;
                    ("workflow", wf.id, wf.name)
                }
                PublishFrom::Agent => {
                    let a = super::agent_admin::find(&client, &ws, &source).await?;
                    ("agent", a.id, a.name)
                }
                PublishFrom::Skill => {
                    let s = client
                        .skills(&ws, None, None, false)
                        .await?
                        .into_iter()
                        .find(|s| s.slug == source || s.id == source)
                        .ok_or_else(|| anyhow!("no skill `{source}`"))?;
                    ("skill", s.id, s.name)
                }
            };
            let sources = vec![TemplateSource {
                kind: kind.into(),
                id,
            }];
            let preview = client.template_export_preview(&ws, &sources).await?;
            if !app.json {
                app.ui.print(
                    &Header::new(format!("Publish {kind} {default_name}")).summary(format!(
                        "{} file(s){}",
                        preview.files.len(),
                        if preview.runs_code { " · runs code" } else { "" }
                    )),
                );
                if !preview.removed.is_empty() {
                    app.ui.print(
                        &Callout::warning(format!(
                            "{} item(s) are left out of the template",
                            preview.removed.len()
                        ))
                        .details(preview.removed.iter().map(ttm_api::workflows::item_label)),
                    );
                }
            }
            let slug = slug.unwrap_or_else(|| {
                default_name
                    .to_lowercase()
                    .replace(|c: char| !c.is_ascii_alphanumeric(), "-")
            });
            let name = name.unwrap_or(default_name);
            if !app
                .wizard()
                .confirm(&format!("Publish {slug} to the public library?"))?
            {
                bail!("cancelled");
            }
            let item = json!({ "slug": slug, "kind": preview.kind_suggested.clone().unwrap_or_else(|| kind.into()), "name": name });
            let out = client
                .template_publish(&ws, &sources, &preview.digest, &item, None)
                .await?;
            emit(&app, &out, |ui| {
                let item_id = out["item_id"].as_str().unwrap_or("");
                let mut c = Callout::success(format!(
                    "Published {slug} v{}",
                    out["version"].as_i64().unwrap_or(1)
                ));
                if let Some(url) = out["url"].as_str() {
                    c = c.detail(ui.paint(Token::Blue, url));
                }
                // Prices are set on the web (browser session only).
                if amount.is_some() {
                    c = c.detail(format!(
                        "Set the price on the web: {}/me/items/{item_id}",
                        app.host()
                    ));
                }
                ui.print(&c.next("ttm template list"));
            });
        }
        TemplateCommand::Install { slug, ws } => {
            let ws = app.workspace_id(ws.workspace.as_deref())?;
            let item = client
                .catalog(&[], Some(&slug))
                .await?
                .into_iter()
                .find(|i| i.slug == slug || i.id == slug)
                .ok_or_else(|| anyhow!("no template `{slug}`; see `ttm template list`"))?;
            let preview = client.template_install_preview(&item.id, &ws, None).await?;
            if preview.requires_purchase && !preview.owned {
                return Err(ttm_ui::UsageError::new(
                    "purchase_required",
                    format!("{slug} must be bought first"),
                )
                .next(format!(
                    "open {}",
                    item.url
                        .clone()
                        .unwrap_or_else(|| format!("{}/catalog/{slug}", app.host()))
                ))
                .into());
            }
            if !app.json {
                app.ui.print(
                    &Header::new(format!("Install {} ({})", item.name, item.kind)).summary(
                        if preview.runs_code {
                            "! contains skills that run code".to_string()
                        } else {
                            "no code runs".to_string()
                        },
                    ),
                );
                let mut t = Table::new(&[Col::new("CREATES"), Col::new("NAME").flex(1).min(6)]);
                if let Some(w) = &preview.creates.workflow {
                    t.push(vec![
                        Cell::new("workflow"),
                        Cell::new(ttm_api::workflows::item_label(w)),
                    ]);
                }
                for (label, list) in [
                    ("rule", &preview.creates.rules),
                    ("agent", &preview.creates.agents),
                    ("skill", &preview.creates.skills),
                ] {
                    for v in list {
                        t.push(vec![
                            Cell::new(label),
                            Cell::new(ttm_api::workflows::item_label(v)),
                        ]);
                    }
                }
                app.ui.print(&t);
            }
            if !app
                .wizard()
                .confirm(&format!("Install {slug} into this workspace?"))?
            {
                bail!("cancelled");
            }
            let out = client
                .template_install(&item.id, &ws, None, &json!({}), &json!({}))
                .await?;
            let has_skills = !preview.creates.skills.is_empty();
            emit(&app, &out, |ui| {
                let c = Callout::success(format!("Installed {slug}"));
                ui.print(&if has_skills {
                    c.detail("Skills wait for review before agents use them.")
                        .next("ttm skill review")
                } else {
                    c.next("ttm workflow show")
                });
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}
