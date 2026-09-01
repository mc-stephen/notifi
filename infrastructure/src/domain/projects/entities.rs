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
            created_at: summary.created_at,
        }
    }
}
