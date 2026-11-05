//! What every command shares: resolved context, discovery, the token and an API client.

use anyhow::{Context as _, Result, anyhow};
use time::OffsetDateTime;
use ttm_api::{Client, TokenResponse};
use ttm_core::{Context, Discovery, Overrides, StoredToken, TokenSource};
use ttm_ui::Ui;

pub struct App {
    pub ctx: Context,
    pub json: bool,
    /// Everything printed for people goes through here.
    pub ui: Ui,
    /// `-y/--yes`: confirm and take defaults without asking.
    #[allow(dead_code)] // read by the wizard commands
    pub yes: bool,
    /// `--host` was given explicitly (login then remembers it in the profile).
    pub host_flag: bool,
    /// `--profile` was given explicitly.
    pub profile_flag: bool,
}

impl App {
    pub fn load(global: &crate::cli::GlobalArgs, ui: Ui) -> Result<Self> {
        let overrides = Overrides {
            host: global.host.clone(),
            profile: global.profile.clone(),
        };
        let ctx = Context::load(&overrides).context("loading the ttm configuration")?;
        Ok(Self {
            ctx,
            json: global.json,
            ui,
            yes: global.yes,
            host_flag: global.host.is_some() || std::env::var_os("TTM_HOST").is_some(),
            profile_flag: global.profile.is_some(),
        })
    }

    /// An authenticated client for a v2 command: fails with `unsupported_feature` (exit 2) before calling any new
    /// endpoint when the server's discovery does not list `feature`.
    pub async fn api_for(&self, feature: &str, what: &str) -> Result<(Client, Discovery)> {
        let (client, discovery) = self.api().await?;
        if !discovery.has_feature(feature) {
            return Err(ttm_ui::UsageError::unsupported_feature(feature, what).into());
        }
        Ok((client, discovery))
    }

    /// `--workspace`, else the current workspace of the profile.
    pub fn workspace_id(&self, flag: Option<&str>) -> Result<String> {
        flag.map(String::from)
            .or_else(|| self.ctx.workspace())
            .ok_or_else(|| {
                ttm_ui::UsageError::missing_flag("--workspace", false)
                    .fix("pass --workspace, or choose one with `ttm workspace switch <name>`")
                    .into()
            })
    }

    /// `--project <key|id>`, else the project linked to this directory: `(id, key)`.
    pub async fn project_ref(&self, client: &Client, flag: Option<&str>) -> Result<(String, String)> {
        match flag {
            Some(p) => {
                let found = client
                    .find_project(p)
                    .await?
                    .ok_or_else(|| anyhow!("no project `{p}` in your workspaces; see the web"))?;
                Ok((found.id, found.key))
            }
            None => {
                let linked =
                    crate::commands::project::linked().map_err(|_| {
                        anyhow::Error::from(ttm_ui::UsageError::missing_flag("--project", false).fix(
                            "pass --project <KEY>, or run inside a linked repository (ttm project link)",
                        ))
                    })?;
                Ok((linked.file.project_id, linked.file.key))
            }
        }
    }

    /// A wizard for this command: questions only at a terminal, `--yes` takes the defaults.
    #[allow(dead_code)] // used by the wizard commands
    pub fn wizard(&self) -> ttm_ui::Wizard<'_> {
        ttm_ui::Wizard::new(&self.ui, self.yes)
    }

    pub fn host(&self) -> &str {
        &self.ctx.host
    }

    /// Client without a token (discovery, OAuth endpoints).
    pub fn anonymous(&self, discovery: &Discovery) -> Result<Client> {
        Ok(Client::new(discovery.api_base(self.host()), None)?.with_cache_dir(&self.ctx.paths.cache_dir))
    }

    pub async fn discovery(&self, refresh: bool) -> Result<Discovery> {
        let http = Client::new(format!("{}/api/v1", self.host()), None)?;
        let discovery =
            ttm_core::discovery::discover(http.http(), self.host(), &self.ctx.paths.cache_dir, refresh)
                .await
                .with_context(|| format!("cannot reach {}", self.host()))?;
        if let Some(min) = discovery.upgrade_required(ttm_core::VERSION) {
            tracing::warn!(
                "the server needs ttm >= {min} (this is {}); run `ttm self-update`",
                ttm_core::VERSION
            );
        }
        Ok(discovery)
    }

    pub fn token(&self) -> Result<Option<(StoredToken, TokenSource)>> {
        Ok(self.ctx.credentials.load(&self.ctx.profile_name, self.host())?)
    }

    pub fn save_token(&self, token: &StoredToken) -> Result<TokenSource> {
        Ok(self
            .ctx
            .credentials
            .save(&self.ctx.profile_name, self.host(), token)?)
    }

    /// An authenticated client. Refreshes an expired OAuth token when a refresh token is stored.
    pub async fn api(&self) -> Result<(Client, Discovery)> {
        let discovery = self.discovery(false).await?;
        let (mut token, source) = self.token()?.ok_or_else(|| {
            anyhow!(
                "not signed in to {} (profile {}); run `ttm login`",
                self.host(),
                self.ctx.profile_name
            )
        })?;
        if source != TokenSource::Env && token.is_expired(OffsetDateTime::now_utc()) {
            let refresh = token
                .refresh_token
                .clone()
                .ok_or_else(|| anyhow!("the saved token has expired; run `ttm login`"))?;
            let fresh = self
                .anonymous(&discovery)?
                .refresh_token(&discovery.token_endpoint(self.host()), &refresh)
                .await
                .context("the session expired and could not be renewed; run `ttm login`")?;
            token = stored_from(fresh, Some(refresh));
            self.save_token(&token)?;
        }
        let client = Client::new(discovery.api_base(self.host()), Some(token.access_token))?
            .with_cache_dir(&self.ctx.paths.cache_dir);
        Ok((client, discovery))
    }
}

/// Token endpoint response → what is stored. Keeps the previous refresh token when the server does not rotate it.
pub fn stored_from(response: TokenResponse, previous_refresh: Option<String>) -> StoredToken {
    let expires_at = response
        .expires_in
        .filter(|s| *s > 0)
        .map(|s| OffsetDateTime::now_utc() + time::Duration::seconds(s));
    StoredToken {
        access_token: response.access_token,
        refresh_token: response.refresh_token.or(previous_refresh),
        expires_at,
        scope: response.scope,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_from_sets_expiry_and_keeps_refresh() {
        let response = TokenResponse {
            access_token: "a".into(),
            token_type: "Bearer".into(),
            expires_in: Some(3600),
            refresh_token: None,
            scope: Some("workspace:read".into()),
        };
        let stored = stored_from(response, Some("r".into()));
        assert_eq!(stored.refresh_token.as_deref(), Some("r"));
        let left = stored.expires_at.unwrap() - OffsetDateTime::now_utc();
        assert!(left.whole_seconds() > 3500 && left.whole_seconds() <= 3600);
    }
}
