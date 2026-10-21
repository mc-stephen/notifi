//! The persistence port for product notification sends.
//!
//! One row per `POST /v1/notifications` call, or per campaign-expanded
//! recipient. Instant sends execute inline and land completed/failed;
//! scheduled rows and campaign rows wait for the workers.

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
    /// Set when this row was expanded from a campaign (one row per
    /// recipient). Null for direct API sends and schedule runs.
    pub campaign_id: Option<String>,
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

    /// Inserts one `pending` row for a campaign expansion — a single
    /// recipient per row so per-recipient status can be derived by the
    /// campaign's stats. Created here; executed by the campaign worker.
    fn create_for_campaign(
        &self,
        project_id: &str,
        campaign_id: &str,
        channel: &str,
        recipient: &str,
        title: &str,
        body: &str,
    ) -> BoxFut<'_, Result<SendRecord, StoreError>>;

    /// All sends expanded from a campaign, oldest first (stats derive
    /// from these rows — nothing is ever double-stored).
    fn list_by_campaign(
        &self,
        campaign_id: &str,
    ) -> BoxFut<'_, Result<Vec<SendRecord>, StoreError>>;

    /// Deletes a campaign's not-yet-fired rows (`pending` only) — the
    /// cancel action. Delivered/failed rows stay as delivery history.
    /// Returns how many rows were removed.
    fn delete_pending_by_campaign(&self, campaign_id: &str) -> BoxFut<'_, Result<i64, StoreError>>;
}
