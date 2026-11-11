//! Last-ping probe store for inbound webhook doors.
//!
//! One row per trigger token: the most recent authorized fire's request
//! details (caller IP, whitelisted headers, capped body). The builder's
//! webhook trigger "Incoming data" tab reads it back. Capture is
//! best-effort — recording failures never fail the fire — and pings
//! that fail the door's checks (401/403) never reach the flow, so
//! they're never probed.

use chrono::{DateTime, Utc};

use crate::ports::auth_store::StoreError;
use crate::ports::trigger_token_store::BoxFut;

/// The last ping recorded for one webhook door.
#[derive(Debug, Clone)]
pub struct InboundProbeRecord {
    pub token: String,
    pub flow_id: String,
    pub project_id: String,
    pub client_ip: Option<String>,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
    pub forwarded_for: Option<String>,
    pub body: Option<String>,
    pub fired_at: DateTime<Utc>,
}

pub trait InboundProbeStore: Send + Sync {
    /// Upsert the door's last ping (the token is the key).
    fn record(&self, record: InboundProbeRecord) -> BoxFut<'_, Result<(), StoreError>>;

    /// The last ping for one door.
    fn last_for_token(
        &self,
        token: &str,
    ) -> BoxFut<'_, Result<Option<InboundProbeRecord>, StoreError>>;

    /// The most recent ping across a flow's doors (no token given).
    fn last_for_flow(
        &self,
        project_id: &str,
        flow_id: &str,
    ) -> BoxFut<'_, Result<Option<InboundProbeRecord>, StoreError>>;
}
