//! Picks and builds the active [`ConfigStore`], and the dev-facing reset.
//!
//! One home for the driver decision so composition (`lib.rs`) and ops
//! (`--reset`) can never disagree about which
//! storage backs the config. Delete idempotency (missing = success) lets
//! a wiped-already store reset to a clean no-op.
//!
//! The decision is now absolute: the object store is a boot requirement,
//! not a fallback ladder. All four `NOTIFI_R2_*` values must be set or
//! there is nothing to run — boot aborts naming what is missing.

use std::sync::Arc;

use crate::infra::R2ConfigStore;
use crate::infra::config::{AppConfig, R2Config};
use crate::ports::config_store::ConfigStore;

/// The env names the object store needs, in reporting order.
const R2_ENV_NAMES: [&str; 4] = [
    "NOTIFI_R2_ACCOUNT_ID",
    "NOTIFI_R2_ACCESS_KEY_ID",
    "NOTIFI_R2_SECRET_ACCESS_KEY",
    "NOTIFI_R2_BUCKET",
];

/// The R2 values, as optional strings, in the same order as [`R2_ENV_NAMES`].
fn r2_parts(r2: &R2Config) -> [Option<&String>; 4] {
    [
        r2.account_id.as_ref(),
        r2.access_key_id.as_ref(),
        r2.secret_access_key.as_ref(),
        r2.bucket.as_ref(),
    ]
}

/// Builds the store the runtime addresses projects through. The object
/// store (R2 cloudflare family, S3-compatible) is mandatory: anything
/// missing returns an error naming the unset variable — callers refuse
/// to boot on it.
pub fn build(config: &AppConfig) -> Result<Arc<dyn ConfigStore + Send + Sync>, String> {
    let missing: Vec<&str> = R2_ENV_NAMES
        .iter()
        .zip(r2_parts(&config.r2))
        .filter_map(|(name, part)| part.is_none().then_some(*name))
        .collect();
    if !missing.is_empty() {
        return Err(format!(
            "object store not configured — the runtime boots R2-or-nothing; set all of {} (missing: {})",
            R2_ENV_NAMES.join(", "),
            missing.join(", "),
        ));
    }
    tracing::info!(
        bucket = %config.r2.bucket.as_ref().expect("all values checked above"),
        "config store: R2 object storage"
    );
    Ok(Arc::new(R2ConfigStore::from_config(&config.r2)?))
}

/// Builds the store AND makes its top-level folders exist (`projects/`,
/// `backup/`, `users/`) — every boot flows through here, so the storage
/// layout is visible in a fresh bucket before the first project arrives.
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
    fn missing_r2_values_refuse_to_build_and_name_themselves() {
        let config = AppConfig::default();
        let err = match build(&config) {
            Err(err) => err,
            Ok(_) => panic!("nothing configured must refuse"),
        };
        assert!(err.starts_with("object store not configured"));
        for name in R2_ENV_NAMES {
            assert!(err.contains(name), "error must name {name}");
        }
    }

    #[test]
    fn partially_configured_r2_refuses_too() {
        let config = AppConfig {
            r2: crate::infra::config::R2Config {
                account_id: Some("a".to_string()),
                access_key_id: Some("k".to_string()),
                secret_access_key: None,
                bucket: Some("notifi".to_string()),
            },
            ..AppConfig::default()
        };
        let err = match build(&config) {
            Err(err) => err,
            Ok(_) => panic!("a partial set must refuse"),
        };
        assert!(err.contains("NOTIFI_R2_SECRET_ACCESS_KEY"), "got: {err}");
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
