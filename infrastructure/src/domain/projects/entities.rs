//! Project model shared by the service and the HTTP layer.

use chrono::{DateTime, Utc};

/// A project visible to a user (owner or member).
#[derive(Debug, Clone)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub environment: String,
    pub require_2fa: bool,
    pub created_at: DateTime<Utc>,
}

impl From<crate::ports::projects_store::ProjectSummary> for Project {
    fn from(summary: crate::ports::projects_store::ProjectSummary) -> Self {
        Self {
            id: summary.id,
            name: summary.name,
            slug: summary.slug,
            description: summary.description,
            environment: summary.environment,
            require_2fa: summary.require_2fa,
            created_at: summary.created_at,
        }
    }
}

/// A membership created through the team invite flow.
#[derive(Debug, Clone)]
pub struct ProjectMember {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub has_2fa: bool,
    pub last_active_at: Option<DateTime<Utc>>,
}

/// A pending team invite (returned without the token, except in dev).
#[derive(Debug, Clone)]
pub struct ProjectInvite {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub email: String,
    pub role: String,
    pub expires_at: DateTime<Utc>,
}
