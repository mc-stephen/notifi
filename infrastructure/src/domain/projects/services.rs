//! Project use cases: listing and the environment gate.

use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::audit::AuditService;
use crate::domain::projects::entities::Project;
use crate::ports::auth_store::StoreError;
use crate::ports::projects_store::ProjectsStore;

/// Allowed project gate values (mirrors the schema CHECK).
pub const ENVIRONMENTS: [&str; 2] = ["development", "production"];

pub struct ProjectService {
    store: Arc<dyn ProjectsStore>,
    audit: Arc<AuditService>,
}

impl ProjectService {
    pub fn new(store: Arc<dyn ProjectsStore>, audit: Arc<AuditService>) -> Self {
        Self { store, audit }
    }

    /// Projects the user owns or belongs to, oldest first.
    pub async fn list(&self, user_id: UserId) -> Result<Vec<Project>, AuthError> {
        Ok(self
            .store
            .list_projects(user_id)
            .await?
            .into_iter()
            .map(Project::from)
            .collect())
    }

    /// Creates a new project.
    pub async fn create(
        &self,
        user_id: UserId,
        name: &str,
        description: Option<&str>,
    ) -> Result<Project, AuthError> {
        if name.trim().is_empty() {
            return Err(AuthError::Validation("project name is required".into()));
        }
        let record = self
            .store
            .create_project(user_id, name, description)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AuthError::Conflict("a project with this name already exists".into()),
                StoreError::Storage(m) => AuthError::Storage(m),
            })?;

        let project = Project::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectCreated,
                    Some(&user_id.to_string()),
                    None,
                    Some(&project.id),
                    format!("project '{}' created", project.name),
                    None,
                ),
            )
            .await;

        Ok(project)
    }

    /// Switches the project's environment gate.
    pub async fn set_environment(
        &self,
        user_id: UserId,
        project_id: &str,
        environment: &str,
    ) -> Result<Project, AuthError> {
        if !ENVIRONMENTS.contains(&environment) {
            return Err(AuthError::Validation(format!(
                "environment must be one of: {}",
                ENVIRONMENTS.join(", ")
            )));
        }

        let project = self
            .store
            .set_project_environment(user_id, project_id, environment)
            .await?
            .map(Project::from)
            .ok_or_else(|| AuthError::NotFound("project not found".to_string()))?;

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::ProjectEnvironmentChanged,
                    Some(&user_id.to_string()),
                    None,
                    Some(project_id),
                    format!("environment switched to {environment}"),
                    None,
                ),
            )
            .await;

        Ok(project)
    }
}
