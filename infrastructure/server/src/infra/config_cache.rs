//! Read-through cache in front of the [`ConfigStore`] port.
//!
//! Config bytes are tiny and read-happy (per-delivery template loads),
//! so every reader rides this layer: a Redis GET answers a warm key
//! directly — the bucket behind it sees nothing. Writes flip the order:
//! store first, then invalidate, so the next read repopulates from the
//! source of truth (no cache-vs-store skew to reason about).
//!
//! Listing stays uncached on purpose: it powers admin/verification flows
//! where freshness beats request volume.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use redis::aio::ConnectionManager;

use crate::ports::config_store::ConfigStore;

/// Cache TTL for config keys. Tenant configs change rarely (admin flow),
/// so a generous TTL keeps steady-state bucket requests at zero; the
/// invalidate-on-write path makes edits visible immediately.
pub const CONFIG_CACHE_TTL: Duration = Duration::from_secs(600);

/// This feature's page label under [`crate::infra::redis::APP_KEY_PREFIX`]
/// — keys compose to `notifi:cfg:{store key}` at every use site, so the
/// brand label stays one edit one place.
const CACHE_PREFIX: &str = "cfg:";

/// Pluggable cache surface — the seam only `infra` knows about.
/// Implementations: Redis (shared across server + worker processes) and
/// an in-memory map for tests.
#[async_trait]
pub trait CacheBackend: Send + Sync {
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    async fn set(&self, key: &str, bytes: Vec<u8>);
    async fn invalidate(&self, key: &str);
    /// Drops every key this backend holds — the reset half for the cache:
    /// nothing survives the wipe, TTL'd or forever.
    async fn clear(&self);
}

/// Emits the fully-labeled key string for both directions.
macro_rules! cache_key {
    ($key:expr) => {
        format!(
            "{}{CACHE_PREFIX}{}",
            crate::infra::redis::APP_KEY_PREFIX,
            $key
        )
    };
}

/// Redis-backed cache behind the same `NOTIFI_REDIS_URL` the rest of the
/// platform already uses (`infra::redis::connect`). `new` carries the
/// config cache's TTL; `without_ttl` flips to forever keys (no expiry —
/// the system-template warm layer, where a restart is the refresh path).
pub struct RedisCacheBackend {
    conn: ConnectionManager,
    ttl: Option<Duration>,
}

impl RedisCacheBackend {
    pub fn new(conn: ConnectionManager) -> Self {
        Self {
            conn,
            ttl: Some(CONFIG_CACHE_TTL),
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Forever keys: `SET` without expiry. Refreshing is explicit —
    /// a warm pass overwrites, nothing expires behind anyone's back.
    pub fn without_ttl(mut self) -> Self {
        self.ttl = None;
        self
    }
}

#[async_trait]
impl CacheBackend for RedisCacheBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut connection = self.conn.clone();
        redis::cmd("GET")
            .arg(cache_key!(key))
            .query_async(&mut connection)
            .await
            .ok()
            .flatten()
    }

    async fn set(&self, key: &str, bytes: Vec<u8>) {
        let mut connection = self.conn.clone();
        let mut cmd = redis::cmd("SET");
        cmd.arg(cache_key!(key)).arg(bytes);
        if let Some(ttl) = self.ttl {
            cmd.arg("EX").arg(ttl.as_secs().max(1));
        }
        let _: Result<(), _> = cmd.query_async(&mut connection).await;
    }

    async fn invalidate(&self, key: &str) {
        let mut connection = self.conn.clone();
        let _: Result<(), _> = redis::cmd("DEL")
            .arg(cache_key!(key))
            .query_async(&mut connection)
            .await;
    }

    /// SCAN walks the keyspace in cursor batches; DEL frees each batch
    /// found. Only this cache's fully-labeled namespace matches, so other
    /// tenants of the shared Redis keep their keys. Best-effort like
    /// every cache write: a mid-wipe failure loses nothing durable
    /// (entries re-read through the store, TTL'd ones self-heal within
    /// ten minutes).
    async fn clear(&self) {
        let mut connection = self.conn.clone();
        let mut cursor: u64 = 0;
        loop {
            let (next, keys): (u64, Vec<String>) = match redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(format!(
                    "{}{CACHE_PREFIX}*",
                    crate::infra::redis::APP_KEY_PREFIX
                ))
                .arg("COUNT")
                .arg(1000)
                .query_async(&mut connection)
                .await
            {
                Ok(pair) => pair,
                Err(_) => return,
            };
            if !keys.is_empty() {
                let _: Result<(), _> = redis::cmd("DEL")
                    .arg(keys)
                    .query_async(&mut connection)
                    .await;
            }
            cursor = next;
            if cursor == 0 {
                break;
            }
        }
    }
}

