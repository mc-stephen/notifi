//! The "other" channel — vendors that belong to no single platform
//! family. OneSignal speaks both phone families at once (its
//! subscription ids are one platform-agnostic namespace), so its one
//! saved config arms android and apple destinations together; the
//! The webhook relay posts raw payloads to services Notifi does not
//! natively integrate; the gateway is the hosted pull-route (your app
//! polls a Notifi-hosted queue). No controller lives here: the onesignal
//! and pushy flavors wrap the ones the device family already ships
//! (both speak phone families — and web, for pushy — from one key), the
//! relay wraps the webhook family's relay / function flavors, and the
//! gateway owns its only home (`ports::gateway` queue +
//! `src/api/routes/gateway.rs`). A provider that outgrows the wrapper
//! earns a real channel of its own.

pub mod config;
pub mod providers;

pub use config::OtherChannelConfig;
