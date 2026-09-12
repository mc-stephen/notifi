//! Request/response shapes for the projects API (camelCase on the wire).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::projects::{Project, ProjectInvite, ProjectMember};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEnvironmentRequest {
    pub environment: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRequire2faRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMemberRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberDto {
    pub user_id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub has_2fa: bool,
    pub last_active_at: Option<DateTime<Utc>>,
}

impl From<ProjectMember> for ProjectMemberDto {
    fn from(member: ProjectMember) -> Self {
        Self {
            user_id: member.user_id,
            name: member.name,
            email: member.email,
            role: member.role,
            has_2fa: member.has_2fa,
            last_active_at: member.last_active_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInviteDto {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub email: String,
    pub role: String,
    pub expires_at: DateTime<Utc>,
}

impl From<ProjectInvite> for ProjectInviteDto {
    fn from(invite: ProjectInvite) -> Self {
        Self {
            id: invite.id,
            project_id: invite.project_id,
            project_name: invite.project_name,
            email: invite.email,
            role: invite.role,
            expires_at: invite.expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub environment: String,
    pub require_2fa: bool,
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
            require_2fa: project.require_2fa,
            created_at: project.created_at,
        }
    }
}
