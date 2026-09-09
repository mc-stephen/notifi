use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::admin::notifications::BroadcastResult;
use crate::ports::notifications_store::{BroadcastRecord, BroadcastStats, BroadcastStatus};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastDto {
    pub id: String,
    pub admin_email: String,
    pub notification_type: String,
    pub title: String,
    pub content: String,
    pub audience_kind: Option<String>,
    pub channels: Vec<String>,
    pub status: BroadcastStatus,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub recipient_count: i64,
    pub read_count: i64,
    pub created_at: DateTime<Utc>,
}

impl BroadcastDto {
    pub fn with_stats(record: BroadcastRecord, stats: BroadcastStats) -> Self {
        Self {
            id: record.id,
            admin_email: record.admin_email,
            notification_type: record.notification_type.to_string(),
            title: record.title,
            content: record.content,
            audience_kind: record
                .audience
                .get("kind")
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            channels: record.channels,
            status: record.status,
            scheduled_for: record.scheduled_for,
            sent_at: record.sent_at,
            recipient_count: record.recipient_count.max(stats.total),
            read_count: stats.read,
            created_at: record.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastCreatedDto {
    pub id: String,
    pub recipient_count: i64,
    pub scheduled: bool,
}

impl From<BroadcastResult> for BroadcastCreatedDto {
    fn from(result: BroadcastResult) -> Self {
        Self {
            id: result.id,
            recipient_count: result.recipient_count,
            scheduled: result.scheduled,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudienceRequest {
    pub kind: String,
    #[serde(default)]
    pub user_ids: Vec<String>,
    #[serde(default)]
    pub project_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastRequest {
    pub title: String,
    pub content: String,
    #[serde(default = "default_broadcast_type")]
    pub notification_type: String,
    #[serde(default = "default_channels")]
    pub channels: Vec<String>,
    pub audience: AudienceRequest,
    pub scheduled_for: Option<DateTime<Utc>>,
}

fn default_broadcast_type() -> String {
    "system".to_string()
}

fn default_channels() -> Vec<String> {
    vec!["in_app".to_string()]
}
