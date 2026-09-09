//! Admin user management — list/search platform users, view stats, suspend/restore.

use std::sync::Arc;

use serde_json::json;

use crate::domain::admin::entities::AdminUser;
use crate::domain::auth::entities::{User, UserId, UserStatus};
use crate::domain::auth::errors::AuthError;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::audit::AuditService;
use crate::ports::auth_store::AuthStore;
use crate::ports::notifications_store::NotificationsStore;
use crate::ports::projects_store::ProjectsStore;
use crate::ports::tickets_store::TicketsStore;

/// A platform user plus admin-facing aggregates.
#[derive(Debug, Clone)]
pub struct UserDetail {
    pub user: User,
    pub project_count: i64,
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
    ) -> Result<(Vec<User>, i64), AuthError> {
        Ok(self.auth.list_users(search, status, limit, offset).await?)
    }

    pub async fn get_user_detail(&self, user_id: UserId) -> Result<Option<UserDetail>, AuthError> {
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
    ) -> Result<UserDetail, AuthError> {
        let user = self
            .auth
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AuthError::NotFound("user not found".into()))?;

        if user.status != status {
            let updated = self.auth.set_user_status(user_id, status).await?;
            if !updated {
                return Err(AuthError::NotFound("user not found".into()));
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
            .ok_or_else(|| AuthError::NotFound("user not found".into()))
    }

    pub async fn revoke_user_sessions(&self, admin: &AdminUser, user_id: UserId) -> Result<(), AuthError> {
        let user = self.auth
            .find_user_by_id(user_id)
            .await?
            .ok_or_else(|| AuthError::NotFound("user not found".into()))?;
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

    async fn detail_for(&self, user: User) -> Result<UserDetail, AuthError> {
        let projects = self.projects.list_projects(user.id).await?;
        let tickets = self.tickets.count_tickets_for_user(&user.id.to_string()).await?;
        let notification_count = self.notifications.count_all_for_user(user.id).await?;
        Ok(UserDetail {
            user,
            project_count: projects.len() as i64,
            ticket_total: tickets.total,
            tickets_open: tickets.open,
            notification_count,
        })
    }
}
