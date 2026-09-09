//! Admin notification broadcasts — compose to audiences, history, scheduling.

use std::collections::BTreeSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::domain::admin::entities::AdminUser;
use crate::domain::auth::entities::{UserId, UserStatus};
use crate::domain::auth::errors::AuthError;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::audit::AuditService;
use crate::domain::notifications::entities::NotificationType;
use crate::domain::notifications::services::validate_content;
use crate::ports::auth_store::AuthStore;
use crate::ports::notifications_store::{
    BroadcastRecord, BroadcastRecipient, BroadcastStats, BroadcastStatus, NotificationsStore,
};
use crate::ports::projects_store::ProjectsStore;

/// Who receives a broadcast.
#[derive(Debug, Clone)]
pub enum Audience {
    /// Every active platform user (snapshotted at send/schedule time).
    All,
    /// Explicit user ids.
    Users(Vec<UserId>),
    /// Owners + members of these projects.
    Projects(Vec<String>),
}

pub struct BroadcastInput {
    pub title: String,
    pub content: String,
    pub notification_type: NotificationType,
    /// Selected channels. Must include `in_app` (email/push delivery lands
    /// in M3 — selections are recorded but only in-app delivers today).
    pub channels: Vec<String>,
    pub audience: Audience,
    pub scheduled_for: Option<DateTime<Utc>>,
}

pub struct BroadcastResult {
    pub id: String,
    pub recipient_count: i64,
    pub scheduled: bool,
}

pub struct AdminNotificationsService {
    auth: Arc<dyn AuthStore>,
    projects: Arc<dyn ProjectsStore>,
    notifications: Arc<dyn NotificationsStore>,
    audit: Arc<AuditService>,
}

impl AdminNotificationsService {
    pub fn new(
        auth: Arc<dyn AuthStore>,
        projects: Arc<dyn ProjectsStore>,
        notifications: Arc<dyn NotificationsStore>,
        audit: Arc<AuditService>,
    ) -> Self {
        Self {
            auth,
            projects,
            notifications,
            audit,
        }
    }

    /// Composes a broadcast: resolves + snapshots the audience, persists the
    /// broadcast row, and either sends immediately or schedules it.
    pub async fn broadcast(
        &self,
        admin: &AdminUser,
        input: BroadcastInput,
    ) -> Result<BroadcastResult, AuthError> {
        let (title, content) = validate_content(&input.title, &input.content)?;

        let mut channels: Vec<String> = input
            .channels
            .iter()
            .map(|c| c.trim().to_lowercase())
            .filter(|c| !c.is_empty())
            .collect();
        channels.sort();
        channels.dedup();
        if channels.is_empty() || channels.iter().any(|c| c != "in_app" && c != "email") {
            return Err(AuthError::Validation(
                "channels must be a non-empty subset of in_app, email".to_string(),
            ));
        }
        if !channels.contains(&"in_app".to_string()) {
            return Err(AuthError::Validation(
                "email delivery is not implemented yet; select in-app to deliver".to_string(),
            ));
        }

        if let Some(at) = input.scheduled_for
            && at <= Utc::now()
        {
            return Err(AuthError::Validation(
                "scheduled_for must be in the future".to_string(),
            ));
        }

        let user_ids = self.resolve_audience(&input.audience).await?;
        if user_ids.is_empty() {
            return Err(AuthError::Validation(
                "audience matches no active users".to_string(),
            ));
        }

        let now = Utc::now();
        let scheduled = input.scheduled_for.is_some();
        let audience_json = audience_snapshot(&input.audience, &user_ids);
        let broadcast = BroadcastRecord {
            id: notifi_core::Ulid::new().to_string(),
            admin_id: admin.id.to_string(),
            admin_email: admin.email.to_string(),
            notification_type: input.notification_type,
            title: title.clone(),
            content: content.clone(),
            audience: audience_json,
            channels,
            status: if scheduled {
                BroadcastStatus::Scheduled
            } else {
                BroadcastStatus::Sent
            },
            scheduled_for: input.scheduled_for,
            sent_at: if scheduled { None } else { Some(now) },
            recipient_count: if scheduled { 0 } else { user_ids.len() as i64 },
            created_at: now,
        };
        self.notifications.create_broadcast(&broadcast).await?;

        if !scheduled {
            let recipients = recipient_rows(&user_ids, &title, &content);
            self.notifications
                .create_many(&broadcast.id, input.notification_type, &recipients)
                .await?;
        }

        let (action, message) = if scheduled {
            (
                AuditAction::NotificationScheduled,
                format!(
                    "admin '{}' scheduled '{}' to {} users via {}",
                    admin.email,
                    title,
                    user_ids.len(),
                    broadcast.channels.join(","),
                ),
            )
        } else {
            (
                AuditAction::NotificationBroadcast,
                format!(
                    "admin '{}' broadcast '{}' to {} users via {}",
                    admin.email,
                    title,
                    user_ids.len(),
                    broadcast.channels.join(","),
                ),
            )
        };
        self.audit
            .record(
                now,
                &AuditEvent::new(
                    action,
                    Some(&admin.id.to_string()),
                    None,
                    None,
                    message,
                    Some(json!({
                        "broadcast_id": broadcast.id,
                        "recipient_count": user_ids.len(),
                    })),
                ),
            )
            .await;

        Ok(BroadcastResult {
            id: broadcast.id,
            recipient_count: user_ids.len() as i64,
            scheduled,
        })
    }

