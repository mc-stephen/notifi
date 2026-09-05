use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::notifications::entities::{NotificationOrigin, NotificationType};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InAppNotificationDto {
    pub id: String,
    pub user_id: String,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub origin: NotificationOrigin,
    pub title: String,
    pub content: String,
    pub read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<crate::domain::notifications::InAppNotification> for InAppNotificationDto {
    fn from(n: crate::domain::notifications::InAppNotification) -> Self {
        let read = n.read_at.is_some();
        Self {
            id: n.id,
            user_id: n.user_id,
            notification_type: n.notification_type,
            origin: n.origin,
            title: n.title,
            content: n.content,
            read,
            read_at: n.read_at,
            created_at: n.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotificationsQuery {
    #[serde(default)]
    pub unread_only: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub before: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetReadRequest {
    pub read: bool,
}
