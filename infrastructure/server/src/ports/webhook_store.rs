//! The persistence port for owner-managed webhooks.
//!
//! Every webhook owns exactly one flow (`flow_id`, unique). Reads JOIN the
//! flow for its name so list views never issue a second query — mirroring
//! the schedule store's pattern.

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stored webhook row (flow name joined in).
#[derive(Debug, Clone)]
pub struct WebhookRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub flow_id: String,
    pub flow_name: String,
    pub events: serde_json::Value,
    pub active: bool,
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Insert fields for a validated webhook (the service creates the flow
/// first and passes its id in).
#[derive(Debug, Clone)]
pub struct NewWebhook {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub flow_id: String,
    pub events: serde_json::Value,
    pub active: bool,
    pub created_by: UserId,
}

/// Full replacement of a webhook's mutable columns (service resolves
/// patches into this before calling update).
#[derive(Debug, Clone)]
pub struct WebhookUpdate {
    pub name: String,
    pub description: String,
    pub events: serde_json::Value,
    pub active: bool,
    pub archived_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub trait WebhookStore: Send + Sync {
    /// Whether the actor owns or belongs to the project.
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// All webhooks for a project, oldest first.
    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<WebhookRecord>, StoreError>>;

    /// One webhook by id, if it exists.
    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<WebhookRecord>, StoreError>>;

    /// Flow ids owned by any webhook in the project (the flows list hides
    /// these — they are managed from their webhook's page).
    fn owned_flow_ids(&self, project_id: &str) -> BoxFut<'_, Result<Vec<String>, StoreError>>;

    /// Insert a webhook.
    fn create(&self, draft: &NewWebhook) -> BoxFut<'_, Result<WebhookRecord, StoreError>>;

    /// Replace a webhook's mutable columns (bumps updated_at).
    fn update(
        &self,
        id: &str,
        patch: &WebhookUpdate,
    ) -> BoxFut<'_, Result<Option<WebhookRecord>, StoreError>>;

    /// Delete a webhook row. The owned flow is deleted separately (the
    /// flow table cascades the row only on flow deletion, not here).
    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>>;
}
