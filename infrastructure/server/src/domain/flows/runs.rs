//! Flow run history: one row per execution of a flow (webhook fire,
//! schedule run, event-triggered send).
//!
//! The builder's History tab reads runs back: the list shows status,
//! trigger mode, timing, and the terminal error; a run's detail replays
//! the `steps` trace — one entry per node outcome (trigger, template
//! render, provider attempt, HTTP step) with its duration and detail.
//! Recording is best-effort end to end: a failed history write never
//! fails the delivery it describes.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::kernel::id::Ulid;
use crate::ports::flow_run_store::FlowRunStore;

pub const RUN_SUCCEEDED: &str = "succeeded";
pub const RUN_FAILED: &str = "failed";
pub const MODE_WEBHOOK: &str = "webhook";
pub const MODE_SCHEDULE: &str = "schedule";
pub const MODE_EVENT: &str = "event";

/// One node's outcome in a run's steps trace.
#[derive(Debug, Clone)]
pub struct RunStep {
    pub node_id: String,
    /// trigger | template | provider | http-request | resolve
    pub kind: String,
    /// Human label (provider id, template name, method + url…).
    pub label: String,
    /// ok | failed | skipped
    pub status: String,
    pub detail: Option<String>,
    pub ms: u64,
}

impl RunStep {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "nodeId": self.node_id,
            "kind": self.kind,
            "label": self.label,
            "status": self.status,
            "detail": self.detail,
            "ms": self.ms,
        })
    }
}

fn steps_json(steps: &[RunStep]) -> serde_json::Value {
    serde_json::Value::Array(steps.iter().map(RunStep::to_json).collect())
}

/// Best-effort recorder over the run store: recording failures are
/// logged and swallowed — history must not break delivery.
pub struct RunRecorder {
    runs: Arc<dyn FlowRunStore>,
}

impl RunRecorder {
    pub fn new(runs: Arc<dyn FlowRunStore>) -> Self {
        Self { runs }
    }

    /// Records one completed run (v1 writes the row once, at the end —
    /// no "running" rows to leak on a crash mid-flight).
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        &self,
        mode: &str,
        project_id: &str,
        flow_id: &str,
        status: &str,
        error: Option<String>,
        steps: &[RunStep],
        send_id: Option<&str>,
        started_at: DateTime<Utc>,
    ) {
        let record = crate::ports::flow_run_store::FlowRunRecord {
            id: Ulid::new().to_string(),
            project_id: project_id.to_string(),
            flow_id: flow_id.to_string(),
            mode: mode.to_string(),
            status: status.to_string(),
            error,
            started_at,
            finished_at: Some(Utc::now()),
            send_id: send_id.map(str::to_string),
            steps: steps_json(steps),
        };
        if let Err(e) = self.runs.insert(record).await {
            tracing::warn!(flow_id = %flow_id, error = %e, "run history write failed");
        }
    }

    fn map_store_error(e: crate::ports::auth_store::StoreError) -> crate::domain::error::AppError {
        match e {
            crate::ports::auth_store::StoreError::Storage(m) => {
                crate::domain::error::AppError::Storage(m)
            }
            crate::ports::auth_store::StoreError::Conflict => {
                crate::domain::error::AppError::Conflict("conflicting run change".into())
            }
        }
    }

    /// A flow's runs, newest first (the builder's History tab).
    pub async fn list(
        &self,
        project_id: &str,
        flow_id: &str,
        limit: i64,
    ) -> Result<Vec<crate::ports::flow_run_store::FlowRunRecord>, crate::domain::error::AppError>
    {
        self.runs
            .list(project_id, flow_id, limit)
            .await
            .map_err(Self::map_store_error)
    }

    /// One run, scoped to its flow.
    pub async fn get(
        &self,
        project_id: &str,
        flow_id: &str,
        run_id: &str,
    ) -> Result<Option<crate::ports::flow_run_store::FlowRunRecord>, crate::domain::error::AppError>
    {
        self.runs
            .get(project_id, flow_id, run_id)
            .await
            .map_err(Self::map_store_error)
    }
}