    /// Cancels a scheduled broadcast. Only scheduled rows can be cancelled.
    pub async fn cancel_broadcast(&self, admin: &AdminUser, id: &str) -> Result<(), AuthError> {
        self.notifications
            .get_broadcast(id)
            .await?
            .ok_or_else(|| AuthError::NotFound("broadcast not found".into()))?;
        let cancelled = self.notifications.cancel_broadcast(id).await?;
        if !cancelled {
            return Err(AuthError::Conflict(
                "only scheduled broadcasts can be cancelled".to_string(),
            ));
        }
        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::NotificationCancelled,
                    Some(&admin.id.to_string()),
                    None,
                    None,
                    format!("admin '{}' cancelled broadcast '{id}'", admin.email),
                    Some(json!({ "broadcast_id": id })),
                ),
            )
            .await;
        Ok(())
    }

    pub async fn history(
        &self,
        status: Option<BroadcastStatus>,
        notification_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<BroadcastRecord>, i64), AuthError> {
        Ok(self
            .notifications
            .list_broadcasts(status, notification_type, limit, offset)
            .await?)
    }

    pub async fn detail(
        &self,
        id: &str,
    ) -> Result<Option<(BroadcastRecord, BroadcastStats)>, AuthError> {
        let Some(broadcast) = self.notifications.get_broadcast(id).await? else {
            return Ok(None);
        };
        let stats = self.notifications.broadcast_read_stats(id).await?;
        Ok(Some((broadcast, stats)))
    }

    /// Worker step: claims due broadcasts, materializes them, marks sent.
    /// Returns the number of broadcasts sent.
    pub async fn run_due_worker(&self, now: DateTime<Utc>) -> Result<i64, AuthError> {
        let due = self.notifications.claim_due_scheduled(now, 25).await?;
        let mut sent = 0i64;
        for broadcast in due {
            let user_ids = audience_user_ids(&broadcast.audience);
            let recipients = recipient_rows(&user_ids, &broadcast.title, &broadcast.content);
            let count = if recipients.is_empty() {
                0
            } else {
                self.notifications
                    .create_many(&broadcast.id, broadcast.notification_type, &recipients)
                    .await?
            };
            if self
                .notifications
                .mark_broadcast_sent(&broadcast.id, count)
                .await?
            {
                sent += 1;
            }
        }
        Ok(sent)
    }

    /// Resolves an audience to active user ids (deduped, stable order).
    async fn resolve_audience(&self, audience: &Audience) -> Result<Vec<UserId>, AuthError> {
        let mut ids = BTreeSet::new();
        match audience {
            Audience::All => {
                for id in self
                    .auth
                    .list_all_user_ids(Some(UserStatus::Active))
                    .await?
                {
                    ids.insert(id.to_string());
                }
            }
            Audience::Users(wanted) => {
                for id in wanted {
                    let user = self
                        .auth
                        .find_user_by_id(*id)
                        .await?
                        .ok_or_else(|| {
                            AuthError::Validation(format!("unknown user '{id}'"))
                        })?;
                    if user.status != UserStatus::Active {
                        return Err(AuthError::Validation(format!(
                            "user '{}' is not active",
                            user.email
                        )));
                    }
                    ids.insert(id.to_string());
                }
            }
            Audience::Projects(project_ids) => {
                for pid in project_ids {
                    let record = self
                        .projects
                        .get_any_project(pid)
                        .await?
                        .ok_or_else(|| AuthError::Validation(format!("unknown project '{pid}'")))?;
                    if let Some(owner) = record.owner_id
                        && self.is_active_id(&owner).await?
                    {
                        ids.insert(owner);
                    }
                    for member in self.projects.list_project_members(pid).await? {
                        if self.is_active_id(&member.user_id).await? {
                            ids.insert(member.user_id);
                        }
                    }
                }
            }
        }
        Ok(ids
            .into_iter()
            .filter_map(|id| id.parse::<UserId>().ok())
            .collect())
    }

    async fn is_active_id(&self, id: &str) -> Result<bool, AuthError> {
        let Ok(uid) = id.parse::<UserId>() else {
            return Ok(false);
        };
        Ok(self
            .auth
            .find_user_by_id(uid)
            .await?
            .is_some_and(|u| u.status == UserStatus::Active))
    }
}

/// Audience snapshot stored on the broadcast row (ids resolved at compose).
fn audience_snapshot(audience: &Audience, user_ids: &[UserId]) -> serde_json::Value {
    let kind = match audience {
        Audience::All => "all",
        Audience::Users(_) => "users",
        Audience::Projects(_) => "projects",
    };
    json!({
        "kind": kind,
        "user_ids": user_ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
    })
}

fn audience_user_ids(audience: &serde_json::Value) -> Vec<UserId> {
    audience
        .get("user_ids")
        .and_then(|v| v.as_array())
        .map(|ids| {
            ids.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| s.parse::<UserId>().ok())
                .collect()
        })
        .unwrap_or_default()
}

fn recipient_rows(user_ids: &[UserId], title: &str, content: &str) -> Vec<BroadcastRecipient> {
    user_ids
        .iter()
        .map(|id| BroadcastRecipient {
            user_id: id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
        })
        .collect()
}
