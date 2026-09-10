//! The persistence port for the billing slice.
//!
//! Billing is per project. User-scoped methods enforce project
//! visibility inside the store (same rule as recipients/templates:
//! owner or member, invisible projects behave as missing).
//! Implemented by `infra` (Postgres) and `testing` (in-memory fakes).

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::domain::billing::entities::{BillingCycle, SubscriptionStatus};
use crate::domain::billing::{NewPlan, UpdatePlan};
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A live project with its creation time, for history replay.
pub type ProjectBirth = (String, chrono::DateTime<chrono::Utc>);

/// A plan row. `capabilities` / `yearly_discount` are raw JSON; the
/// domain parses them into [`crate::domain::billing::Plan`].
#[derive(Debug, Clone)]
pub struct PlanRecord {
    pub id: String,
    pub name: String,
    pub price_cents: Option<i64>,
    pub currency: String,
    pub interval: String,
    pub capabilities: Option<serde_json::Value>,
    pub yearly_discount: Option<serde_json::Value>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// One project's subscription row (plan resolved by the service).
#[derive(Debug, Clone)]
pub struct SubscriptionRecord {
    pub id: String,
    pub project_id: String,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub billing_cycle: BillingCycle,
    /// Paid plan ends at period end, then reverts to free.
    pub cancel_at_period_end: bool,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A subscription plus the billed project and its owner (admin views).
/// Owner fields are `None` when the creator account was deleted.
#[derive(Debug, Clone)]
pub struct SubscriberRecord {
    pub subscription: SubscriptionRecord,
    pub project_name: String,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
}

pub trait BillingStore: Send + Sync {
    /// Whether the actor owns or belongs to the project.
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Active plans for the customer catalog.
    fn list_active_plans(&self) -> BoxFut<'_, Result<Vec<PlanRecord>, StoreError>>;

    /// Any plan by id (active or archived).
    fn get_plan(&self, plan_id: &str) -> BoxFut<'_, Result<Option<PlanRecord>, StoreError>>;

    /// The project's subscription row, if one exists.
    fn get_subscription(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>>;

    /// Insert the free monthly row for a project that has none. Called
    /// after a visibility check; a racing insert surfaces as `Conflict`
    /// and the caller re-reads.
    fn ensure_free_subscription(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<SubscriptionRecord, StoreError>>;

    /// Create or replace the project's subscription row (new period).
    #[allow(clippy::too_many_arguments)]
    fn set_subscription(
        &self,
        project_id: &str,
        plan_id: &str,
        status: SubscriptionStatus,
        billing_cycle: BillingCycle,
        period_start: chrono::DateTime<chrono::Utc>,
        period_end: chrono::DateTime<chrono::Utc>,
    ) -> BoxFut<'_, Result<SubscriptionRecord, StoreError>>;

    /// Flip a subscription's status. `None` when the row is missing.
    fn set_subscription_status(
        &self,
        project_id: &str,
        status: SubscriptionStatus,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>>;

    /// Mark a subscription to end at period end (or clear the mark).
    /// `None` when the row is missing.
    fn set_cancel_at_period_end(
        &self,
        project_id: &str,
        cancel: bool,
    ) -> BoxFut<'_, Result<Option<SubscriptionRecord>, StoreError>>;

    // === Admin-scoped methods (no actor visibility checks) =================

    /// Every plan, active and archived.
    fn list_all_plans(&self) -> BoxFut<'_, Result<Vec<PlanRecord>, StoreError>>;

    /// Active + past-due subscriptions on a plan (blocks hard delete).
    fn count_active_subscribers(&self, plan_id: &str) -> BoxFut<'_, Result<i64, StoreError>>;

    /// Every live project with its creation time, for history replay.
    fn list_projects_with_created(&self) -> BoxFut<'_, Result<Vec<ProjectBirth>, StoreError>>;

    /// Active + past-due subscriptions on a plan with project + owner
    /// identity, newest first (admin drill-down).
    fn list_subscribers(
        &self,
        plan_id: &str,
    ) -> BoxFut<'_, Result<Vec<SubscriberRecord>, StoreError>>;

    fn create_plan(&self, id: &str, input: &NewPlan) -> BoxFut<'_, Result<PlanRecord, StoreError>>;

    fn update_plan(
        &self,
        plan_id: &str,
        input: &UpdatePlan,
    ) -> BoxFut<'_, Result<Option<PlanRecord>, StoreError>>;

    fn delete_plan(&self, plan_id: &str) -> BoxFut<'_, Result<bool, StoreError>>;
}
