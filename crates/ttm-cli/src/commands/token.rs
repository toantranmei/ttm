//! `ttm token list | create | revoke`. The `/me/tokens` API only accepts the browser session (a token cannot mint or
//! list tokens), so these commands open `<host>/me/ket-noi`, where the token is shown once.

use crate::app::App;
use crate::cli::TokenCommand;
use crate::output::emit;
use crate::util::open_browser;
use anyhow::{Result, bail};
use std::process::ExitCode;
use ttm_ui::{Callout, Token};

pub async fn run(app: App, command: TokenCommand) -> Result<ExitCode> {
    let url = match &command {
        TokenCommand::List => page_url(app.host(), &[("tab", "token".into())])?,
        TokenCommand::Create { scope, expires, name } => {
            let mut params = vec![("tab", "token".to_string()), ("new", "1".to_string())];
            if let Some(name) = name {
                params.push(("name", name.clone()));
            }
            if let Some(scope) = scope {
                params.push(("scopes", normalize_scopes(scope)));
            }
            if let Some(expires) = expires {
                params.push(("expires_days", parse_days(expires)?.to_string()));
            }
            page_url(app.host(), &params)?
        }
        TokenCommand::Revoke { id } => {
            let mut params = vec![("tab", "token".to_string())];
            if let Some(id) = id {
                params.push(("revoke", id.clone()));
            }
            page_url(app.host(), &params)?
        }
    };
    if !app.json {
        open_browser(&app.ui, &url, false);
    }
    let out = serde_json::json!({ "url": url });
    emit(&app, &out, |ui| {
        ui.print(
            &Callout::info("Tokens are managed on the web (the API does not accept a token here)")
                .detail(format!("Finish in the browser: {}", ui.paint(Token::Blue, &url))),
        );
    });
    Ok(ExitCode::SUCCESS)
}

pub fn page_url(host: &str, params: &[(&str, String)]) -> Result<String> {
    let mut url = url::Url::parse(&format!("{host}/me/ket-noi"))?;
    {
        let mut q = url.query_pairs_mut();
        for (k, v) in params {
            q.append_pair(k, v);
        }
    }
    Ok(url.into())
}

/// `notee:read, workspace:read` → `notee:read workspace:read`.
pub fn normalize_scopes(raw: &str) -> String {
    raw.split([',', ' '])
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `30d`, `30` → 30; 1..=365.
pub fn parse_days(raw: &str) -> Result<u32> {
    let digits = raw.trim().trim_end_matches(['d', 'D']);
    let days: u32 = digits
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid lifetime `{raw}` (use e.g. 30d)"))?;
    if !(1..=365).contains(&days) {
        bail!("lifetime must be between 1 and 365 days");
    }
    Ok(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_url_carries_prefill() {
        let url = page_url(
            "https://a.com",
            &[
                ("tab", "token".into()),
                ("scopes", normalize_scopes("notee:read,workspace:read")),
            ],
        )
        .unwrap();
        assert_eq!(
            url,
            "https://a.com/me/ket-noi?tab=token&scopes=notee%3Aread+workspace%3Aread"
        );
    }

    #[test]
    fn days() {
        assert_eq!(parse_days("30d").unwrap(), 30);
        assert_eq!(parse_days("365").unwrap(), 365);
        assert!(parse_days("0d").is_err());
        assert!(parse_days("400d").is_err());
        assert!(parse_days("month").is_err());
    }
}
