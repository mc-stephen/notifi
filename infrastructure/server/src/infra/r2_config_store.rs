//! Cloudflare R2 config store — the object-store driver for tenant brand
//! config, S3-compatible and backed by Cloudflare's zero-egress R2.
//!
//! The bucket IS the convention root: key `brands/_default/config/...`
//! maps 1:1 to bucket object `brands/_default/config/...`, so nothing
//! special-cases the folder scheme and the file driver's keys work as-is.
//!
//! Endpoint derives from the account id (`R2Config::endpoint`) rather than
//! accepting a pasted URL — hand-copied R2 endpoints often end in a glued
//! bucket path that heisenbug-burrows into every key.
//!
//! Reads go through the shared `object_store` client (SigV4 signing,
//! retries, streaming) — nothing Notifi-specific is added on the wire.

use std::sync::Arc;

use futures::StreamExt;
use object_store::aws::AmazonS3Builder;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};

use crate::infra::config::R2Config;
use crate::ports::config_store::ConfigStore;

pub struct R2ConfigStore {
    client: Arc<dyn ObjectStore>,
    /// Bucket name surfaced only through error messages.
    bucket: String,
}

impl R2ConfigStore {
    /// Builds the client from a fully-configured [`R2Config`]. Fails with
    /// an explicit message when the config is incomplete — callers should
    /// gate on [`R2Config::configured`] before constructing.
    pub fn from_config(config: &R2Config) -> Result<Self, String> {
        if !config.configured() {
            return Err(
                "r2 config store needs NOTIFI_R2_ACCOUNT_ID, NOTIFI_R2_ACCESS_KEY_ID, \
                 NOTIFI_R2_SECRET_ACCESS_KEY, and NOTIFI_R2_BUCKET"
                    .to_string(),
            );
        }

        let secret = config
            .secret_access_key
            .as_ref()
            .expect("configured() checked");
        let key_id = config.access_key_id.as_ref().expect("configured() checked");
        let bucket = config.bucket.clone().expect("configured() checked");

        let built = AmazonS3Builder::new()
            .with_endpoint(config.endpoint())
            .with_bucket_name(bucket.clone())
            .with_access_key_id(key_id.clone())
            .with_secret_access_key(secret.clone())
            .with_region("auto") // R2 has exactly one value; omitting fails SigV4
            .build()
            .map_err(|e| format!("r2 client build failed: {e}"))?;
        let client: Arc<dyn ObjectStore> = Arc::new(built);

        Ok(Self { client, bucket })
    }

    fn object_path(&self, key: &str) -> Result<ObjectPath, String> {
        crate::ports::config_store::validate_key(key)?;
        ObjectPath::parse(key).map_err(|e| format!("invalid config key '{key}': {e}"))
    }
}

#[async_trait::async_trait]
impl ConfigStore for R2ConfigStore {
    async fn load_raw(&self, key: &str) -> Result<Vec<u8>, String> {
        let path = self.object_path(key)?;
        match self.client.get(&path).await {
            Ok(result) => {
                let bytes = result
                    .bytes()
                    .await
                    .map_err(|e| format!("config read failed for {key}: {e}"))?;
                Ok(bytes.to_vec())
            }
            Err(e) if e.to_string().to_lowercase().contains("not found") => {
                Err(format!("config key not found: {key}"))
            }
            Err(e) => Err(format!(
                "config read failed for {key} (bucket {}): {e}",
                self.bucket
            )),
        }
    }

    async fn put_raw(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
        let path = self.object_path(key)?;
        self.client
            .put(&path, PutPayload::from(bytes))
            .await
            .map_err(|e| {
                format!(
                    "config write failed for {key} (bucket {}): {e}",
                    self.bucket
                )
            })?;
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, String> {
        let base = self.object_path(prefix)?;
        let stream = self.client.list(Some(&base));
        let mut keys = Vec::new();
        let mut buffered = stream;
        while let Some(meta) = buffered.next().await {
            keys.push(
                meta.map_err(|e| format!("config list failed for {prefix}: {e}"))?
                    .location
                    .as_ref()
                    .to_string(),
            );
        }
        keys.sort();
        Ok(keys)
    }

    async fn delete_raw(&self, key: &str) -> Result<(), String> {
        let path = self.object_path(key)?;
        self.client.delete(&path).await.map_err(|e| {
            format!(
                "config delete failed for {key} (bucket {}): {e}",
                self.bucket
            )
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod r2_config_store_tests {
    use super::*;
    use crate::infra::config::R2Config;

    fn config() -> R2Config {
        R2Config {
            account_id: Some("159ffac81ea78b878c70fe1568e39515".to_string()),
            access_key_id: Some("k".to_string()),
            secret_access_key: Some("s".to_string()),
            bucket: Some("notifi".to_string()),
        }
    }

    #[test]
    fn endpoint_derives_from_account_id() {
        assert_eq!(
            config().endpoint(),
            "https://159ffac81ea78b878c70fe1568e39515.r2.cloudflarestorage.com"
        );
    }

    #[test]
    fn incomplete_config_is_an_explicit_error() {
        let mut trimmed = config();
        trimmed.bucket = None;
        assert!(!trimmed.configured());
        match R2ConfigStore::from_config(&trimmed) {
            Ok(_) => panic!("incomplete config must not build a client"),
            Err(e) => assert!(e.contains("NOTIFI_R2_BUCKET"), "got: {e}"),
        }
    }

    #[test]
    fn invalid_keys_rejected_before_any_wire_call() {
        let store = R2ConfigStore::from_config(&config()).expect("builds");
        let result = futures::executor::block_on(store.load_raw("/brands/absolute/path.json"));
        match result {
            Ok(_) => panic!("absolute keys must not pass"),
            Err(e) => assert!(e.contains("invalid config key"), "got: {e}"),
        }
    }
}
