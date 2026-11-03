//! Workspaces and the signed-in user.

use crate::{Client, Me, Member, Result};

impl Client {
    /// `GET /me`: the user and their workspaces (scope `workspace:read`).
    pub async fn me(&self) -> Result<Me> {
        self.get("/me").await
    }

    /// `GET /workspaces/{id}/members`: members and guests, owner first. Guests get 403.
    pub async fn workspace_members(&self, workspace_id: &str) -> Result<Vec<Member>> {
        self.get(&format!("/workspaces/{}/members", encode(workspace_id)))
            .await
    }
}

/// Percent-encodes one path segment.
pub(crate) fn encode(segment: &str) -> String {
    url::form_urlencoded::byte_serialize(segment.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}
