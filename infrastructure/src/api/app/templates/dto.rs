//! Request/response shapes for the templates API (camelCase on the wire).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::templates::{Attachment, Template};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTemplateRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_channel")]
    pub channel: String,
    /// Per-representation content object (subject/html/text/sms/push).
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    /// Attachment metadata (name, mimeType, sizeBytes, url) — no upload here.
    #[serde(default)]
    pub attachments: Vec<AttachmentInputDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTemplateRequest {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub attachments: Option<Vec<AttachmentInputDto>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInputDto {
    pub name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
    pub url: String,
}

fn default_channel() -> String {
    "email".to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDto {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub url: String,
}

impl From<Attachment> for AttachmentDto {
    fn from(a: Attachment) -> Self {
        Self {
            id: a.id,
            name: a.name,
            mime_type: a.mime_type,
            size_bytes: a.size_bytes,
            url: a.url,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub channel: String,
    pub content: serde_json::Value,
    pub version: i32,
    pub attachments: Vec<AttachmentDto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Template> for TemplateDto {
    fn from(t: Template) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            name: t.name,
            description: t.description,
            channel: t.channel,
            content: t.content,
            version: t.version,
            attachments: t.attachments.into_iter().map(AttachmentDto::from).collect(),
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}
