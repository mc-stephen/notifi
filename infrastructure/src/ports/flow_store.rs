//! The persistence port for notification flow definitions.
//!
//! Flows v2: every project flow lives here. "Default" is emergent — any
//! flow carrying a `notification.created` trigger qualifies for sends —
//! so there is no system flag, only plain CRUD scoped by project.

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stored flow definition row.
#[derive(Debug, Clone)]
pub struct FlowRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub definition: serde_json::Value,
    pub version: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub trait FlowStore: Send + Sync {
    /// Whether the actor owns or belongs to the project.
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// All flows for a project, oldest first.
    fn list_by_project(&self, project_id: &str) -> BoxFut<'_, Result<Vec<FlowRecord>, StoreError>>;

    /// One flow by id, if it exists.
    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<FlowRecord>, StoreError>>;

    /// Insert a flow.
    fn create(
        &self,
        project_id: &str,
        name: &str,
        definition: &serde_json::Value,
    ) -> BoxFut<'_, Result<FlowRecord, StoreError>>;

    /// Replace a flow's name/definition (bumps version).
    fn update(
        &self,
        id: &str,
        name: &str,
        definition: &serde_json::Value,
    ) -> BoxFut<'_, Result<Option<FlowRecord>, StoreError>>;

    /// Delete a flow. Returns false when there was nothing to delete.
    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>>;
}
