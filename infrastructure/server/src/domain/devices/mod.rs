//! Device domain — per-device destination records (M5 of ARCHITECTURE).
//!
//! A `project_devices` row is the project's way of knowing where a
//! delivery can land: one destination + vendor + vendor token each.
//! Registration is dashboard-owned today (the owner registers devices/
//! apps by hand); device-side self-registration rides the external
//! product API when API-key auth lands (M6 era).
//!
//! Delivery ignores these rows until the device adapters land
//! (`infra::delivery_adapters` — M5/M6 era); the records exist so those
//! lookups have something to find. Proven by `server/tests/devices_api.rs`
//! (register, refresh, list filters, revoke, role gates) and the
//! validation unit tests inside `services.rs`.

pub mod entities;
pub mod services;

pub use entities::Device;
pub use entities::DeviceId;
pub use services::DeviceDestinationCatalog;
pub use services::DeviceService;
