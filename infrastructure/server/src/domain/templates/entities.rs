//! Template model shared by the service and the HTTP layer.

use serde_json::Value;

use crate::ports::templates_store::TemplateRecord;

/// A named, reusable message definition owned by a project.
///
/// Houses multiple per-channel content variants in the flexible `content`
/// blob (e.g. `subject`/`html`/`text` for email, `sms` for text, `push`
/// `{title, body}` for push/in-app) plus the primary `channel` hint.
#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub channel: String,
    pub content: Value,
    pub version: i32,
    pub attachments: Vec<Attachment>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<TemplateRecord> for Template {
    fn from(record: TemplateRecord) -> Self {
        Self {
            id: record.id,
            project_id: record.project_id,
            name: record.name,
            description: record.description,
            channel: record.channel,
            content: record.content,
            version: record.version,
            attachments: record
                .attachments
                .into_iter()
                .map(Attachment::from)
                .collect(),
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// An attachment referenced by a template (metadata + object-store URL).
#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub url: String,
}

impl From<crate::ports::templates_store::AttachmentRecord> for Attachment {
    fn from(record: crate::ports::templates_store::AttachmentRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            mime_type: record.mime_type,
            size_bytes: record.size_bytes,
            url: record.url,
        }
    }
}
