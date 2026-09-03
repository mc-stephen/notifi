//! Request/response shapes for the recipients API (camelCase on the wire).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::recipients::Recipient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecipientRequest {
    /// The brand's in-house targeting key, unique within the project.
    pub user_id: String,
    pub name: String,
    /// Optional flexible contact blob (email, phone, device/push ids, ...).
    #[serde(default)]
    pub contacts: Option<serde_json::Value>,
}

/// PATCH body. `name` and `contacts` are optional; absent fields keep their
/// current value. When `contacts` is supplied it replaces the whole blob.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecipientRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub contacts: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipientDto {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub name: String,
    pub contacts: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl From<Recipient> for RecipientDto {
    fn from(recipient: Recipient) -> Self {
        Self {
            id: recipient.id,
            project_id: recipient.project_id,
            user_id: recipient.user_id,
            name: recipient.name,
            contacts: recipient.contacts,
            created_at: recipient.created_at,
        }
    }
}
