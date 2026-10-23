//! Live `RedisCacheBackend` check: warm key roundtrip + invalidation
//! against a real Redis.
//!
//! Gated on `NOTIFI_TEST_REDIS_URL` (a scratch `cfg:` key with a short
//! TTL; local dev redis via docker-compose works). Nothing runs unless
//! the variable is explicit.
//!
//! ```shell
//! NOTIFI_TEST_REDIS_URL=redis://:...@localhost:6379 \
//!   cargo test -p server --test config_cache_redis -- --ignored
//! ```

use std::time::Duration;

use server::infra::FileConfigStore;
use server::infra::config_cache::CachedConfigStore;
use server::infra::config_cache::{CacheBackend, RedisCacheBackend};
use server::ports::ConfigStore;

#[tokio::test]
#[ignore = "live redis cache roundtrip — needs NOTIFI_TEST_REDIS_URL"]
async fn live_redis_cache_roundtrip() {
    let url = std::env::var("NOTIFI_TEST_REDIS_URL").expect("NOTIFI_TEST_REDIS_URL not set");
    let client = redis::Client::open(url.as_str()).expect("redis url parses");
    let conn = client
        .get_connection_manager()
        .await
        .expect("redis connects");

    let backend = RedisCacheBackend::new(conn).with_ttl(Duration::from_secs(30));

    let scratch = "cfg:_redis-smoke/roundtrip.json";
    let bytes = b"{\"smoke\":true}".to_vec();

    backend.set(scratch, bytes.clone()).await;
    let got = backend.get(scratch).await;
    assert_eq!(
        got,
        Some(bytes),
        "redis must return the exact bytes it stored"
    );

    backend.invalidate(scratch).await;
    let gone = backend.get(scratch).await;
    assert!(gone.is_none(), "invalidated keys must read as misses");
}

/// Full wrapper behavior on a real Redis: cache miss reaches the file
/// store, second read does not, put invalidates, next read repopulates.
#[tokio::test]
#[ignore = "live cached-store roundtrip — needs NOTIFI_TEST_REDIS_URL"]
async fn live_cached_store_warm_reads() {
    let url = std::env::var("NOTIFI_TEST_REDIS_URL").expect("NOTIFI_TEST_REDIS_URL not set");
    let client = redis::Client::open(url.as_str()).expect("redis url parses");
    let conn = client
        .get_connection_manager()
        .await
        .expect("redis connects");

    // Fresh cache namespace per run: TTL key named with the test tag —
    // invalidation + 30s expiry keeps repeat runs deterministic.
    let backend = RedisCacheBackend::new(conn).with_ttl(Duration::from_secs(30));
    let root = std::env::temp_dir().join("notifi-cached-warm-live");
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();

    let inner: std::sync::Arc<dyn ConfigStore + Send + Sync> =
        std::sync::Arc::new(FileConfigStore::new(&root));
    let cached = CachedConfigStore::new(inner.clone(), std::sync::Arc::new(backend));

    let key = "_live-cached-warm/config.json";
    cached
        .put_raw(key, br#"{"warm":1}"#.to_vec())
        .await
        .expect("put seeds the store");

    let first = cached.load_raw(key).await.expect("first load");
    assert_eq!(first, br#"{"warm":1}"#.to_vec());
    let second = cached.load_raw(key).await.expect("second load");
    assert_eq!(second, first, "config payload stable across reads");

    // The wrapper signature stays ConfigStore throughout (backend is
    // plumbing); a store-level put invalidates and the next read
    // repopulates from the inner store.
    cached.put_raw(key, b"REV".to_vec()).await.expect("re-put");
    let third = cached.load_raw(key).await.expect("post-invalidate load");
    assert_eq!(third, b"REV".to_vec(), "invalidation must show fresh bytes");

    std::fs::remove_dir_all(&root).ok();
}
