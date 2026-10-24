//! Request/response shapes for the devices API (camelCase on the wire).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::devices::Device;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceRequest {
    /// Device destination (`android`, `apple`, `linux`, `web`, `window`).
    pub destination: String,
    /// The vendor whose credentials deliver to this token.
    pub vendor: String,
    /// The vendor registration token or endpoint URI.
    pub token: String,
    #[serde(default)]
    pub label: Option<String>,
    /// Optional recipient assignment — `None` registers unassigned and the
    /// dashboard can target the device through the recipient later.
    #[serde(default)]
    pub recipient_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDto {
    pub id: String,
    pub project_id: String,
    pub recipient_id: Option<String>,
    pub destination: String,
    pub vendor: String,
    pub token: String,
    pub label: Option<String>,
    pub status: String,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Device> for DeviceDto {
    fn from(device: Device) -> Self {
        Self {
            id: device.id.to_string(),
            project_id: device.project_id,
            recipient_id: device.recipient_id,
            destination: device.destination,
            vendor: device.vendor,
            token: device.token,
            label: device.label,
            status: device.status,
            last_seen_at: device.last_seen_at,
            created_at: device.created_at,
            updated_at: device.updated_at,
        }
    }
}
