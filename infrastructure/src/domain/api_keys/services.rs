//! Project API-key use cases: issuance (hash-only storage), lifecycle,
//! per-key IP allowlists, and the verification seam.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use rand::RngCore;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::domain::api_keys::entities::{ApiKey, IpAllowEntry};
use crate::domain::audit::AuditService;
use crate::domain::audit::entities::{AuditAction, AuditEvent};
use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::ports::api_keys_store::{ApiKeysStore, KeyAuth, ip_allowed, normalize_cidr};
use crate::ports::auth_store::StoreError;

pub const KEY_PERMISSIONS: [&str; 2] = ["read", "write"];
pub const DEFAULT_KEY_PERMISSIONS: [&str; 1] = ["read"];

const MAX_NAME: usize = 100;
const MAX_LABEL: usize = 100;

fn hash_secret(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}

fn generate_secret() -> (String, String, String) {
    let prefix = "napi_".to_string();
    let mut bytes = [0u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    let raw = format!("{prefix}{}", hex::encode(bytes));
    let hash = hash_secret(&raw);
    (raw, prefix, hash)
}

fn normalize_permissions(raw: &[String]) -> Result<Vec<String>, AppError> {
    if raw.is_empty() {
        return Ok(DEFAULT_KEY_PERMISSIONS
            .iter()
            .map(|s| s.to_string())
            .collect());
    }
    let mut out = Vec::with_capacity(raw.len());
    for permission in raw {
        let normalized = permission.trim().to_lowercase();
        if !KEY_PERMISSIONS.contains(&normalized.as_str()) {
            return Err(AppError::Validation(format!(
                "invalid permission '{permission}' (expected one of: {})",
                KEY_PERMISSIONS.join(", ")
            )));
        }
        if !out.contains(&normalized) {
            out.push(normalized);
        }
    }
    Ok(out)
}

pub struct ApiKeyService {
    store: Arc<dyn ApiKeysStore>,
    audit: Arc<AuditService>,
}

impl ApiKeyService {
    pub fn new(store: Arc<dyn ApiKeysStore>, audit: Arc<AuditService>) -> Self {
        Self { store, audit }
    }

    /// Issues a key. Returns the record plus the raw secret, which is shown
    /// exactly once and never stored.
    pub async fn create(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        permissions: &[String],
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(ApiKey, String), AppError> {
        let name = name.trim();
        if name.is_empty() || name.len() > MAX_NAME {
            return Err(AppError::Validation(
                "name is required (100 characters max)".to_string(),
            ));
        }
        let permissions = normalize_permissions(permissions)?;
        if let Some(expires_at) = expires_at
            && expires_at <= Utc::now()
        {
            return Err(AppError::Validation(
                "expiry must be in the future".to_string(),
            ));
        }

        let (raw, prefix, hash) = generate_secret();
        let record = self
            .store
            .create_key(
                actor,
                project_id,
                name,
                &permissions,
                expires_at,
                &prefix,
                &hash,
            )
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict(
                    "a key with this name already exists for this project".into(),
                ),
                StoreError::Storage(m) if m.contains("not visible") => {
                    AppError::NotFound("project not found".to_string())
                }
                StoreError::Storage(m) => AppError::Storage(m),
            })?;

        let key = ApiKey::from(record);
        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::ApiKeyCreated,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("API key '{}' created", key.name),
                    Some(json!({ "key_id": key.id })),
                ),
            )
            .await;
        Ok((key, raw))
    }

    pub async fn list(&self, actor: UserId, project_id: &str) -> Result<Vec<ApiKey>, AppError> {
        Ok(self
            .store
            .list_keys(actor, project_id)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(ApiKey::from)
            .collect())
    }

    pub async fn get(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> Result<ApiKey, AppError> {
        self.store
            .get_key(actor, project_id, key_id)
            .await
            .map_err(map_store_error)?
            .map(ApiKey::from)
            .ok_or_else(|| AppError::NotFound("API key not found".to_string()))
    }

    pub async fn rename(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        name: &str,
    ) -> Result<ApiKey, AppError> {
        let name = name.trim();
        if name.is_empty() || name.len() > MAX_NAME {
            return Err(AppError::Validation(
                "name is required (100 characters max)".to_string(),
            ));
        }
        let updated = self
            .store
            .rename_key(actor, project_id, key_id, name)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => AppError::Conflict(
                    "a key with this name already exists for this project".into(),
                ),
                StoreError::Storage(m) => AppError::Storage(m),
            })?;
        if !updated {
            return Err(AppError::NotFound("API key not found".to_string()));
        }
        self.get(actor, project_id, key_id).await
    }

    pub async fn set_enabled(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        enabled: bool,
    ) -> Result<ApiKey, AppError> {
        let updated = self
            .store
            .set_key_enabled(actor, project_id, key_id, enabled)
            .await
            .map_err(map_store_error)?;
        if !updated {
            return Err(AppError::NotFound("API key not found".to_string()));
        }
        self.get(actor, project_id, key_id).await
    }

    pub async fn revoke(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> Result<ApiKey, AppError> {
        let key = self.get(actor, project_id, key_id).await?;
        if key.revoked_at.is_some() {
            return Ok(key);
        }
        let revoked = self
            .store
            .revoke_key(actor, project_id, key_id)
            .await
            .map_err(map_store_error)?;
        if !revoked {
            return Err(AppError::NotFound("API key not found".to_string()));
        }
        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::ApiKeyRevoked,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("API key '{}' revoked", key.name),
                    Some(json!({ "key_id": key.id })),
                ),
            )
            .await;
        self.get(actor, project_id, key_id).await
    }

    pub async fn list_ips(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> Result<Vec<IpAllowEntry>, AppError> {
        self.get(actor, project_id, key_id).await?;
        Ok(self
            .store
            .list_ips(actor, project_id, key_id)
            .await
            .map_err(map_store_error)?
            .into_iter()
            .map(IpAllowEntry::from)
            .collect())
    }

    pub async fn add_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        cidr: &str,
        label: &str,
    ) -> Result<IpAllowEntry, AppError> {
        let key = self.get(actor, project_id, key_id).await?;
        let cidr = normalize_cidr(cidr).map_err(AppError::Validation)?;
        let label = label.trim();
        if label.len() > MAX_LABEL {
            return Err(AppError::Validation(
                "label must be under 100 characters".to_string(),
            ));
        }
        let entry = self
            .store
            .add_ip(actor, project_id, key_id, &cidr, label)
            .await
            .map_err(|e| match e {
                StoreError::Conflict => {
                    AppError::Conflict("this address is already allowlisted on this key".into())
                }
                StoreError::Storage(m) => AppError::Storage(m),
            })?;
        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::ApiKeyIpAllowed,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("IP {} allowed on API key '{}'", entry.cidr, key.name),
                    Some(json!({ "key_id": key.id })),
                ),
            )
            .await;
        Ok(IpAllowEntry::from(entry))
    }

    pub async fn remove_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        entry_id: &str,
    ) -> Result<(), AppError> {
        let key = self.get(actor, project_id, key_id).await?;
        let removed = self
            .store
            .remove_ip(actor, project_id, key_id, entry_id)
            .await
            .map_err(map_store_error)?;
        if !removed {
            return Err(AppError::NotFound("allowlist entry not found".to_string()));
        }
        self.audit
            .record(
                Utc::now(),
                &AuditEvent::new(
                    AuditAction::ApiKeyIpRemoved,
                    Some(&actor.to_string()),
                    None,
                    Some(project_id),
                    format!("IP allowlist entry removed from API key '{}'", key.name),
                    Some(json!({ "key_id": key.id })),
                ),
            )
            .await;
        Ok(())
    }

    /// Verification seam for key-authenticated surfaces (the external API).
    /// Pure transport inputs: the presented secret and the already-parsed
    /// peer IP string. Touches `last_used_at` on success only.
    pub async fn verify(&self, raw_secret: &str, peer_ip: &str) -> Result<KeyAuth, AppError> {
        let Some(record) = self
            .store
            .find_key_by_hash(&hash_secret(raw_secret.trim()))
            .await
            .map_err(map_store_error)?
        else {
            return Ok(KeyAuth::Unknown);
        };
        if !record.is_usable(Utc::now()) {
            return Ok(KeyAuth::Disabled);
        }
        let entries = self
            .store
            .list_ips_for_key(&record.id)
            .await
            .map_err(map_store_error)?;
        // An empty allowlist means unrestricted; the first entry restricts.
        if !entries.is_empty() && !entries.iter().any(|e| ip_allowed(&e.cidr, peer_ip)) {
            return Ok(KeyAuth::IpDenied);
        }
        let _ = self.store.touch_key_last_used(&record.id).await;
        Ok(KeyAuth::Allowed {
            key_id: record.id,
            project_id: record.project_id,
        })
    }
}

fn map_store_error(err: StoreError) -> AppError {
    match err {
        StoreError::Conflict => AppError::Conflict("conflicting update".into()),
        StoreError::Storage(m) if m.contains("not visible") || m.contains("not found") => {
            AppError::NotFound("API key not found".to_string())
        }
        StoreError::Storage(m) => AppError::Storage(m),
    }
}
