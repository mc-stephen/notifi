//! The persistence port for product notification sends.
//!
//! One row per `POST /v1/notifications` call. Instant sends execute inline
//! and land completed/failed; scheduled rows wait for the Phase B worker.

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stored send row.
#[derive(Debug, Clone)]
pub struct SendRecord {
    pub id: String,
    pub project_id: String,
    pub channel: String,
    pub recipients: serde_json::Value,
    pub title: String,
    pub body: String,
    pub send_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub attempts: i32,
    pub provider: Option<String>,
    pub external_id: Option<String>,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub trait SendStore: Send + Sync {
    /// Whether the actor owns or belongs to the project.
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Insert a send row in `sending` state. Returns the row.
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        project_id: &str,
        channel: &str,
        recipients: &serde_json::Value,
        title: &str,
        body: &str,
        send_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFut<'_, Result<SendRecord, StoreError>>;

    /// Mark a send completed with the winning provider (+ external id).
    fn mark_completed(
        &self,
        id: &str,
        provider: &str,
        external_id: Option<&str>,
    ) -> BoxFut<'_, Result<Option<SendRecord>, StoreError>>;

    /// Mark a send failed with the terminal error.
    fn mark_failed(
        &self,
        id: &str,
        last_error: &str,
    ) -> BoxFut<'_, Result<Option<SendRecord>, StoreError>>;

    /// Newest-first sends for a project (delivery history).
    fn list_by_project(
        &self,
        project_id: &str,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<SendRecord>, StoreError>>;
}
