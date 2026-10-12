//! Response shapes for send history (camelCase).

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::sends::entities::Send;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendHistoryDto {
    pub id: String,
    pub project_id: String,
    pub channel: String,
    pub recipient_count: usize,
    pub title: String,
    pub status: String,
    pub provider: Option<String>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<Send> for SendHistoryDto {
    fn from(s: Send) -> Self {
        Self {
            recipient_count: s.recipients.len(),
            id: s.id,
            project_id: s.project_id,
            channel: s.channel,
            title: s.title,
            status: s.status.as_str().to_string(),
            provider: s.provider,
            last_error: s.last_error,
            created_at: s.created_at,
        }
    }
}
