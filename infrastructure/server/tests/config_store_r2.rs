//! Live R2 config-store checks against the real bucket.
//!
//! Gated on the full `NOTIFI_R2_*` quartet (see `infra::config::R2Config`).
//! The bucket carries no defaults by design: `DefaultProjectLayout` in
//! code owns the arrangement and every project materializes its
//! `projects/{slug}/**` tree at creation, so these tests verify real ops
//! (ensured root folders, reset, tree report) rather than mirroring a
//! local folder.
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

/// Contract check on the live bucket: put → load → list behave for a
/// scratch key that never touches the projects namespace.
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

    match store.list_keys("projects/").await {
        Ok(keys) => println!("read probe OK — {} keys under projects/", keys.len()),
        Err(e) => panic!("read probe failed (credentials problem, not permissions): {e}"),
    }
}

/// Reset-to-empty: the code-side [`server::kernel::config
/// ::DefaultProjectLayout`] owns defaults, so the bucket's `projects/`
/// namespace ends fully cleared — real project folders materialize when
/// projects are created on the dashboard, and nothing needs preserving.
/// The top-level `.keep` folder markers are re-laid afterwards by the
/// factory, so a fresh bucket still shows `projects/` and `backup/`.
///
/// Destructive: it deletes everything under `projects/` before this test
/// passes — lists first, deletes through the same [`factory::reset`]
/// path `--reset-config` uses, then asserts the namespace holds only the
/// folder markers back.
#[tokio::test]
#[ignore = "live R2 reset — deletes EVERY projects/ key in the bucket"]
async fn reset_projects_to_empty() {
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
    println!("reset deleted {deleted} project keys");

    let store = r2_from_env();
    let keys = store
        .list_keys("projects/")
        .await
        .expect("list after reset");
    assert_eq!(
        keys,
        vec!["projects/.keep".to_string()],
        "projects/ must end with just the folder marker; got {keys:?}"
    );
}

/// The "keep track" look: prints what the bucket's projects namespace
/// holds right now. Run any time.
#[tokio::test]
#[ignore = "live R2 report — needs the NOTIFI_R2_* quartet"]
async fn report_projects_tree() {
    let store = r2_from_env();
    let keys = store.list_keys("projects/").await.expect("bucket lists");
    if keys.is_empty() {
        println!("projects/ — (empty; project folders materialize when projects are created)");
    } else {
        println!("projects/ tree ({} keys):", keys.len());
        for key in keys {
            println!("  {key}");
        }
    }
}

/// Same look for the backup folder — the worker's daily `.sql` file
/// history (keys only; the bucket holds no sizes in a flat listing).
/// Run any time.
#[tokio::test]
#[ignore = "live R2 report — needs the NOTIFI_R2_* quartet"]
async fn report_backup_tree() {
    let store = r2_from_env();
    let keys = store.list_keys("backup/").await.expect("bucket lists");
    if keys.is_empty() {
        println!("backup/ — (empty; no backup has been run yet)");
    } else {
        println!("backup/ tree ({} files):", keys.len());
        for key in keys {
            println!("  {key}");
        }
    }
}
