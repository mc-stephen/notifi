//! The "other" channel — vendors that belong to no single platform
//! family. OneSignal speaks both phone families at once (its
//! subscription ids are one platform-agnostic namespace), so its one
//! saved config arms android and apple destinations together; the
//! webhook relay posts raw payloads to services Notifi does not
//! natively integrate. No controller lives here: each flavor wraps the
//! one its owning family already ships (the device family's OneSignal,
//! the webhook family's relay / function flavors), and a provider that
//! outgrows the wrapper earns a real channel of its own.

pub mod config;

pub use config::OtherChannelConfig;
