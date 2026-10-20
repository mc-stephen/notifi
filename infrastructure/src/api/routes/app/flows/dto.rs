//! Request/response shapes for project flow management (camelCase).
//!
//! `source` marks what owns the flow: `flow_page` (created on the flows
//! page and listed there), `schedule`, or `webhook` (owner-managed —
//! never listed, opened via the owner's page).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::flows::entities::Flow;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub definition: serde_json::Value,
    pub version: i32,
    /// Who owns/created the flow (see module docs).
    pub source: &'static str,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Owner tag for [`FlowDto::source`].
pub const SOURCE_FLOW_PAGE: &str = "flow_page";
pub const SOURCE_SCHEDULE: &str = "schedule";
pub const SOURCE_WEBHOOK: &str = "webhook";

impl From<Flow> for FlowDto {
    fn from(f: Flow) -> Self {
        Self {
            id: f.id,
            project_id: f.project_id,
            name: f.name,
            definition: f.definition,
            version: f.version,
            source: SOURCE_FLOW_PAGE,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFlowRequest {
    pub project_id: String,
    pub name: Option<String>,
    pub definition: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFlowRequest {
    pub project_id: String,
    pub name: String,
    pub definition: serde_json::Value,
}
