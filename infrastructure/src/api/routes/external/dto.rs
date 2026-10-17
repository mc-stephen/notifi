//! Request/response shapes for the external product API (camelCase).
//!
//! v0 authenticates with the dashboard session (API keys land in M6).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::sends::entities::Send;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRequest {
    pub project_id: String,
    pub channel: String,
    pub recipients: Vec<String>,
    pub title: String,
    pub body: String,
    /// ISO timestamp. Omitted/now = instant; future times are refused
    /// (scheduled sends arrive in Phase B).
    pub send_at: Option<DateTime<Utc>>,
}

/// Inbound webhook fire. Everything optional — caller values win, the
/// trigger node's stored config and its template fill the gaps.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundFireRequest {
    pub channel: Option<String>,
    pub recipients: Option<Vec<String>>,
    pub title: Option<String>,
    pub body: Option<String>,
    pub template_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendDto {
    pub id: String,
    pub project_id: String,
    pub channel: String,
    pub recipients: Vec<String>,
    pub title: String,
    pub status: String,
    pub provider: Option<String>,
    pub external_id: Option<String>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Send> for SendDto {
    fn from(s: Send) -> Self {
        Self {
            id: s.id,
            project_id: s.project_id,
            channel: s.channel,
            recipients: s.recipients,
            title: s.title,
            status: s.status.as_str().to_string(),
            provider: s.provider,
            external_id: s.external_id,
            last_error: s.last_error,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}
