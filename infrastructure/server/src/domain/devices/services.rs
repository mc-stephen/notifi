//! Device registration use cases — the dashboard side of the device
//! handshake (M5 of ARCHITECTURE; device-side self-registration rides the
//! external API when API-key auth lands).
//!
//! `register` is the one gate with real logic: the token must be
//! non-empty, and the destination + vendor pair must exist in the device
//! destination catalog handed to the service at composition — built by
//! the api assembler from the same Rust tables the Browse modal renders,
//! so no truth is duplicated here. Project visibility (owner or member)
//! is the store's job, same predicate family as `RecipientsStore`, and
//! register returns its invisible-project / unknown-recipient signals as
//! not-found problems. Proven by `server/tests/devices_api.rs` and the
//! `validate_registration` unit tests below.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;

use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::devices::entities::Device;
use crate::domain::error::AppError;
use crate::ports::auth_store::StoreError;
use crate::ports::devices_store::{DeviceRegistration, DevicesStore};

/// Destination → registered vendor ids: the device family's slice of the
/// served provider registry. The api assembler builds it from its own
/// device channel tables (which derive from `device_channel::providers`
/// anyway); the service validates against what it is given.
#[derive(Debug, Clone, Default)]
pub struct DeviceDestinationCatalog {
    channels: HashMap<String, Vec<String>>,
}

impl DeviceDestinationCatalog {
    /// One entry per device destination, value = the vendor ids the
    /// registry lists under it.
    pub fn new(channels: HashMap<String, Vec<String>>) -> Self {
        Self { channels }
    }

    fn has_destination(&self, destination: &str) -> bool {
        self.channels.contains_key(destination)
    }

    fn contains_vendor(&self, destination: &str, vendor: &str) -> bool {
        self.channels
            .get(destination)
            .is_some_and(|vendors| vendors.iter().any(|id| id == vendor))
    }
}

/// The registration gate: token present, destination known, vendor
/// registered under it. Pure so the unit tests need no store.
fn validate_registration(
    input: &DeviceRegistration,
    catalog: &DeviceDestinationCatalog,
) -> Result<(), AppError> {
    if input.token.trim().is_empty() {
        return Err(AppError::Validation("device token is required".to_string()));
    }
    if !catalog.has_destination(&input.destination) {
        return Err(AppError::Validation(format!(
            "unknown device destination '{}'",
            input.destination
        )));
    }
    if !catalog.contains_vendor(&input.destination, &input.vendor) {
        return Err(AppError::Validation(format!(
            "vendor '{}' is not registered under destination '{}'",
            input.vendor, input.destination
        )));
    }
    Ok(())
}

pub struct DeviceService {
    store: Arc<dyn DevicesStore>,
    audit: Arc<AuditService>,
    catalog: DeviceDestinationCatalog,
}

impl DeviceService {
    pub fn new(
        store: Arc<dyn DevicesStore>,
        audit: Arc<AuditService>,
        catalog: DeviceDestinationCatalog,
    ) -> Self {
        Self {
            store,
            audit,
            catalog,
        }
    }

    /// Registers a device, or refreshes the existing record when the same
    /// `(project, destination, token)` re-registers (store-level upsert).
    pub async fn register(
        &self,
        actor: UserId,
        project_id: &str,
        mut input: DeviceRegistration,
    ) -> Result<Device, AppError> {
        validate_registration(&input, &self.catalog)?;
        input.token = input.token.trim().to_string();

        let record = self
            .store
            .register(actor, project_id, input)
            .await
            .map_err(map_register_error)?;
        let device = Device::from(record);

        self.audit
            .record(
                chrono::Utc::now(),
                &AuditEvent::new(
                    AuditAction::DeviceRegistered,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!(
                        "device '{}' registered ({} / {})",
                        device.id, device.destination, device.vendor
                    ),
                    Some(json!({
                        "device_id": device.id.to_string(),
                        "destination": device.destination,
                        "vendor": device.vendor,
                    })),
                ),
            )
            .await;

        Ok(device)
    }

    /// Devices in the project, newest first, optionally filtered to a
    /// single recipient.
    pub async fn list(
        &self,
        actor: UserId,
        project_id: &str,
        recipient_id: Option<&str>,
    ) -> Result<Vec<Device>, AppError> {
        Ok(self
            .store
            .list(actor, project_id, recipient_id)
            .await
            .map_err(map_register_error)?
            .into_iter()
            .map(Device::from)
            .collect())
    }

