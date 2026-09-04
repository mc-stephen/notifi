//! Audit log entity: an append-only record of a system action.

use chrono::{DateTime, Utc};
use serde::Serialize;

/// A kind of system action recorded in the audit log.
///
/// `event_type` uses the `<domain>.<past_tense>` convention shared with the
/// event model; `message` is a human-readable summary. Never store secrets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditAction {
    UserSignup,
    UserLogin,
    UserLogout,
    UserPasswordReset,
    UserEmailVerified,
    UserOnboardingCompleted,
    ProjectEnvironmentChanged,
    RecipientCreated,
    RecipientUpdated,
    RecipientDeleted,
    TemplateCreated,
    TemplateUpdated,
    TemplateDeleted,
    SupportTicketCreated,
    SupportTicketReplied,
}

impl AuditAction {
    pub fn event_type(self) -> &'static str {
        match self {
            Self::UserSignup => "user.signup",
            Self::UserLogin => "user.login",
            Self::UserLogout => "user.logout",
            Self::UserPasswordReset => "user.password_reset",
            Self::UserEmailVerified => "user.email_verified",
            Self::UserOnboardingCompleted => "user.onboarding_completed",
            Self::ProjectEnvironmentChanged => "project.environment_changed",
            Self::RecipientCreated => "recipient.created",
            Self::RecipientUpdated => "recipient.updated",
            Self::RecipientDeleted => "recipient.deleted",
            Self::TemplateCreated => "template.created",
            Self::TemplateUpdated => "template.updated",
            Self::TemplateDeleted => "template.deleted",
            Self::SupportTicketCreated => "support.ticket_created",
            Self::SupportTicketReplied => "support.ticket_replied",
        }
    }
}

/// An immutable, append-only audit record.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub id: String,
    /// The actor who performed the action; `None` for system/background.
    pub user_id: Option<String>,
    pub actor_name: Option<String>,
    pub event_type: String,
    pub message: String,
    /// Project the action took place in, if any.
    pub project_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub occurred_at: DateTime<Utc>,
}

/// An action to record, built by the emitting service and handed to
/// [`crate::domain::audit::AuditService`]. Bundles the fields so both the
/// service and the store constructors stay lean.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub action: AuditAction,
    pub user_id: Option<String>,
    pub actor_name: Option<String>,
    pub project_id: Option<String>,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
}

impl AuditEvent {
    pub fn new(
        action: AuditAction,
        user_id: Option<&str>,
        actor_name: Option<&str>,
        project_id: Option<&str>,
        message: String,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            action,
            user_id: user_id.map(str::to_owned),
            actor_name: actor_name.map(str::to_owned),
            project_id: project_id.map(str::to_owned),
            message,
            metadata,
        }
    }
}

impl AuditEntry {
    pub fn new(id: String, event: &AuditEvent, occurred_at: DateTime<Utc>) -> Self {
        Self {
            id,
            user_id: event.user_id.clone(),
            actor_name: event.actor_name.clone(),
            event_type: event.action.event_type().to_string(),
            message: event.message.clone(),
            project_id: event.project_id.clone(),
            metadata: event.metadata.clone(),
            occurred_at,
        }
    }
}
