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
    UserSuspended,
    UserRestored,
    UserTotpEnabled,
    UserTotpDisabled,
    ProjectRequire2faChanged,
    ProjectMemberAdded,
    ProjectMemberInvited,
    ProjectInviteDeclined,
    AdminPasswordChanged,
    AdminCreated,
    AdminApprovalDecided,
    AdminLogin,
    AdminLogout,
    AdminTotpEnabled,
    AdminSessionsRevoked,
    NotificationBroadcast,
    NotificationScheduled,
    NotificationCancelled,
    ProjectEnvironmentChanged,
    RecipientCreated,
    RecipientUpdated,
    RecipientDeleted,
    TemplateCreated,
    TemplateUpdated,
    TemplateDeleted,
    SupportTicketCreated,
    SupportTicketReplied,
    SupportTicketStatusChanged,
    ProjectCreated,
    SubscriptionChanged,
    SubscriptionCancelled,
    PlanCreated,
    PlanUpdated,
    PlanDeleted,
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
            Self::SupportTicketStatusChanged => "support.ticket_status_changed",
            Self::SubscriptionChanged => "billing.subscription_changed",
            Self::SubscriptionCancelled => "billing.subscription_cancelled",
            Self::PlanCreated => "billing.plan_created",
            Self::PlanUpdated => "billing.plan_updated",
            Self::PlanDeleted => "billing.plan_deleted",
            Self::UserSuspended => "user.suspended",
            Self::UserRestored => "user.restored",
            Self::UserTotpEnabled => "user.totp_enabled",
            Self::UserTotpDisabled => "user.totp_disabled",
            Self::ProjectRequire2faChanged => "project.require_2fa_changed",
            Self::ProjectMemberAdded => "project.member_added",
            Self::ProjectMemberInvited => "project.member_invited",
            Self::ProjectInviteDeclined => "project.invite_declined",
            Self::AdminPasswordChanged => "admin.password_changed",
            Self::AdminCreated => "admin.created",
            Self::AdminApprovalDecided => "admin.approval_decided",
            Self::AdminLogin => "admin.login",
            Self::AdminLogout => "admin.logout",
            Self::AdminTotpEnabled => "admin.totp_enabled",
            Self::AdminSessionsRevoked => "admin.sessions_revoked",
            Self::NotificationBroadcast => "notification.broadcast",
            Self::NotificationScheduled => "notification.scheduled",
            Self::NotificationCancelled => "notification.cancelled",
            Self::ProjectCreated => "project.created",
        }
    }
}

/// Who performed an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    #[default]
    User,
    Admin,
    System,
}

impl ActorType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Admin => "admin",
            Self::System => "system",
        }
    }
}

impl std::str::FromStr for ActorType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Self::User),
            "admin" => Ok(Self::Admin),
            "system" => Ok(Self::System),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An immutable, append-only audit record.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub id: String,
    /// The actor who performed the action; `None` for system/background.
    pub user_id: Option<String>,
    /// Admin actor id, when `actor_type` is admin.
    pub admin_id: Option<String>,
    pub actor_type: ActorType,
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
    pub admin_id: Option<String>,
    pub actor_type: ActorType,
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
            admin_id: None,
            actor_type: ActorType::User,
            actor_name: actor_name.map(str::to_owned),
            project_id: project_id.map(str::to_owned),
            message,
            metadata,
        }
    }

    /// Attribute the event to a platform admin instead of a user.
    pub fn with_admin(mut self, admin_id: &str, actor_name: Option<&str>) -> Self {
        self.user_id = None;
        self.admin_id = Some(admin_id.to_owned());
        self.actor_type = ActorType::Admin;
        if actor_name.is_some() {
            self.actor_name = actor_name.map(str::to_owned);
        }
        self
    }

    /// Convenience for admin-attributed events.
    pub fn new_admin(
        action: AuditAction,
        admin_id: &str,
        actor_name: Option<&str>,
        project_id: Option<&str>,
        message: String,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            action,
            user_id: None,
            admin_id: Some(admin_id.to_owned()),
            actor_type: ActorType::Admin,
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
            admin_id: event.admin_id.clone(),
            actor_type: event.actor_type,
            actor_name: event.actor_name.clone(),
            event_type: event.action.event_type().to_string(),
            message: event.message.clone(),
            project_id: event.project_id.clone(),
            metadata: event.metadata.clone(),
            occurred_at,
        }
    }
}
