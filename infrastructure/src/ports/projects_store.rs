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

    /// Creates a new project owned by the user.
    fn create_project(
        &self,
        user_id: UserId,
        name: &str,
        description: Option<&str>,
    ) -> BoxFut<'_, Result<ProjectSummary, StoreError>>;

    /// Updates the environment gate on a project the user owns or belongs
    /// to; `None` when the project doesn't exist or isn't visible to them
    /// (indistinguishable on purpose — no resource enumeration).
    fn set_project_environment(
        &self,
        user_id: UserId,
        project_id: &str,
        environment: &str,
    ) -> BoxFut<'_, Result<Option<ProjectSummary>, StoreError>>;

    // === Admin-scoped reads (no actor visibility checks) =================

    /// All projects (newest first) with owner identity + member count.
    /// Returns the page plus the total matching count.
    fn list_all_projects(
        &self,
        search: Option<&str>,
        environment: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> BoxFut<'_, Result<(Vec<AdminProjectRecord>, i64), StoreError>>;

    /// Any project by id, with owner identity + member count.
    fn get_any_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<AdminProjectRecord>, StoreError>>;

    /// Members of a project with identity, for admin views.
    fn list_project_members(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<ProjectMemberRecord>, StoreError>>;
}

/// A project plus its owner's identity, for admin views. Owner fields are
/// `None` when the creator account was deleted.
#[derive(Debug, Clone)]
pub struct AdminProjectRecord {
    pub project: ProjectSummary,
    pub owner_id: Option<String>,
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub member_count: i64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// One project membership with the member's identity, for admin views.
#[derive(Debug, Clone)]
pub struct ProjectMemberRecord {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: String,
}