/// In-memory cache for tests (and a reference for what Redis does).
pub struct MemoryCacheBackend {
    entries: Mutex<HashMap<String, (Vec<u8>, Instant)>>,
    ttl: Duration,
}

impl MemoryCacheBackend {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            ttl,
        }
    }
}

#[async_trait]
impl CacheBackend for MemoryCacheBackend {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let entries = self.entries.lock().expect("cache lock");
        let (bytes, stored_at) = entries.get(key)?;
        if stored_at.elapsed() < self.ttl {
            Some(bytes.clone())
        } else {
            None
        }
    }

    async fn set(&self, key: &str, bytes: Vec<u8>) {
        self.entries
            .lock()
            .expect("cache lock")
            .insert(key.to_string(), (bytes, Instant::now()));
    }

    async fn invalidate(&self, key: &str) {
        self.entries.lock().expect("cache lock").remove(key);
    }

    async fn clear(&self) {
        self.entries.lock().expect("cache lock").clear();
    }
}

/// Dev-reset third half: drop the config cache namespace so no stale
/// pre-reset bytes outlive the database + store wipe. Best-effort — the
/// cache self-heals within ten minutes and the reset must not fail
/// because an ephemeral layer did, so only connection failures surface
/// here and the caller warns instead of aborting.
pub async fn wipe_config_cache(config: &crate::AppConfig) -> Result<(), String> {
    if config.redis.url.is_none() {
        // Redis not configured — nothing the cache could hold.
        return Ok(());
    }
    let client = crate::infra::redis::connect(config)?;
    let manager = client
        .get_connection_manager()
        .await
        .map_err(|e| format!("redis unreachable during cache wipe: {e}"))?;
    RedisCacheBackend::new(manager).clear().await;
    Ok(())
}

/// The port-facing wrapper: every consumer keeps [`ConfigStore`]
/// signatures; caching is invisible plumbing between Redis and whichever
/// store is active (file locally, R2 in deployment).
pub struct CachedConfigStore {
    inner: Arc<dyn ConfigStore + Send + Sync>,
    backend: Arc<dyn CacheBackend + Send + Sync>,
}

impl CachedConfigStore {
    pub fn new(
        inner: Arc<dyn ConfigStore + Send + Sync>,
        backend: Arc<dyn CacheBackend + Send + Sync>,
    ) -> Self {
        Self { inner, backend }
    }
}

#[async_trait]
impl ConfigStore for CachedConfigStore {
    async fn load_raw(&self, key: &str) -> Result<Vec<u8>, String> {
        if let Some(bytes) = self.backend.get(key).await {
            return Ok(bytes);
        }
        let bytes = self.inner.load_raw(key).await?;
        self.backend.set(key, bytes.clone()).await;
        Ok(bytes)
    }

    async fn put_raw(&self, key: &str, bytes: Vec<u8>) -> Result<(), String> {
        self.inner.put_raw(key, bytes).await?;
        self.backend.invalidate(key).await;
        Ok(())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, String> {
        self.inner.list_keys(prefix).await
    }

    async fn delete_raw(&self, key: &str) -> Result<(), String> {
        self.inner.delete_raw(key).await?;
        self.backend.invalidate(key).await;
        Ok(())
    }

    async fn ensure_root_folders(&self) -> Result<(), String> {
        // An infra ensure, not data: straight to the wrapped store, no
        // cache traffic in either direction.
        self.inner.ensure_root_folders().await
    }
}

#[cfg(test)]
mod config_cache_tests {
    use super::*;

    /// Inner store that counts loads: the whole contract is "second read
    /// of a warm key does not reach the store".
    struct CountingStore {
        loads: Mutex<u32>,
        puts: Mutex<Vec<String>>,
        payload: Vec<u8>,
    }

    impl CountingStore {
        /// Fixed config-ish payload; tests only assert it round-trips.
        fn new() -> Self {
            Self {
                loads: Mutex::new(0),
                puts: Mutex::new(Vec::new()),
                payload: b"template body".to_vec(),
            }
        }

        fn loads(&self) -> u32 {
            *self.loads.lock().expect("count lock")
        }
    }

    #[async_trait::async_trait]
    impl ConfigStore for CountingStore {
        async fn load_raw(&self, _key: &str) -> Result<Vec<u8>, String> {
            *self.loads.lock().expect("count lock") += 1;
            Ok(self.payload.clone())
        }

