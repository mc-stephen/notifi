use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    TeamAdd,
    TeamRemove,
    RoleChange,
    ProviderAdd,
    ProviderDelete,
    ApiKeyCreated,
    ApiKeyRevoked,
    ProjectCreated,
    BillingChange,
    #[default]
    System,
    NewLogin,
    Marketing,
    Billing,
    Welcome,
    Update,
}

impl NotificationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TeamAdd => "team_add",
            Self::TeamRemove => "team_remove",
            Self::RoleChange => "role_change",
            Self::ProviderAdd => "provider_add",
            Self::ProviderDelete => "provider_delete",
            Self::ApiKeyCreated => "api_key_created",
            Self::ApiKeyRevoked => "api_key_revoked",
            Self::ProjectCreated => "project_created",
            Self::BillingChange => "billing_change",
            Self::System => "system",
            Self::NewLogin => "new_login",
            Self::Marketing => "marketing",
            Self::Billing => "billing",
            Self::Welcome => "welcome",
            Self::Update => "update",
        }
    }
}

impl FromStr for NotificationType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "team_add" => Ok(Self::TeamAdd),
            "team_remove" => Ok(Self::TeamRemove),
            "role_change" => Ok(Self::RoleChange),
            "provider_add" => Ok(Self::ProviderAdd),
            "provider_delete" => Ok(Self::ProviderDelete),
            "api_key_created" => Ok(Self::ApiKeyCreated),
            "api_key_revoked" => Ok(Self::ApiKeyRevoked),
            "project_created" => Ok(Self::ProjectCreated),
            "billing_change" => Ok(Self::BillingChange),
            "system" => Ok(Self::System),
            "new_login" => Ok(Self::NewLogin),
            "marketing" => Ok(Self::Marketing),
            "billing" => Ok(Self::Billing),
            "welcome" => Ok(Self::Welcome),
            "update" => Ok(Self::Update),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationOrigin {
    #[default]
    System,
    Admin,
}

impl NotificationOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Admin => "admin",
        }
    }
}

impl FromStr for NotificationOrigin {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(Self::System),
            "admin" => Ok(Self::Admin),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for NotificationOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct InAppNotification {
    pub id: String,
    pub user_id: String,
    pub notification_type: NotificationType,
    pub origin: NotificationOrigin,
    pub title: String,
    pub content: String,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl InAppNotification {
    pub fn is_read(&self) -> bool {
        self.read_at.is_some()
    }
}

impl From<crate::ports::notifications_store::NotificationRecord> for InAppNotification {
    fn from(record: crate::ports::notifications_store::NotificationRecord) -> Self {
        Self {
            id: record.id,
            user_id: record.user_id,
            notification_type: record.notification_type,
            origin: record.origin,
            title: record.title,
            content: record.content,
            read_at: record.read_at,
            created_at: record.created_at,
            deleted_at: record.deleted_at,
        }
    }
}
