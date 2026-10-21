//! Recipient model shared by the service and the HTTP layer.

use serde_json::Value;

/// A recipient (a brand's end-user) owned by a project.
#[derive(Debug, Clone)]
pub struct Recipient {
    pub id: String,
    pub project_id: String,
    /// The brand's in-house targeting key, unique within `project_id`.
    pub user_id: String,
    pub name: String,
    /// Flexible contact blob (email, phone, device/push ids, ...).
    pub contacts: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<crate::ports::recipients_store::RecipientRecord> for Recipient {
    fn from(record: crate::ports::recipients_store::RecipientRecord) -> Self {
        Self {
            id: record.id,
            project_id: record.project_id,
            user_id: record.user_id,
            name: record.name,
            contacts: record.contacts,
            created_at: record.created_at,
        }
    }
}