    /// A single visible device; `None` when not found or not visible.
    pub async fn get(
        &self,
        actor: UserId,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<Device>, AppError> {
        Ok(self
            .store
            .get(actor, project_id, device_id)
            .await
            .map_err(map_register_error)?
            .map(Device::from))
    }

    /// Soft-revokes a visible device (status flip; the row stays).
    /// `None` when not found or not visible.
    pub async fn revoke(
        &self,
        actor: UserId,
        project_id: &str,
        device_id: &str,
    ) -> Result<Option<Device>, AppError> {
        let revoked = self
            .store
            .revoke(actor, project_id, device_id)
            .await
            .map_err(map_register_error)?
            .map(Device::from);

        if let Some(device) = &revoked {
            self.audit
                .record(
                    chrono::Utc::now(),
                    &AuditEvent::new(
                        AuditAction::DeviceRevoked,
                        Some(&actor.to_string()),
                        None,
                        Some(project_id),
                        format!("device '{}' revoked ({})", device.id, device.destination),
                        Some(json!({
                            "device_id": device.id.to_string(),
                            "destination": device.destination,
                            "vendor": device.vendor,
                        })),
                    ),
                )
                .await;
        }

        Ok(revoked)
    }
}

/// register has no `None` channel, so its not-found signals arrive as
/// store `Storage` messages (exact texts from the port contract) and are
/// promoted to proper 404s here; everything else maps normally.
fn map_register_error(err: StoreError) -> AppError {
    match err {
        StoreError::Storage(m) if m.contains("project not found") => {
            AppError::NotFound("project not found or not visible to you".to_string())
        }
        StoreError::Storage(m) if m.contains("recipient not found") => {
            AppError::NotFound("recipient not found".to_string())
        }
        StoreError::Conflict => AppError::Conflict("device registration conflict".to_string()),
        StoreError::Storage(m) => AppError::Storage(m),
    }
}

#[cfg(test)]
mod device_validation_tests {
    use super::*;

    fn catalog() -> DeviceDestinationCatalog {
        let mut channels: HashMap<String, Vec<String>> = HashMap::new();
        channels.insert("android".to_string(), vec!["fcm".to_string()]);
        channels.insert("apple".to_string(), vec!["apns".to_string()]);
        DeviceDestinationCatalog::new(channels)
    }

    fn registration(destination: &str, vendor: &str, token: &str) -> DeviceRegistration {
        DeviceRegistration {
            destination: destination.to_string(),
            vendor: vendor.to_string(),
            token: token.to_string(),
            label: None,
            recipient_id: None,
        }
    }

    #[test]
    fn register_accepts_a_registered_destination_vendor_pair() {
        assert!(
            validate_registration(&registration("android", "fcm", "tok-1"), &catalog()).is_ok()
        );
    }

    #[test]
    fn register_rejects_an_unknown_destination() {
        let err = validate_registration(&registration("email", "fcm", "tok-1"), &catalog())
            .expect_err("email is not a device destination");
        assert!(
            matches!(err, AppError::Validation(ref m) if m.contains("unknown device destination"))
        );
    }

    #[test]
    fn register_rejects_a_vendor_outside_its_destination() {
        let err =
            validate_registration(&registration("apple", "fcm", "tok-1"), &catalog()).expect_err(
                "fcm is not registered under apple — the registry pins destinations to their vendors",
            );
        assert!(matches!(err, AppError::Validation(ref m) if m.contains("not registered under")));
    }

    #[test]
    fn register_rejects_an_empty_or_whitespace_token() {
        for token in ["", "   "] {
            let err = validate_registration(&registration("android", "fcm", token), &catalog())
                .expect_err("an empty token can never deliver");
            assert!(matches!(err, AppError::Validation(ref m) if m.contains("token is required")));
        }
    }

    #[test]
    fn validation_does_not_leak_destination_details_on_empty_token() {
        // The token gate runs first: it never name-validates destination/
        // vendor pairs (mirror of the no-enumeration contract elsewhere).
        let err = validate_registration(&registration("email", "fcm", "  "), &catalog())
            .expect_err("empty token");
        assert!(matches!(err, AppError::Validation(ref m) if m.contains("token is required")));
    }
}
