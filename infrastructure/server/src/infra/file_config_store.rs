//! Filesystem-backed config store — the local-dev baseline driver.
//!
//! Reads/writes/lists under `kernel`'s convention root directly, i.e.
//! exactly what [`crate::kernel::config::ConfigResolver`] did when it was
//! the only path. `NOTIFI_CONFIG_ROOT` selects the root (defaults to
//! `configs`); `infrastructure/.cargo/config.toml` points dev bootstraps
//! at `server/assets`.

use std::path::PathBuf;

use crate::ports::config_store::ConfigStore;

/// Root-relative semantics: key `brands/_default/config/email/config.json`
/// maps to `{root}/brands/_default/config/email/config.json`.
pub struct FileConfigStore {
    root: PathBuf,
}

impl FileConfigStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The same root the kernel path helpers resolve, so file keys and
    /// config paths stay interchangeable.
    pub fn from_root_env() -> Self {
        Self::new(crate::kernel::config::ConfigResolver::root())
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.root.join(key)
    }
}

#[async_trait::async_trait]
impl ConfigStore for FileConfigStore {
    async fn load_raw(&self, key: &str) -> Result<Vec<u8>, String> {
        let path = self.path_for(key);
        if !path.is_file() {
            return Err(format!(
                "config key not found: {key} (root {})",
                self.root.display()
            ));
        }
        std::fs::read(&path).map_err(|e| format!("config read failed for {key}: {e}"))
    }

    async fn put_raw(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("config dir create failed for {key}: {e}"))?;
        }
        std::fs::write(&path, &bytes).map_err(|e| format!("config write failed for {key}: {e}"))
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, String> {
        let base = self.path_for(prefix);
        if !base.is_dir() {
            return Ok(Vec::new());
        }

        let mut keys = Vec::new();
        let mut stack = vec![base.clone()];
        while let Some(dir) = stack.pop() {
            let entries = std::fs::read_dir(&dir)
                .map_err(|e| format!("config list failed for {prefix}: {e}"))?;
            for entry in entries {
                let entry = entry.map_err(|e| format!("config list failed for {prefix}: {e}"))?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    keys.push(
                        path.strip_prefix(&self.root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string(),
                    );
                }
            }
        }
        keys.sort();
        Ok(keys)
    }
}

#[cfg(test)]
mod file_config_store_tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(name);
        std::fs::remove_dir_all(&root).ok();
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[tokio::test]
    async fn loads_missing_key_as_error() {
        let store = FileConfigStore::new(temp_root("notifi-fcs-missing"));
        let err = store
            .load_raw("brands/_default/config/email/config.json")
            .await
            .expect_err("missing key is an error");
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn put_then_load_roundtrip_creates_parents() {
        let root = temp_root("notifi-fcs-roundtrip");
        let store = FileConfigStore::new(&root);
        store
            .put_raw(
                "brands/acme/config/email/config.json",
                br#"{"host":"smtp.x"}"#.to_vec(),
            )
            .await
            .expect("put");

        let raw = store
            .load_raw("brands/acme/config/email/config.json")
            .await
            .expect("load");
        assert_eq!(raw, br#"{"host":"smtp.x"}"#.to_vec());
        assert!(root.join("brands/acme/config/email/config.json").is_file());
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn lists_keys_sorted_under_prefix_only() {
        let root = temp_root("notifi-fcs-list");
        let store = FileConfigStore::new(&root);
        store
            .put_raw("brands/acme/config/email/config.json", b"{}".to_vec())
            .await
            .unwrap();
        store
            .put_raw(
                "brands/acme/templates/welcome/email.html",
                b"<p>hi</p>".to_vec(),
            )
            .await
            .unwrap();
        store
            .put_raw("brands/_default/config/sms/other.json", b"{}".to_vec())
            .await
            .unwrap();

        let keys = store.list_keys("brands/acme").await.expect("list");
        assert_eq!(
            keys,
            vec![
                "brands/acme/config/email/config.json".to_string(),
                "brands/acme/templates/welcome/email.html".to_string(),
            ]
        );
        std::fs::remove_dir_all(&root).ok();
    }
}
