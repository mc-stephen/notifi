//! Device model shared by the service and the HTTP layer.

use crate::define_id;

// Typed identifier for `project_devices.id` (docs live on the macro
// expansion; see `kernel::id::define_id`).
define_id!(DeviceId);

/// A registered device: destination + vendor + the vendor's token.
#[derive(Debug, Clone)]
pub struct Device {
    pub id: DeviceId,
    pub project_id: String,
    /// The recipient this device delivers to, when assigned.
    pub recipient_id: Option<String>,
    /// Device destination (`android`, `apple`, `linux`, `web`, `window`).
    pub destination: String,
    /// The vendor whose credentials deliver to this token.
    pub vendor: String,
    /// The vendor registration token or endpoint URI.
    pub token: String,
    pub label: Option<String>,
    /// `active` or `revoked`.
    pub status: String,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<crate::ports::devices_store::DeviceRecord> for Device {
    fn from(record: crate::ports::devices_store::DeviceRecord) -> Self {
        Self {
            id: record
                .id
                .parse()
                .expect("store ids are canonical ULIDs; ULID survives string round-trips"),
            project_id: record.project_id,
            recipient_id: record.recipient_id,
            destination: record.destination,
            vendor: record.vendor,
            token: record.token,
            label: record.label,
            status: record.status,
            last_seen_at: record.last_seen_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}
