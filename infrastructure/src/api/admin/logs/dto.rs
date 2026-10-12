use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::audit::entities::AuditEntry;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLogDto {
    pub id: String,
    pub user_id: Option<String>,
    pub admin_id: Option<String>,
    pub actor_type: String,
    pub actor_name: Option<String>,
    pub event_type: String,
    pub message: String,
    pub project_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
}

impl From<AuditEntry> for AdminLogDto {
    fn from(entry: AuditEntry) -> Self {
        Self {
            id: entry.id,
            user_id: entry.user_id,
            admin_id: entry.admin_id,
            actor_type: entry.actor_type.as_str().to_string(),
            actor_name: entry.actor_name,
            event_type: entry.event_type,
            message: entry.message,
            project_id: entry.project_id,
            metadata: entry.metadata,
            occurred_at: entry.occurred_at,
        }
    }
}
