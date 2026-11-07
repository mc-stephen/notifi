//! Admin user management — list/search platform users, view stats, suspend/restore.

use std::sync::Arc;

use serde_json::json;

use crate::domain::admin::entities::AdminUser;
use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::{User, UserId, UserStatus};
use crate::domain::error::AppError;
use crate::ports::auth_store::AuthStore;
use crate::ports::notifications_store::NotificationsStore;
use crate::ports::projects_store::ProjectsStore;
use crate::ports::tickets_store::TicketsStore;

/// A platform user plus admin-facing aggregates.
#[derive(Debug, Clone)]
pub struct UserDetail {
    pub user: User,
    pub project_count: i64,
    pub project_limit: i64,
    pub ticket_total: i64,
    pub tickets_open: i64,
    pub notification_count: i64,
}

pub struct AdminUsersService {
    auth: Arc<dyn AuthStore>,
    projects: Arc<dyn ProjectsStore>,
    notifications: Arc<dyn NotificationsStore>,
    tickets: Arc<dyn TicketsStore>,
    audit: Arc<AuditService>,
}

impl AdminUsersService {
    pub fn new(
        auth: Arc<dyn AuthStore>,
        projects: Arc<dyn ProjectsStore>,
        notifications: Arc<dyn NotificationsStore>,
        tickets: Arc<dyn TicketsStore>,
        audit: Arc<AuditService>,
    ) -> Self {
        Self {
            auth,
            projects,
            notifications,
            tickets,
            audit,
        }
    }

    pub async fn list_users(
        &self,
        search: Option<&str>,
        status: Option<UserStatus>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<User>, i64), AppError> {
        Ok(self.auth.list_users(search, status, limit, offset).await?)
    }

    pub async fn get_user_detail(&self, user_id: UserId) -> Result<Option<UserDetail>, AppError> {
        let Some(user) = self.auth.find_user_by_id(user_id).await? else {
            return Ok(None);
        };
        Ok(Some(self.detail_for(user).await?))
    }

    pub async fn set_user_status(
        &self,
        admin: &AdminUser,
        user_id: UserId,
        status: UserStatus,
    ) -> Result<UserDetail, AppError> {
        let user = self
            .auth
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".into()))?;

        if user.status != status {
            let updated = self.auth.set_user_status(user_id, status).await?;
            if !updated {
                return Err(AppError::NotFound("user not found".into()));
            }
            if status == UserStatus::Suspended {
                // Suspended accounts lose every session immediately.
                self.auth.revoke_all_sessions_for_user(user_id).await?;
            }

            let (action, message) = match status {
                UserStatus::Suspended => (
                    AuditAction::UserSuspended,
                    format!("admin '{}' suspended user '{}'", admin.email, user.email),
                ),
                UserStatus::Active => (
                    AuditAction::UserRestored,
                    format!("admin '{}' restored user '{}'", admin.email, user.email),
                ),
            };
            self.audit
                .record(
                    chrono::Utc::now(),
                    &AuditEvent::new_admin(
                        action,
                        &admin.id.to_string(),
                        Some(&admin.name),
                        None,
                        message,
                        Some(json!({ "user_id": user_id.to_string() })),
                    ),
                )
                .await;
        }

        self.get_user_detail(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".into()))
    }

    pub async fn set_project_limit(
        &self,
        admin: &AdminUser,
        user_id: UserId,
        limit: i64,
    ) -> Result<UserDetail, AppError> {
        // Deliberate range, not a nitpick: 0 would lock the account out
        // entirely and a million invites a spreadsheet mistake.
        if !(1..=1000).contains(&limit) {
            return Err(AppError::Validation(
                "project limit must be between 1 and 1000".to_string(),
            ));
        }
        let user = self
            .auth
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".into()))?;
        if !self.auth.set_project_limit(user_id, limit).await? {
            return Err(AppError::NotFound("user not found".into()));
        }
        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::UserProjectLimitChanged,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    None,
                    format!(
                        "admin '{}' set user '{}' project limit to {limit}",
                        admin.email, user.email
                    ),
                    Some(json!({ "user_id": user_id.to_string(), "limit": limit })),
                ),
            )
            .await;
        self.detail_for(user).await
    }

    pub async fn revoke_user_sessions(
        &self,
        admin: &AdminUser,
        user_id: UserId,
    ) -> Result<(), AppError> {
        let user = self
            .auth
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user not found".into()))?;
        self.auth.revoke_all_sessions_for_user(user_id).await?;
        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new_admin(
                    AuditAction::AdminSessionsRevoked,
                    &admin.id.to_string(),
                    Some(&admin.name),
                    None,
                    format!(
                        "admin '{}' revoked all sessions for user '{}'",
                        admin.email, user.email
                    ),
                    Some(json!({ "user_id": user_id.to_string() })),
                ),
            )
            .await;
        Ok(())
    }

    async fn detail_for(&self, user: User) -> Result<UserDetail, AppError> {
        // Owned + member projects; the ceiling rides along so the admin
        // console can read "owned / limit" from one record.
        let projects = self.projects.list_projects(user.id).await?;
        let project_limit = self.projects.project_limit(user.id).await?;
        let tickets = self
            .tickets
            .count_tickets_for_user(&user.id.to_string())
            .await?;
        let notification_count = self.notifications.count_all_for_user(user.id).await?;
        Ok(UserDetail {
            user,
            project_count: projects.len() as i64,
            project_limit,
            ticket_total: tickets.total,
            tickets_open: tickets.open,
            notification_count,
        })
    }
}
