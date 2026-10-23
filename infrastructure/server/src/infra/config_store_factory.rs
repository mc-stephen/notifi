//! Picks and builds the active [`ConfigStore`], and the dev-facing reset.
//!
//! One home for the driver decision so composition (`lib.rs`) and ops
//! (`--reset-db` / `--reset-config`) can never disagree about which
//! storage backs the config. Delete idempotency (missing = success) lets
//! a wiped-already store reset to a clean no-op.

use std::sync::Arc;

use crate::infra::config::AppConfig;
use crate::infra::{FileConfigStore, R2ConfigStore};
use crate::ports::config_store::ConfigStore;

/// Builds the store the runtime addresses projects through:
/// * R2 object store when all four `NOTIFI_R2_*` values are set — the
///   bucket is the convention root;
/// * the file driver otherwise (`NOTIFI_CONFIG_ROOT`, dev points it at
///   `server/assets` via `.cargo/config.toml`).
pub fn build(config: &AppConfig) -> Result<Arc<dyn ConfigStore + Send + Sync>, String> {
    if config.r2.configured() {
        tracing::info!(
            bucket = %config.r2.bucket.clone().expect("configured() checked"),
            "config store: R2 object storage"
        );
        Ok(Arc::new(R2ConfigStore::from_config(&config.r2)?))
    } else {
        tracing::info!("config store: filesystem (NOTIFI_CONFIG_ROOT)");
        Ok(Arc::new(FileConfigStore::from_root_env()))
    }
}

/// Builds the store AND makes its top-level folders exist (`projects/`)
/// — every boot flows through here, so the storage layout is visible in
/// a fresh bucket or root before the first project arrives.
pub async fn build_ensured(
    config: &AppConfig,
) -> Result<Arc<dyn ConfigStore + Send + Sync>, String> {
    let store = build(config)?;
    store
        .ensure_root_folders()
        .await
        .map_err(|e| format!("root folders could not be ensured: {e}"))?;
    Ok(store)
}

/// Deletes every project's config from the active store — the dev twin of
/// a database reset. Defaults are code (`kernel::config
/// ::DefaultProjectLayout`), so nothing is preserved and nothing needs
/// re-seeding: the store ends without any `projects/{slug}/` keys, and
/// each future project materializes its own tree on creation. Delete
/// idempotency (missing = success) lets a wiped-already store reset to a
/// clean no-op. The top-level folder markers are rebuilt afterwards, so
/// an empty bucket still shows its layout.
pub async fn reset(config: &AppConfig) -> Result<usize, String> {
    let store = build(config)?;
    // The wipe takes the whole top-level prefix, the `.keep` marker
    // included — `ensure_root_folders` below puts it straight back.
    let keys = store.list_keys("projects/").await?;
    for key in &keys {
        store.delete_raw(key).await?;
    }
    store
        .ensure_root_folders()
        .await
        .map_err(|e| format!("root folders could not be re-ensured after reset: {e}"))?;
    Ok(keys.len())
}

#[cfg(test)]
mod factory_tests {
    use super::*;

    #[test]
    fn file_store_is_the_default_when_r2_is_unset() {
        let config = AppConfig::default();
        // Building never touches the filesystem for the file driver —
        // the root resolves from env.
        let _store = build(&config).expect("file store builds");
    }

    #[test]
    fn r2_store_builds_from_a_configured_env() {
        let config = AppConfig {
            r2: crate::infra::config::R2Config {
                account_id: Some("a".to_string()),
                access_key_id: Some("k".to_string()),
                secret_access_key: Some("s".to_string()),
                bucket: Some("notifi".to_string()),
            },
            ..AppConfig::default()
        };
        let _store = build(&config).expect("r2 store builds offline");
    }
}
