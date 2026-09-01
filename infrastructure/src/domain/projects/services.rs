//! Project use cases: listing and the environment gate.

use std::sync::Arc;

use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::projects::entities::Project;
use crate::ports::projects_store::ProjectsStore;

/// Allowed project gate values (mirrors the schema CHECK).
pub const ENVIRONMENTS: [&str; 2] = ["development", "production"];

pub struct ProjectService {
    store: Arc<dyn ProjectsStore>,
}

impl ProjectService {
    pub fn new(store: Arc<dyn ProjectsStore>) -> Self {
        Self { store }
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

        self.store
            .set_project_environment(user_id, project_id, environment)
            .await?
            .map(Project::from)
            .ok_or_else(|| AuthError::NotFound("project not found".to_string()))
    }
}
