//! `ttm inbox [--unread] [--kind factory] [--watch] [--interval 5]` and `ttm inbox read [<id>…|--all]`.
//! `--watch` polls `GET /notifications?since=<newest created_at>` (no TUI; `--json` prints NDJSON).

use crate::app::App;
use crate::cli::{InboxArgs, InboxSub};
use crate::output::{emit, emit_line};
use anyhow::Result;
use std::process::ExitCode;
use std::time::Duration;
use ttm_api::inbox::{InboxQuery, Notification};
use ttm_ui::{Callout, Cell, Col, Table, Token, Ui, next_line};

/// Smallest `--interval`.
pub const MIN_INTERVAL: u64 = 2;

fn dot(ui: &Ui, unread: bool) -> String {
    match (unread, ui.mode().plain) {
        (true, true) => "*".into(),
        (true, false) => ui.paint(Token::Teal, "●"),
        (false, _) => String::new(),
    }
}

/// `● 14:02 skill_review  Skill ttm-tdd v3 waiting for review → <url>`.
pub fn watch_line(ui: &Ui, n: &Notification) -> String {
    let time = n
        .created_at
        .as_deref()
        .and_then(|t| t.get(11..16))
        .unwrap_or("--:--");
    let mut line = format!(
        "{} {} {}  {}",
        if ui.mode().plain {
            "new".to_string()
        } else {
            ui.paint(Token::Teal, "●")
        },
        ui.paint(Token::Muted, time),
        ui.paint(Token::Purple, n.reason()),
        n.message()
    );
    if let Some(url) = &n.url {
        line.push_str(&format!(" → {}", ui.paint(Token::Blue, url)));
    }
    line
}

pub async fn run(app: App, args: InboxArgs) -> Result<ExitCode> {
    let (client, _) = app.api().await?;
    if let Some(InboxSub::Read { ids, all }) = args.sub {
        let out = client.notifications_read(if all { &[] } else { &ids }).await?;
        emit(&app, &out, |ui| {
            let what = if all || ids.is_empty() {
                "All notifications".to_string()
            } else {
                format!("{} notification(s)", ids.len())
            };
            ui.print(&Callout::success(format!("{what} marked read")))
        });
        return Ok(ExitCode::SUCCESS);
    }
    let mut query = InboxQuery {
        kinds: args.kind.clone(),
        unread: args.unread,
        since: None,
    };
    let list = client.notifications(&query).await?;
    if args.watch && app.json {
        for n in &list {
            emit_line(&app.ui, n);
        }
    } else {
        emit(&app, &list, |ui| {
            let unread = list.iter().filter(|n| n.unread()).count();
            ui.line(&format!("{} notification(s), {unread} unread", list.len()));
            let mut t = Table::new(&[
                Col::new(""),
                Col::new("KIND"),
                Col::new("TEXT").flex(3).min(10),
                Col::new("AGE").right(),
                Col::new("LINK").flex(1).min(6),
            ])
            .empty_text("Nothing new.");
            for n in &list {
                t.push(vec![
                    Cell::new(dot(ui, n.unread())),
                    Cell::token(n.reason(), Token::Purple),
                    Cell::new(n.message()),
                    Cell::token(super::cr::age(n.created_at.as_deref()), Token::Muted),
                    Cell::token(n.url.as_deref().unwrap_or(""), Token::Blue),
                ]);
            }
            ui.print(&t);
            if !args.watch && unread > 0 {
                ui.line(&next_line(ui.mode(), "ttm inbox read --all"));
            }
        });
    }
    if !args.watch {
        return Ok(ExitCode::SUCCESS);
    }
    let interval = Duration::from_secs(args.interval.max(MIN_INTERVAL));
    query.since = newest(&list, None);
    if !app.json {
        app.ui.eline(&app.ui.paint(
            Token::Muted,
            &format!("Watching every {}s. Ctrl-C to stop.", interval.as_secs()),
        ));
    }
    let stop = tokio::signal::ctrl_c();
    tokio::pin!(stop);
    loop {
        tokio::select! {
            _ = &mut stop => break,
            _ = tokio::time::sleep(interval) => {}
        }
        let fresh = match client.notifications(&query).await {
            Ok(f) => f,
            Err(err) => {
                tracing::warn!("inbox poll failed: {err}");
                continue;
            }
        };
        // Oldest first, one line each.
        let mut fresh = fresh;
        fresh.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        for n in &fresh {
            if app.json {
                emit_line(&app.ui, n);
            } else {
                app.ui.line(&watch_line(&app.ui, n));
            }
        }
        query.since = newest(&fresh, query.since.take());
    }
    Ok(ExitCode::SUCCESS)
}

/// The newest `created_at` of `list`, or `current`.
fn newest(list: &[Notification], current: Option<String>) -> Option<String> {
    list.iter()
        .filter_map(|n| n.created_at.clone())
        .chain(current)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ttm_ui::{ColorLevel, Mode};

    #[test]
    fn watch_lines_and_since() {
        let n: Notification = serde_json::from_value(serde_json::json!({
            "id": "n1", "kind": "factory", "text": "Skill ttm-tdd v3 waiting for review",
            "url": "https://x/f", "created_at": "2026-11-05T14:02:00Z", "data": {"reason": "skill_review"}
        }))
        .unwrap();
        let (ui, _) = Ui::buffered(Mode::fixed(ColorLevel::None, false, 80));
        assert_eq!(
            watch_line(&ui, &n),
            "● 14:02 skill_review  Skill ttm-tdd v3 waiting for review → https://x/f"
        );
        assert_eq!(
            newest(&[n], Some("2026-11-05T10:00:00Z".into())).as_deref(),
            Some("2026-11-05T14:02:00Z")
        );
    }
}
