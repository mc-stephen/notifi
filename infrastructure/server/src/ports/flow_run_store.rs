//! Run-history store for flow executions.
//!
//! One row per completed run of a flow (webhook fire, schedule run,
//! event-triggered send): status, trigger mode, terminal error,
//! start/finish stamps, the send row it produced, and the `steps` trace
//! (per-node outcomes as JSON — see `domain::flows::runs::RunStep`).
//! v1 writes the row once at run end, so there are no dangling
//! "running" rows; recording failures never fail the delivery.

use chrono::{DateTime, Utc};

use crate::ports::auth_store::StoreError;
use crate::ports::trigger_token_store::BoxFut;

/// One completed flow run.
#[derive(Debug, Clone)]
pub struct FlowRunRecord {
    pub id: String,
    pub project_id: String,
    pub flow_id: String,
    /// webhook | schedule | event
    pub mode: String,
    /// succeeded | failed
    pub status: String,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    /// The send row the run produced, when one exists.
    pub send_id: Option<String>,
    /// JSON array of step objects ({nodeId, kind, label, status, detail, ms}).
    pub steps: serde_json::Value,
}

pub trait FlowRunStore: Send + Sync {
    /// Records one completed run.
    fn insert(&self, record: FlowRunRecord) -> BoxFut<'_, Result<(), StoreError>>;

    /// A flow's runs, newest first.
    fn list(
        &self,
        project_id: &str,
        flow_id: &str,
        limit: i64,
    ) -> BoxFut<'_, Result<Vec<FlowRunRecord>, StoreError>>;

    /// One run, scoped to its flow.
    fn get(
        &self,
        project_id: &str,
        flow_id: &str,
        run_id: &str,
    ) -> BoxFut<'_, Result<Option<FlowRunRecord>, StoreError>>;
}
