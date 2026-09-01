//! Request/response shapes for the projects API (camelCase on the wire).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::projects::Project;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEnvironmentRequest {
    pub environment: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub environment: String,
    pub created_at: DateTime<Utc>,
}

impl From<Project> for ProjectDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            slug: project.slug,
            description: project.description,
            environment: project.environment,
            created_at: project.created_at,
        }
    }
}
