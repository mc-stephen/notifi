//! API-key contracts: issuance records, per-key IP allowlists, verification.
//!
//! Framework-free by rule: no axum/sqlx/reqwest types may appear here.
//! IP matching uses only `std::net` parsing plus the `ipnet` crate.

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};

use crate::domain::auth::entities::UserId;
use crate::ports::auth_store::StoreError;

pub type BoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// An issued key as stored (never the raw secret — only its hash).
#[derive(Debug, Clone)]
pub struct ApiKeyRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub prefix: String,
    pub key_hash: String,
    pub environment: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// One allowlist entry on a key.
#[derive(Debug, Clone)]
pub struct IpAllowEntry {
    pub id: String,
    pub key_id: String,
    pub cidr: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
}

impl ApiKeyRecord {
    /// Whether the key is usable right now (before IP checks).
    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
        self.enabled && self.revoked_at.is_none() && self.expires_at.is_none_or(|exp| exp > now)
    }
}

/// Outcome of verifying a presented secret against a peer IP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAuth {
    /// Allowed; carries the key id + project for the caller.
    Allowed { key_id: String, project_id: String },
    /// Unknown secret.
    Unknown,
    /// Known but disabled, revoked, or expired.
    Disabled,
    /// Peer IP is outside every allowlist entry.
    IpDenied,
}

/// Normalizes user input (`203.0.113.7`, `203.0.113.0/24`, `::1`) into a
/// canonical CIDR string. Bare addresses become host routes; anything
/// unparsable is rejected with a human message.
pub fn normalize_cidr(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("address is required (IP or CIDR, e.g. 203.0.113.7 or 10.0.0.0/8)".to_string());
    }
    if let Ok(net) = trimmed.parse::<ipnet::IpNet>() {
        return Ok(net.to_string());
    }
    // Bare IP without a mask → host route.
    if let Ok(addr) = trimmed.parse::<std::net::IpAddr>() {
        let net: ipnet::IpNet = addr.into();
        return Ok(net.to_string());
    }
    Err(format!(
        "'{trimmed}' is not a valid IP address or CIDR range"
    ))
}

/// Whether `peer` (already validated by the transport) falls inside `cidr`.
/// Unparsable inputs deny (fail-closed); callers normalize on write, so
/// this path only trips on hand-edited rows.
pub fn ip_allowed(cidr: &str, peer: &str) -> bool {
    let (Ok(net), Ok(addr)) = (
        cidr.parse::<ipnet::IpNet>(),
        peer.parse::<std::net::IpAddr>(),
    ) else {
        return false;
    };
    net.contains(&addr)
}

pub trait ApiKeysStore: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn create_key(
        &self,
        actor: UserId,
        project_id: &str,
        name: &str,
        environment: &str,
        permissions: &[String],
        expires_at: Option<DateTime<Utc>>,
        prefix: &str,
        key_hash: &str,
    ) -> BoxFut<'_, Result<ApiKeyRecord, StoreError>>;

    fn list_keys(
        &self,
        actor: UserId,
        project_id: &str,
    ) -> BoxFut<'_, Result<Vec<ApiKeyRecord>, StoreError>>;

    fn get_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<Option<ApiKeyRecord>, StoreError>>;

    fn rename_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        name: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    fn set_key_enabled(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        enabled: bool,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    fn revoke_key(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;

    /// Looks a key up by secret hash (verification path — no actor scoping;
    /// possession of the secret IS the credential).
    fn find_key_by_hash(
        &self,
        key_hash: &str,
    ) -> BoxFut<'_, Result<Option<ApiKeyRecord>, StoreError>>;

    fn touch_key_last_used(&self, key_id: &str) -> BoxFut<'_, Result<(), StoreError>>;

    fn list_ips(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
    ) -> BoxFut<'_, Result<Vec<IpAllowEntry>, StoreError>>;

    /// Unscoped IP listing for the verification path (possession of the
    /// secret is the credential — there is no actor to scope by).
    fn list_ips_for_key(&self, key_id: &str) -> BoxFut<'_, Result<Vec<IpAllowEntry>, StoreError>>;

    fn add_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        cidr: &str,
        label: &str,
    ) -> BoxFut<'_, Result<IpAllowEntry, StoreError>>;

    fn remove_ip(
        &self,
        actor: UserId,
        project_id: &str,
        key_id: &str,
        entry_id: &str,
    ) -> BoxFut<'_, Result<bool, StoreError>>;
}