        async fn put_raw(&self, key: &str, _bytes: Vec<u8>) -> Result<(), String> {
            self.puts.lock().expect("put lock").push(key.to_string());
            Ok(())
        }

        async fn delete_raw(&self, _key: &str) -> Result<(), String> {
            Ok(())
        }

        async fn list_keys(&self, _prefix: &str) -> Result<Vec<String>, String> {
            Ok(Vec::new())
        }

        async fn ensure_root_folders(&self) -> Result<(), String> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn warm_key_never_hits_the_store_twice() {
        let inner = Arc::new(CountingStore::new());
        let cached = CachedConfigStore::new(
            inner.clone(),
            Arc::new(MemoryCacheBackend::new(Duration::from_secs(60))),
        );

        let first = cached
            .load_raw("projects/acme/templates/welcome/email.html")
            .await
            .unwrap();
        let second = cached
            .load_raw("projects/acme/templates/welcome/email.html")
            .await
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(inner.loads(), 1, "second read must come from cache");
    }

    #[tokio::test]
    async fn put_invalidates_and_the_next_read_repopulates() {
        let inner = Arc::new(CountingStore::new());
        let cached = CachedConfigStore::new(
            inner.clone(),
            Arc::new(MemoryCacheBackend::new(Duration::from_secs(60))),
        );
        let key = "projects/acme/config/email/config.json";

        cached.load_raw(key).await.unwrap();
        assert_eq!(inner.loads(), 1);

        cached.put_raw(key, b"{}".to_vec()).await.unwrap();

        cached.load_raw(key).await.unwrap();
        assert_eq!(inner.loads(), 2, "invalidation must force a store read");
    }

    #[tokio::test]
    async fn delete_invalidates_so_reads_reach_the_store() {
        let inner = Arc::new(CountingStore::new());
        let cached = CachedConfigStore::new(
            inner.clone(),
            Arc::new(MemoryCacheBackend::new(Duration::from_secs(60))),
        );
        let key = "projects/acme/templates/welcome/sms.txt";

        cached.load_raw(key).await.unwrap();
        cached.load_raw(key).await.unwrap();
        assert_eq!(inner.loads(), 1, "second read warmed");

        cached.delete_raw(key).await.expect("delete");

        cached.load_raw(key).await.unwrap();
        assert_eq!(inner.loads(), 2, "delete must evict the warm entry");
    }

    #[tokio::test]
    async fn listing_passes_straight_through() {
        let inner = Arc::new(CountingStore::new());
        let cached = CachedConfigStore::new(
            inner.clone(),
            Arc::new(MemoryCacheBackend::new(Duration::from_secs(60))),
        );

        cached.list_keys("projects/acme").await.unwrap();
        cached.list_keys("projects/acme").await.unwrap();
        assert_eq!(inner.loads(), 0, "list must not warm the cache");
    }

    #[tokio::test]
    async fn expired_entries_fall_back_to_the_store() {
        let inner = Arc::new(CountingStore::new());
        let cached = CachedConfigStore::new(
            inner.clone(),
            Arc::new(MemoryCacheBackend::new(Duration::from_millis(25))),
        );
        let key = "projects/acme/templates/welcome/sms.txt";

        cached.load_raw(key).await.unwrap();
        tokio::time::sleep(Duration::from_millis(40)).await;
        cached.load_raw(key).await.unwrap();
        assert_eq!(inner.loads(), 2, "expired entries must re-read");
    }

    #[test]
    fn cache_key_prefix_isolates_the_namespace() {
        assert_eq!(
            cache_key!("projects/acme/config/email/config.json"),
            "notifi:cfg:projects/acme/config/email/config.json"
        );
    }

    #[tokio::test]
    async fn clear_drops_every_warm_entry_including_forever_keys() {
        let backend = MemoryCacheBackend::new(Duration::from_secs(60));
        backend
            .set("projects/acme/config/email/config.json", b"a".to_vec())
            .await;
        backend
            .set("projects/acme/templates/welcome/sms.txt", b"b".to_vec())
            .await;

        backend.clear().await;

        assert!(
            backend
                .get("projects/acme/config/email/config.json")
                .await
                .is_none(),
            "the wipe leaves no TTL'd key"
        );
        assert!(
            backend
                .get("projects/acme/templates/welcome/sms.txt")
                .await
                .is_none(),
            "the wipe leaves no warm key behind"
        );
    }
}
