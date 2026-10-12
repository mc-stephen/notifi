//! The persistence port for flow schedules.
//!
//! Schedules own their assigned flow (`flow_id`) plus the execution
//! payload the worker needs. Reads always JOIN the flow for its name and
//! definition so callers never issue a second query.

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stored schedule row (flow + template joined in).
#[derive(Debug, Clone)]
pub struct ScheduleRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub flow_id: String,
    pub flow_name: String,
    pub flow_definition: serde_json::Value,
    pub kind: String,
    pub run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub interval_secs: Option<i64>,
    pub days: Vec<i64>,
    pub run_time: Option<String>,
    pub timezone: String,
    pub template_id: Option<String>,
    pub template_name: Option<String>,
    pub template_content: Option<serde_json::Value>,
    pub channel: String,
    pub title: String,
    pub body: String,
    pub recipients: serde_json::Value,
    pub active: bool,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
    pub run_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Insert fields for a validated schedule (service computes `next_run_at`).
#[derive(Debug, Clone)]
pub struct NewSchedule {
    pub project_id: String,
    pub name: String,
    pub flow_id: String,
    pub kind: String,
    pub run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub interval_secs: Option<i64>,
    pub days: Vec<i64>,
    pub run_time: Option<String>,
    pub timezone: String,
    pub template_id: Option<String>,
    pub channel: String,
    pub title: String,
    pub body: String,
    pub recipients: serde_json::Value,
    pub active: bool,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Full replacement of a schedule's mutable columns (service resolves
/// patches into this before calling update).
#[derive(Debug, Clone)]
pub struct ScheduleUpdate {
    pub name: String,
    pub flow_id: String,
    pub kind: String,
    pub run_at: Option<chrono::DateTime<chrono::Utc>>,
    pub interval_secs: Option<i64>,
    pub days: Vec<i64>,
    pub run_time: Option<String>,
    pub timezone: String,
    pub template_id: Option<String>,
    pub channel: String,
    pub title: String,
    pub body: String,
    pub recipients: serde_json::Value,
    pub active: bool,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub trait ScheduleStore: Send + Sync {
    /// Whether the actor owns or belongs to the project.
    fn project_visible(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// All schedules for a project, oldest first.
    fn list_by_project(
        &self,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<ScheduleRecord>, StoreError>>;

    /// One schedule by id, if it exists.
    fn get(&self, id: &str) -> BoxFut<'_, Result<Option<ScheduleRecord>, StoreError>>;

    /// Flow ids referenced by any schedule in the project (the flows list
    /// hides these — they are managed via schedules).
    fn scheduled_flow_ids(&self, project_id: &str) -> BoxFut<'_, Result<Vec<String>, StoreError>>;

    /// Insert a schedule.
    fn create(&self, draft: &NewSchedule) -> BoxFut<'_, Result<ScheduleRecord, StoreError>>;

    /// Replace a schedule's mutable columns (bumps updated_at).
    fn update(
        &self,
        id: &str,
        patch: &ScheduleUpdate,
    ) -> BoxFut<'_, Result<Option<ScheduleRecord>, StoreError>>;

    /// Delete a schedule. Returns false when there was nothing to delete.
    fn delete(&self, id: &str) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Atomically claim due schedules (active, next_run_at elapsed),
    /// stamping last_run_at so a second worker skips them. Oldest first.
    fn claim_due(
        &self,
        now: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<ScheduleRecord>, StoreError>>;

    /// Record a run outcome: bumps run_count, stamps last run, advances
    /// next_run_at (None stops a once schedule; recurring keeps going),
    /// and deactivates when `active` is false.
    #[allow(clippy::too_many_arguments)]
    fn mark_run(
        &self,
        id: &str,
        last_status: &str,
        last_error: Option<&str>,
        next_run_at: Option<chrono::DateTime<chrono::Utc>>,
        active: bool,
    ) -> BoxFut<'_, Result<Option<ScheduleRecord>, StoreError>>;
}
