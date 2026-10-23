//! Live R2 config-store checks against the real bucket.
//!
//! Gated on the full `NOTIFI_R2_*` quartet (see `infra::config::R2Config`).
//! The seeder mirrors the local `server/assets/brands/**` tree into the
//! bucket byte-for-byte, `_default` included — the structural reference
//! for dev/AI must exist in the cloud exactly as it does locally.
//!
//! ```shell
//! cargo test -p server --test config_store_r2 -- --ignored
//! ```

use server::infra::R2ConfigStore;
use server::infra::config::R2Config;
use server::ports::ConfigStore;

fn r2_from_env() -> R2ConfigStore {
    let config = R2Config {
        account_id: std::env::var("NOTIFI_R2_ACCOUNT_ID").ok(),
        access_key_id: std::env::var("NOTIFI_R2_ACCESS_KEY_ID").ok(),
        secret_access_key: std::env::var("NOTIFI_R2_SECRET_ACCESS_KEY").ok(),
        bucket: std::env::var("NOTIFI_R2_BUCKET").ok(),
    };
    R2ConfigStore::from_config(&config).expect("r2 store builds from NOTIFI_R2_* env")
}

/// Walks a directory recursively, returning (relative key, bytes) pairs
/// with `/` separators in sorted order (determinism for the seed diff).
fn collect_files(dir: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).expect("walk brands root") {
            let entry = entry.expect("walk entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let bytes = std::fs::read(&path).expect("read file");
                let rel = path
                    .strip_prefix(dir)
                    .expect("inside brands root")
                    .to_string_lossy()
                    .to_string();
                out.push((rel, bytes));
            }
        }
    }
    out.sort();
    out
}

/// One-shot seeding of `server/assets/brands/**` into the bucket. Keys are
/// `brands/...` exactly, mirroring the local folder layout 1:1 — the
/// `_default` structural reference included. Idempotent: re-running just
/// rewrites the same objects.
#[tokio::test]
#[ignore = "live R2 seeding — needs NOTIFI_R2_ACCOUNT_ID, NOTIFI_R2_ACCESS_KEY_ID, \
            NOTIFI_R2_SECRET_ACCESS_KEY, NOTIFI_R2_BUCKET"]
async fn seed_brands_tree_to_r2() {
    let store = r2_from_env();

    let brands_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/brands");
    let files = collect_files(&brands_root);
    assert!(!files.is_empty(), "assets/brands must exist locally");

    for (relative, bytes) in &files {
        let key = format!("brands/{relative}");
        store
            .put_raw(&key, bytes.clone())
            .await
            .expect("seed upload");
    }
    println!("seeded {} files from assets/brands/**", files.len());

    // Verify: every local file lands under its identical bucket key.
    let keys = store.list_keys("brands/").await.expect("list bucket");
    for (relative, _) in &files {
        let key = format!("brands/{relative}");
        assert!(
            keys.iter().any(|k| k == &key),
            "bucket missing {key}; listed: {keys:?}"
        );
    }
}

/// Contract check on the live bucket: put → load → list behave for a
/// scratch key that never touches the brands namespace.
#[tokio::test]
#[ignore = "live R2 roundtrip — needs the NOTIFI_R2_* quartet"]
async fn live_put_load_list_roundtrip() {
    let store = r2_from_env();

    let scratch = "_r2-roundtrip-smoke/hello.json";
    store
        .put_raw(scratch, br#"{"ok":true}"#.to_vec())
        .await
        .expect("put scratch key");

    let loaded = store.load_raw(scratch).await.expect("load scratch key");
    assert_eq!(loaded, br#"{"ok":true}"#.to_vec());

    let listed = store
        .list_keys("_r2-roundtrip-smoke/")
        .await
        .expect("list scratch prefix");
    assert!(listed.iter().any(|k| k == scratch), "listed: {listed:?}");
}

/// Credential scope probe: listing works even on read-only tokens. If this
/// passes while the roundtrip PUT fails, the token lacks write scope —
/// recreate it with "Object Read & Write" and refresh the two values.
#[tokio::test]
#[ignore = "live R2 read probe — needs the NOTIFI_R2_* quartet"]
async fn live_read_only_list_probe() {
    let store = r2_from_env();

    match store.list_keys("brands/").await {
        Ok(keys) => println!("read probe OK — {} keys under brands/", keys.len()),
        Err(e) => panic!("read probe failed (credentials problem, not permissions): {e}"),
    }
}

/// Reset-to-empty (Option A): the code-side [`server::kernel::config
/// ::DefaultBrandLayout`] owns defaults, so the bucket's `brands/`
/// namespace ends fully cleared — real brands materialize when projects
/// are created on the dashboard, and nothing needs preserving.
///
/// Destructive: it deletes everything under `brands/` before this test
/// passes — lists first, deletes through the same [`factory::reset`]
/// path `--reset-config` uses, then asserts the namespace is empty.
#[tokio::test]
#[ignore = "live R2 reset — deletes EVERY brands/ key in the bucket"]
async fn reset_brands_to_empty() {
    let config = server::infra::config::AppConfig {
        r2: R2Config {
            account_id: std::env::var("NOTIFI_R2_ACCOUNT_ID").ok(),
            access_key_id: std::env::var("NOTIFI_R2_ACCESS_KEY_ID").ok(),
            secret_access_key: std::env::var("NOTIFI_R2_SECRET_ACCESS_KEY").ok(),
            bucket: std::env::var("NOTIFI_R2_BUCKET").ok(),
        },
        ..server::infra::config::AppConfig::default()
    };

    let deleted = server::infra::config_store_factory::reset(&config)
        .await
        .expect("reset through the shared factory path");
    println!("reset deleted {deleted} brand keys");

    let store = r2_from_env();
    let keys = store.list_keys("brands/").await.expect("list after reset");
    assert!(keys.is_empty(), "brands/ must end empty; got {keys:?}");
}

/// The "keep track" look: prints what the bucket's brands namespace holds
/// right now. Run any time.
#[tokio::test]
#[ignore = "live R2 report — needs the NOTIFI_R2_* quartet"]
async fn report_brands_tree() {
    let store = r2_from_env();
    let keys = store.list_keys("brands/").await.expect("bucket lists");
    if keys.is_empty() {
        println!("brands/ — (empty; brands materialize when projects are created)");
    } else {
        println!("brands/ tree ({} keys):", keys.len());
        for key in keys {
            println!("  {key}");
        }
    }
}
