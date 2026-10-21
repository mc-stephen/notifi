//! Admin project management — list/search all platform projects, view detail.

use std::sync::Arc;

use crate::domain::error::AppError;
use crate::domain::projects::entities::Project;
use crate::ports::projects_store::{ProjectMemberRecord, ProjectsStore};

/// A platform project plus owner identity, member count, and member roster.
#[derive(Debug, Clone)]
pub struct ProjectDetail {
    pub project: Project,
    pub owner_id: Option<String>,
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub member_count: i64,
    pub members: Vec<ProjectMemberRecord>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub struct AdminProjectsService {
    projects: Arc<dyn ProjectsStore>,
}

impl AdminProjectsService {
    pub fn new(projects: Arc<dyn ProjectsStore>) -> Self {
        Self { projects }
    }

    pub async fn list_projects(
        &self,
        search: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<crate::ports::projects_store::AdminProjectRecord>, i64), AppError> {
        Ok(self
            .projects
            .list_all_projects(search, limit, offset)
            .await?)
    }

    pub async fn get_project_detail(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectDetail>, AppError> {
        let Some(record) = self.projects.get_any_project(project_id).await? else {
            return Ok(None);
        };
        let members = self.projects.list_project_members(project_id).await?;
        Ok(Some(ProjectDetail {
            project: Project::from(record.project.clone()),
            owner_id: record.owner_id,
            owner_name: record.owner_name,
            owner_email: record.owner_email,
            member_count: record.member_count,
            members,
            updated_at: record.updated_at,
        }))
    }
}
