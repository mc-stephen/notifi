//! Tenant brand configuration storage — the portable home for channel
//! config JSONs and brand template assets.
//!
//! Keys are relative paths under the convention root; every driver treats
//! the convention root as its base, so the same key reads back identically
//! from disk or a bucket:
//!
//! ```text
//! file driver  → {NOTIFI_CONFIG_ROOT}/{key}
//! object store → {bucket}/{key}          (the bucket IS the root)
//! ```
//!
//! Canonical keys come from [`crate::kernel::config`]:
//!
//! ```text
//! brands/{brand}/config/{channel_name}/{file_name}
//! brands/{brand}/templates/{template_name}/{file_name}
//! ```
//!
//! Nothing here knows about JSON parsing or templates — drivers move raw
//! bytes only. Layers above (channel loaders, renderers) own meaning.

use async_trait::async_trait;

/// Trait implemented by infra drivers (filesystem today, object stores
/// like R2 for deployment). [`crate::infra::config::AppConfig`] decides
/// which one is active at boot.
#[async_trait]
pub trait ConfigStore: Send + Sync {
    /// Reads raw bytes at `key`. Errors when the key is absent — callers
    /// decide what a missing file means for their flow (e.g. "no config
    /// installed for this tenant" vs. a broken config).
    async fn load_raw(&self, key: &str) -> Result<Vec<u8>, String>;

    /// Writes raw bytes at `key`, creating any parent structure needed.
    async fn put_raw(&self, key: &str, bytes: Vec<u8>) -> Result<(), String>;

    /// Lists the keys directly under `prefix` in ascending order. The
    /// prefix itself is relative, like every key here.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, String>;
}
