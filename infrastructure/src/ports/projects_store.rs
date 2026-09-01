//! The persistence port for the projects slice.
//!
//! Implemented by `infra` (Postgres) and `testing` (in-memory fakes), same
//! object-safe boxed-future pattern as [`crate::ports::auth_store::AuthStore`].

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A project as shown to its owner/members.
#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    /// The project-level environment gate: `development` | `production`.
    pub environment: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub trait ProjectsStore: Send + Sync {
    /// All projects the user owns or belongs to, oldest first.
    fn list_projects(&self, user_id: UserId) -> BoxFut<'_, Result<Vec<ProjectSummary>, StoreError>>;

    /// Updates the environment gate on a project the user owns or belongs
    /// to; `None` when the project doesn't exist or isn't visible to them
    /// (indistinguishable on purpose — no resource enumeration).
    fn set_project_environment(
        &self,
        user_id: UserId,
        project_id: &str,
        environment: &str,
    ) -> BoxFut<'_, Result<Option<ProjectSummary>, StoreError>>;
}
