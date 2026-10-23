//! Tenant project configuration storage — the portable home for channel
//! config JSONs and template assets.
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
//! projects/{slug}/config/{channel_name}/{file_name}
//! projects/{slug}/templates/{template_name}/{file_name}
//! ```
//!
//! `{slug}` is the project's stored slug (`kernel::config::project_slug`)
//! — the namespace springs into existence with the project (creation
//! materializes [`crate::kernel::config::DefaultProjectLayout`]), joins
//! every export zip, and is wiped folder-first by the worker's purge.
//!
//! Nothing here knows about JSON parsing or templates — drivers move raw
//! bytes only. Layers above (channel loaders, renderers) own meaning.

use async_trait::async_trait;

/// Defensive key validation: keys MUST be relative, non-empty, and
/// root-convention-shaped strings. A leading `/` is a classic escape on
/// the file driver (`root.join("/abs")` replaces the root outright) and
/// means a different thing on the object driver (normalized away) — both
/// count as an error rather than a silent surprise.
pub(crate) fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.starts_with('/') {
        return Err(format!(
            "invalid config key (must be relative, non-empty): '{key}'"
        ));
    }
    Ok(())
}

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

    /// Removes `key`. Idempotent like S3 semantics: a missing key is a
    /// success, so callers can wipe without checking first. Only files
    /// are removed — no prefix deletion; delete what you listed.
    async fn delete_raw(&self, key: &str) -> Result<(), String>;

    /// Lists the keys directly under `prefix` in ascending order. The
    /// prefix itself is relative, like every key here.
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, String>;

    /// Makes the store's top-level folders exist — so an empty bucket or
    /// a fresh root already shows the layout the runtime will use.
    ///
    /// Drivers decide which roots count: the file driver creates real
    /// directories; the object driver puts a tiny marker object inside
    /// each top-level prefix (object stores have no folder entities — a
    /// prefix becomes visible once anything lives under it). Idempotent
    /// and cheap: every boot calls this and re-`--reset-config` marks the
    /// folders again after wiping.
    async fn ensure_root_folders(&self) -> Result<(), String>;

    /// Materializes a fresh project's default files into this store — the
    /// [`crate::kernel::config::DefaultProjectLayout`] set (per-channel
    /// config + starter templates). Wholesale, not a merge: every file in
    /// the layout is written with its default contents. Project creation
    /// calls this once per new project, keyed by its stored slug.
    async fn materialize_project(&self, slug: &str) -> Result<(), String> {
        for file in crate::kernel::config::DefaultProjectLayout::files(slug) {
            self.put_raw(&file.key, file.contents).await?;
        }
        Ok(())
    }
}
