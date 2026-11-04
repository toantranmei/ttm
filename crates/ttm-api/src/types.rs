//! Response and request types of `/api/v1`, after `openapi/v1.json`. Unknown fields are ignored so newer servers keep
//! working with older clients.

use serde::{Deserialize, Serialize};

/// `GET /me`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Me {
    pub user: MeUser,
    #[serde(default)]
    pub workspaces: Vec<WorkspaceSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeUser {
    pub id: String,
    pub email: String,
    pub name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub plan: Option<String>,
    /// Mei-er membership (`null` for non-members and older servers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership: Option<Membership>,
}

/// `user.membership` of `GET /me`: `{tier, status, since, until, badge}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Membership {
    /// `supporter`, `mei-er`, `sponsor`.
    pub tier: String,
    /// `active` or `grace`.
    #[serde(default)]
    pub status: Option<String>,
    /// RFC 3339.
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub badge: Option<String>,
}

impl Membership {
    pub fn tier(&self) -> Option<crate::vocab::Tier> {
        crate::vocab::Tier::parse(&self.tier)
    }
}

/// `membership: {tier}` (people) or `owner_membership: {tier}` (agents, workspaces) in lists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TierRef {
    pub tier: String,
}

impl TierRef {
    pub fn tier(&self) -> Option<crate::vocab::Tier> {
        crate::vocab::Tier::parse(&self.tier)
    }
}

/// A workspace the user belongs to (`/me.workspaces[]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    /// `personal` or `team`.
    pub kind: String,
    /// `owner`, `admin`, `member` or `guest`.
    pub role: String,
    /// Short URL name (added with team workspaces; absent on older servers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// Membership of the workspace owner (badge next to the workspace).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_membership: Option<TierRef>,
}

impl WorkspaceSummary {
    /// Whether `needle` names this workspace: id, slug, or name (case-insensitive).
    pub fn matches(&self, needle: &str) -> bool {
        let needle = needle.trim();
        self.id == needle
            || self
                .slug
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case(needle))
            || self.name.eq_ignore_ascii_case(needle)
    }
}

/// `GET /workspaces/{id}/members[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub user: MemberUser,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership: Option<TierRef>,
}

impl Member {
    /// The member's tier, on the member or on its user.
    pub fn tier(&self) -> Option<crate::vocab::Tier> {
        self.membership
            .as_ref()
            .or(self.user.membership.as_ref())
            .and_then(TierRef::tier)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberUser {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membership: Option<TierRef>,
}

/// `POST /oauth/token` success.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default = "bearer")]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

fn bearer() -> String {
    "Bearer".into()
}

impl std::fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenResponse")
            .field("token_type", &self.token_type)
            .field("expires_in", &self.expires_in)
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

/// `POST /oauth/device_authorization`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    #[serde(default = "five")]
    pub interval: u64,
}

fn five() -> u64 {
    5
}

/// RFC 6749 §5.2 error body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthErrorBody {
    pub error: String,
    #[serde(default)]
    pub error_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn me_parses_server_shape() {
        let json = serde_json::json!({
            "user": {"id": "u1", "email": "a@b.c", "name": "An", "avatar_url": null, "locale": "vi", "plan": "free"},
            "workspaces": [
                {"id": "w1", "name": "Cá nhân", "kind": "personal", "role": "owner", "daily_key": "k"},
                {"id": "w2", "name": "Team Aurora", "kind": "team", "role": "member", "slug": "aurora"}
            ]
        });
        let me: Me = serde_json::from_value(json).unwrap();
        assert_eq!(me.workspaces.len(), 2);
        assert!(me.workspaces[1].matches("aurora"));
        assert!(me.workspaces[1].matches("team aurora"));
        assert!(me.workspaces[0].matches("w1"));
        assert!(!me.workspaces[0].matches("w2"));
    }

    #[test]
    fn token_response_debug_hides_secrets() {
        let t = TokenResponse {
            access_token: "ttm_oat_SECRET".into(),
            token_type: "Bearer".into(),
            expires_in: Some(60),
            refresh_token: Some("ttm_ort_SECRET".into()),
            scope: None,
        };
        assert!(!format!("{t:?}").contains("SECRET"));
    }

    #[test]
    fn device_interval_defaults_to_five() {
        let d: DeviceAuthorization = serde_json::from_value(serde_json::json!({
            "device_code": "d", "user_code": "ABCD-EFGH", "verification_uri": "https://x/activate", "expires_in": 600
        }))
        .unwrap();
        assert_eq!(d.interval, 5);
    }
}
