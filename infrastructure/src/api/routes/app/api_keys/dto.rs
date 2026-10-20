use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::api_keys::entities::{ApiKey, IpAllowEntry};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateApiKeyRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIpRequest {
    pub cidr: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyDto {
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

impl From<ApiKey> for ApiKeyDto {
    fn from(key: ApiKey) -> Self {
        Self {
            id: key.id,
            project_id: key.project_id,
            name: key.name,
            prefix: key.prefix,
            permissions: key.permissions,
            expires_at: key.expires_at,
            last_used_at: key.last_used_at,
            enabled: key.enabled,
            revoked_at: key.revoked_at,
            created_at: key.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpAllowEntryDto {
    pub id: String,
    pub key_id: String,
    pub cidr: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

impl From<IpAllowEntry> for IpAllowEntryDto {
    fn from(entry: IpAllowEntry) -> Self {
        Self {
            id: entry.id,
            key_id: entry.key_id,
            cidr: entry.cidr,
            label: entry.label,
            created_at: entry.created_at,
        }
    }
}
