use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::domain::admin::projects::ProjectDetail;
use crate::ports::projects_store::{AdminProjectRecord, ProjectMemberRecord};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminProjectDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub environment: String,
    pub owner_id: Option<String>,
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub member_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AdminProjectRecord> for AdminProjectDto {
    fn from(record: AdminProjectRecord) -> Self {
        Self {
            id: record.project.id,
            name: record.project.name,
            slug: record.project.slug,
            description: record.project.description,
            environment: record.project.environment,
            owner_id: record.owner_id,
            owner_name: record.owner_name,
            owner_email: record.owner_email,
            member_count: record.member_count,
            created_at: record.project.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberDto {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: String,
}

impl From<ProjectMemberRecord> for ProjectMemberDto {
    fn from(record: ProjectMemberRecord) -> Self {
        Self {
            user_id: record.user_id,
            name: record.name,
            email: record.email,
            role: record.role,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminProjectDetailDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub environment: String,
    pub owner_id: Option<String>,
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub member_count: i64,
    pub members: Vec<ProjectMemberDto>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProjectDetail> for AdminProjectDetailDto {
    fn from(detail: ProjectDetail) -> Self {
        Self {
            id: detail.project.id,
            name: detail.project.name,
            slug: detail.project.slug,
            description: detail.project.description,
            environment: detail.project.environment,
            owner_id: detail.owner_id,
            owner_name: detail.owner_name,
            owner_email: detail.owner_email,
            member_count: detail.member_count,
            members: detail
                .members
                .into_iter()
                .map(ProjectMemberDto::from)
                .collect(),
            created_at: detail.project.created_at,
            updated_at: detail.updated_at,
        }
    }
}
