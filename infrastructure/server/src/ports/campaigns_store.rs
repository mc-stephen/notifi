//! The persistence port for campaigns.
//!
//! A campaign is a message origin: one channel × one audience × one
//! message × one timing. It owns no flow — fan-out lands in
//! `notification_sends` rows keyed by `campaign_id`, those rows deliver
//! directly through the project's connected providers, and campaigns
//! carry only identity, composition, and lifecycle state. Derived stats
//! are computed from sends; nothing here is stored twice.

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stored campaign row.
#[derive(Debug, Clone)]
pub struct CampaignRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub channel: String,
    pub template_id: Option<String>,
    pub title: String,
    pub body: String,
    /// Who gets it: "all" (channel-capable recipients) or "custom".
    pub audience_mode: String,
    pub selected_user_ids: serde_json::Value,
    /// Explicit primary provider pick (empty = no explicit pick).
    pub provider_id: String,
    pub status: String,
    pub send_at: Option<chrono::DateTime<chrono::Utc>>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Insert fields for a validated campaign (service validates and resolves
/// template renders before calling create).
#[derive(Debug, Clone)]
pub struct NewCampaign {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub created_by: String,
    pub channel: String,
    pub template_id: Option<String>,
    pub title: String,
    pub body: String,
    pub audience_mode: String,
    pub selected_user_ids: serde_json::Value,
    pub provider_id: String,
    pub status: String,
    pub send_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Full replacement of a campaign's mutable columns (service resolves
/// patches into this before calling update).
#[derive(Debug, Clone)]
pub struct CampaignUpdate {
    pub name: String,
    pub description: String,
    pub channel: String,
    pub template_id: Option<String>,
    pub title: String,
    pub body: String,
    pub audience_mode: String,
    pub selected_user_ids: serde_json::Value,
    pub provider_id: String,
    pub send_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub trait CampaignStore: Send + Sync {
    /// Whether the actor owns or belongs to the project.
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// All campaigns for a project, oldest first.
    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<CampaignRecord>, StoreError>>;

    /// One campaign by id, if it exists.
    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<CampaignRecord>, StoreError>>;

    /// Insert a campaign.
    fn create(&self, draft: &NewCampaign) -> BoxFut<'_, Result<CampaignRecord, StoreError>>;

    /// Replace a campaign's mutable columns (bumps updated_at).
    fn update(
        &self,
        id: &str,
        patch: &CampaignUpdate,
    ) -> BoxFut<'_, Result<Option<CampaignRecord>, StoreError>>;

    /// Delete a campaign. Returns false when there was nothing to delete.
    /// Expanded send rows are NOT deleted — delivery history stays.
    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Atomically claim scheduled campaigns whose `send_at` elapsed,
    /// flipping them to `running` and stamping `started_at` so a second
    /// worker skips them. Oldest first.
    fn claim_due(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<CampaignRecord>, StoreError>>;

    /// Running campaigns, oldest first — the worker drains their
    /// pending send rows in batches.
    fn list_running(&self, limit: i64) -> BoxFut<'_, Result<Vec<CampaignRecord>, StoreError>>;

    /// Lifecycle flip with stamping: sets status, optionally stamps
    /// started_at / completed_at (/values ignored when None) and records
    /// last_error. Returns the updated row.
    fn transition(
        &self,
        id: &str,
        status: &str,
        started_at: Option<chrono::DateTime<chrono::Utc>>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
        last_error: Option<&str>,
    ) -> BoxFut<'_, Result<Option<CampaignRecord>, StoreError>>;
}
