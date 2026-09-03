//! Response shapes for the audit-log API (camelCase on the wire).

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::audit::AuditEntry;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogDto {
    pub id: String,
    pub user_id: Option<String>,
    pub actor_name: Option<String>,
    pub event_type: String,
    pub message: String,
    pub project_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
}

impl From<AuditEntry> for LogDto {
    fn from(entry: AuditEntry) -> Self {
        Self {
            id: entry.id,
            user_id: entry.user_id,
            actor_name: entry.actor_name,
            event_type: entry.event_type,
            message: entry.message,
            project_id: entry.project_id,
            metadata: entry.metadata,
            occurred_at: entry.occurred_at,
        }
    }
}
