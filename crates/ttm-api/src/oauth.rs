//! OAuth endpoints used by `ttm login` / `ttm logout`: authorization code + PKCE (loopback), device flow (RFC 8628),
//! refresh, and revocation (RFC 7009). The endpoints are unauthenticated form posts; URLs come from discovery.

use crate::{ApiError, Client, DeviceAuthorization, OAuthErrorBody, Result, TokenResponse};

/// The first-party client id of the CLI.
pub const CLI_CLIENT_ID: &str = "ttm-cli";

/// Scopes `ttm login` asks for by default.
pub const DEFAULT_SCOPES: &str = "workspace:read notee:read notee:write notee:comment factory:read factory:write factory:run offline_access";

pub const DEVICE_CODE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

impl Client {
    /// `grant_type=authorization_code` with the PKCE verifier.
    pub async fn exchange_code(
        &self,
        token_endpoint: &str,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse> {
        self.oauth_form(
            token_endpoint,
            &[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("code_verifier", code_verifier),
                ("client_id", CLI_CLIENT_ID),
            ],
        )
        .await
    }

    pub async fn refresh_token(&self, token_endpoint: &str, refresh_token: &str) -> Result<TokenResponse> {
        self.oauth_form(
            token_endpoint,
            &[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", CLI_CLIENT_ID),
            ],
        )
        .await
    }

    pub async fn device_authorization(&self, endpoint: &str, scope: &str) -> Result<DeviceAuthorization> {
        self.oauth_form(endpoint, &[("client_id", CLI_CLIENT_ID), ("scope", scope)])
            .await
    }

    /// One poll of the device flow. `authorization_pending` / `slow_down` come back as [`ApiError::OAuth`].
    pub async fn poll_device_token(&self, token_endpoint: &str, device_code: &str) -> Result<TokenResponse> {
        self.oauth_form(
            token_endpoint,
            &[
                ("grant_type", DEVICE_CODE_GRANT),
                ("device_code", device_code),
                ("client_id", CLI_CLIENT_ID),
            ],
        )
        .await
    }

    /// RFC 7009: revokes `token` (and, server side, its family).
    pub async fn revoke_token(&self, endpoint: &str, token: &str) -> Result<()> {
        let form = vec![
            ("token".to_string(), token.to_string()),
            ("client_id".into(), CLI_CLIENT_ID.into()),
        ];
        match self.post_form::<serde_json::Value>(endpoint, form).await {
            Ok(_) => Ok(()),
            Err(err) => Err(oauth_error(err)),
        }
    }

    async fn oauth_form<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        fields: &[(&str, &str)],
    ) -> Result<T> {
        let form = fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.post_form(url, form).await.map_err(oauth_error)
    }
}

/// RFC 6749 bodies (`{"error": "slow_down"}`) arrive as a generic problem; turn them into [`ApiError::OAuth`].
fn oauth_error(err: ApiError) -> ApiError {
    if let ApiError::Problem(problem) = &err
        && let Some(detail) = &problem.detail
        && let Ok(body) = serde_json::from_str::<OAuthErrorBody>(detail)
    {
        return ApiError::OAuth {
            status: problem.status,
            error: body.error,
            description: body.error_description,
        };
    }
    err
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Problem;

    #[test]
    fn rfc6749_body_becomes_oauth_error() {
        let err = ApiError::from_body(400, "application/json", r#"{"error":"slow_down"}"#);
        let err = oauth_error(err);
        assert_eq!(err.code(), "slow_down");
        assert!(matches!(err, ApiError::OAuth { status: 400, .. }));
    }

    #[test]
    fn problem_stays_problem() {
        let p = Problem {
            status: 429,
            code: "rate_limited".into(),
            title: "Too many".into(),
            detail: None,
            field: None,
            rule: None,
            params: None,
            required: None,
        };
        assert_eq!(oauth_error(ApiError::Problem(Box::new(p))).code(), "rate_limited");
    }
}
