//! The persistence port for the recipients slice.
//!
//! Implemented by `infra` (Postgres) and `testing` (in-memory fakes), same
//! object-safe boxed-future pattern as `ProjectsStore`.

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A recipient as persisted by the store.
#[derive(Debug, Clone)]
pub struct RecipientRecord {
    pub id: String,
    pub project_id: String,
    /// The brand's in-house targeting key, unique within `project_id`.
    pub user_id: String,
    pub name: String,
    /// Flexible contact blob (email, phone, device/push ids, ...).
    pub contacts: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub trait RecipientsStore: Send + Sync {
    /// Persists a new recipient. A duplicate `(project_id, user_id)` yields
    /// [`StoreError::Conflict`].
    fn create(
        &self,
        actor: UserId,
        project_id: &str,
        user_id: &str,
        name: &str,
        contacts: serde_json::Value,
    ) -> BoxFut<'_, Result<RecipientRecord, StoreError>>;

    /// Recipients in a project the caller belongs to, newest first. `None`
    /// when the project isn't visible to `actor`.
    fn list(
        &self,
        actor: UserId,
        project_id: &str,
        search: Option<&str>,
        limit: i64,
        before: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<RecipientRecord>, StoreError>>;

    /// A single visible recipient; `None` when not found or not visible.
    fn get(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> BoxFut<'_, Result<Option<RecipientRecord>, StoreError>>;

    /// Updates name/contacts of a visible recipient (replaces `contacts`
    /// wholesale). Returns `None` when not found or not visible.
    fn update(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
        name: &str,
        contacts: serde_json::Value,
    ) -> BoxFut<'_, Result<Option<RecipientRecord>, StoreError>>;

    /// Soft-deletes a visible recipient. Returns whether anything was removed.
    fn remove(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;
}
