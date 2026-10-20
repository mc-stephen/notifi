//! Request/response shapes for owner-managed webhooks (camelCase).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::webhooks::entities::Webhook;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub flow_id: String,
    pub flow_name: String,
    pub events: Vec<String>,
    pub active: bool,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Webhook> for WebhookDto {
    fn from(w: Webhook) -> Self {
        Self {
            id: w.id,
            project_id: w.project_id,
            name: w.name,
            description: w.description,
            flow_id: w.flow_id,
            flow_name: w.flow_name,
            events: w.events,
            active: w.active,
            archived_at: w.archived_at,
            created_at: w.created_at,
            updated_at: w.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWebhookRequest {
    pub project_id: String,
    pub name: String,
    pub description: Option<String>,
    pub events: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebhookRequest {
    pub project_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub events: Option<Vec<String>>,
    pub active: Option<bool>,
    /// `true` archives (keeps the flow, retires the webhook), `false` restores.
    pub archived: Option<bool>,
}
