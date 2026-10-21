//! Project API keys: issuance records (hash-only), per-key IP allowlists,
//! and the verification seam the external API will authenticate against.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ports::api_keys_store::ApiKeyRecord;

/// An issued key as seen by the dashboard — never the raw secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKey {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub prefix: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ApiKeyRecord> for ApiKey {
    fn from(record: ApiKeyRecord) -> Self {
        Self {
            id: record.id,
            project_id: record.project_id,
            name: record.name,
            prefix: record.prefix,
            permissions: record.permissions,
            expires_at: record.expires_at,
            last_used_at: record.last_used_at,
            enabled: record.enabled,
            revoked_at: record.revoked_at,
            created_at: record.created_at,
        }
    }
}

/// One allowlist entry on a key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpAllowEntry {
    pub id: String,
    pub key_id: String,
    pub cidr: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

impl From<crate::ports::api_keys_store::IpAllowEntry> for IpAllowEntry {
    fn from(record: crate::ports::api_keys_store::IpAllowEntry) -> Self {
        Self {
            id: record.id,
            key_id: record.key_id,
            cidr: record.cidr,
            label: record.label,
            created_at: record.created_at,
        }
    }
}
