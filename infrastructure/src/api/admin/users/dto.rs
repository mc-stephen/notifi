use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::admin::users::UserDetail;
use crate::domain::auth::entities::{User, UserStatus};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDto {
    pub id: String,
    pub name: String,
    pub email: String,
    pub avatar: Option<String>,
    pub email_verified: bool,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<User> for AdminUserDto {
    fn from(user: User) -> Self {
        let email_verified = user.email_verified();
        Self {
            id: user.id.to_string(),
            name: user.name,
            email: user.email.to_string(),
            avatar: user.avatar_url,
            email_verified,
            status: user.status,
            created_at: user.created_at,
            last_login_at: user.last_login_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDetailDto {
    #[serde(flatten)]
    pub user: AdminUserDto,
    pub project_count: i64,
    pub ticket_total: i64,
    pub tickets_open: i64,
    pub notification_count: i64,
}

impl From<UserDetail> for AdminUserDetailDto {
    fn from(detail: UserDetail) -> Self {
        Self {
            user: AdminUserDto::from(detail.user),
            project_count: detail.project_count,
            ticket_total: detail.ticket_total,
            tickets_open: detail.tickets_open,
            notification_count: detail.notification_count,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetUserStatusRequest {
    pub status: String,
}
