//! The persistence port for the devices slice.
//!
//! Implemented by `infra` (PostgreSQL), same object-safe boxed-future
//! pattern as `RecipientsStore`. Every project-scoped method authorizes
//! the caller at the SQL level (owner or active member — no handlers may
//! bypass the actor).

use std::future::Future;
use std::pin::Pin;

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

/// Boxed future returned by every port method.
pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A device record as persisted by the store.
#[derive(Debug, Clone)]
pub struct DeviceRecord {
    pub id: String,
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

/// What the caller hands over to register (or refresh) a device.
#[derive(Debug, Clone)]
pub struct DeviceRegistration {
    pub destination: String,
    pub vendor: String,
    pub token: String,
    pub label: Option<String>,
    pub recipient_id: Option<String>,
}

pub trait DevicesStore: Send + Sync {
    /// Registers a device, or refreshes the existing record when the same
    /// `(project_id, destination, token)` re-registers: recipient, label,
    /// status (`active`) and timestamps update — the row never duplicates.
    /// A project invisible to `actor`, or a `recipient_id` that doesn't
    /// belong to the project, yields [`StoreError::Storage`] with the
    /// "project not found or not visible" / "recipient not found"
    /// messages.
    fn register(
        &self,
        actor: UserId,
        project_id: &str,
        registration: DeviceRegistration,
    ) -> BoxFut<'_, Result<DeviceRecord, StoreError>>;

    /// Devices in a project the caller belongs to, newest first,
    /// optionally filtered to one recipient.
    fn list(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: Option<&str>,
    ) -> BoxFut<'_, Result<Vec<DeviceRecord>, StoreError>>;

    /// A single visible device; `None` when not found or not visible.
    fn get(
        &self,
        actor: UserId,
        project_id: &str,
        device_id: &str,
    ) -> BoxFut<'_, Result<Option<DeviceRecord>, StoreError>>;

    /// Soft-revokes a visible device (`status` → `revoked`; delivery
    /// ignores it, the row stays). Returns the fresh record; `None` when
    /// not found or not visible.
    fn revoke(
        &self,
        actor: UserId,
        project_id: &str,
        device_id: &str,
    ) -> BoxFut<'_, Result<Option<DeviceRecord>, StoreError>>;
}
