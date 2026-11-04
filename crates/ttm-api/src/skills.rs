//! Skills (S3): workspace skills, versions uploaded as deterministic zips, reviews, Git import.

use crate::Result;
use crate::client::{Body, Client};
use crate::workspace::encode;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// `GET /factory/skills[]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source: Option<Value>,
    #[serde(default)]
    pub verified: bool,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub current_version: Option<i64>,
    #[serde(default)]
    pub latest_version: Option<i64>,
    #[serde(default)]
    pub pending_versions: Option<Value>,
    #[serde(default)]
    pub versions: Vec<SkillVersion>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A version of a skill: `review_status` is `pending`, `approved`, `rejected` or `exempt`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillVersion {
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub review_status: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub manifest: Option<Value>,
    #[serde(default)]
    pub reviewed_by: Option<Value>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `GET /factory/skills/{id}/versions/{v}/diff`: `[{path, status: added|removed|changed|same, unified?}]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillDiff {
    #[serde(default)]
    pub files: Vec<SkillDiffFile>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillDiffFile {
    pub path: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub unified: Option<String>,
}

/// `POST /factory/skills` body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NewSkill {
    pub workspace_id: String,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// `POST /factory/skills/import-git` body.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SkillImport {
    pub workspace_id: String,
    pub url: String,
    pub path: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
}

/// Largest skill bundle accepted.
pub const MAX_SKILL_BYTES: usize = 5 * 1024 * 1024;

impl Client {
    /// `GET /factory/skills?workspace_id&q&tag&builtin=1`.
    pub async fn skills(
        &self,
        workspace_id: &str,
        query: Option<&str>,
        tag: Option<&str>,
        builtin: bool,
    ) -> Result<Vec<Skill>> {
        let mut q = vec![("workspace_id", workspace_id.to_string())];
        if let Some(v) = query {
            q.push(("q", v.to_string()));
        }
        if let Some(v) = tag {
            q.push(("tag", v.to_string()));
        }
        if builtin {
            q.push(("builtin", "1".into()));
        }
        self.get_query("/factory/skills", &q).await
    }

    pub async fn skill(&self, id: &str) -> Result<Skill> {
        self.get(&format!("/factory/skills/{}", encode(id))).await
    }

    pub async fn create_skill(&self, skill: &NewSkill) -> Result<Skill> {
        self.post("/factory/skills", skill).await
    }

    /// Uploads a version as `application/zip` with `X-TTM-Source: ttm` → `{version, sha256, size, review_status}`.
    pub async fn skill_push(&self, id: &str, zip: Vec<u8>, note: Option<&str>) -> Result<SkillVersion> {
        let headers = vec![("X-TTM-Source".to_string(), "ttm".to_string())];
        let query: Vec<(&str, String)> = note.map(|n| vec![("note", n.to_string())]).unwrap_or_default();
        self.json(
            Method::POST,
            &format!("/factory/skills/{}/versions", encode(id)),
            &query,
            Some(Body::Bytes {
                content_type: "application/zip".into(),
                headers,
                bytes: zip,
            }),
        )
        .await
    }

    pub async fn skill_version(&self, id: &str, version: i64) -> Result<SkillVersion> {
        self.get(&format!("/factory/skills/{}/versions/{version}", encode(id)))
            .await
    }

    /// Bundle bytes (`…/download` answers 302 to a signed URL; redirects are followed).
    pub async fn skill_download(&self, id: &str, version: i64) -> Result<Vec<u8>> {
        self.get_bytes(
            &format!("/factory/skills/{}/versions/{version}/download", encode(id)),
            MAX_SKILL_BYTES + 1024 * 1024,
        )
        .await
    }

    pub async fn skill_diff(&self, id: &str, version: i64, against: Option<i64>) -> Result<SkillDiff> {
        let q: Vec<(&str, String)> = against
            .map(|a| vec![("against", a.to_string())])
            .unwrap_or_default();
        self.get_query(
            &format!("/factory/skills/{}/versions/{version}/diff", encode(id)),
            &q,
        )
        .await
    }

    /// `decision`: `approve` or `reject` (a note is required to reject).
    pub async fn skill_review(
        &self,
        id: &str,
        version: i64,
        decision: &str,
        note: Option<&str>,
    ) -> Result<Value> {
        self.post(
            &format!("/factory/skills/{}/versions/{version}/review", encode(id)),
            &serde_json::json!({ "decision": decision, "note": note }),
        )
        .await
    }

    /// `GET /factory/skill-reviews?workspace_id`: versions waiting for review.
    pub async fn skill_reviews(&self, workspace_id: &str) -> Result<Vec<Value>> {
        self.get_query(
            "/factory/skill-reviews",
            &[("workspace_id", workspace_id.to_string())],
        )
        .await
    }

    pub async fn skill_import_git(&self, import: &SkillImport) -> Result<Value> {
        self.post("/factory/skills/import-git", import).await
    }
}
