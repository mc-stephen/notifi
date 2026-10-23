//! `ConfigStore` port ↔ file driver roundtrips versus the real assets tree.
//!
//! Uses a private temp root; the live R2 driver gets its own gated file.

use server::infra::FileConfigStore;
use server::kernel::config::ConfigResolver;
use server::ports::ConfigStore;

fn temp_root(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("notifi-fcs-{name}"));
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[tokio::test]
async fn channel_config_roundtrip_with_kernel_keys() {
    // The kernel and the file driver MUST share one root, so the test pins
    // NOTIFI_CONFIG_ROOT to its temp root and builds both from it — kernel
    // paths and store keys stay one convention without touching repo assets.
    // All env reads happen synchronously before any await (clippy guard).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = ENV_LOCK.lock().unwrap();
    let root = temp_root("kernel-match");
    // SAFETY: process-local mutation, guarded by ENV_LOCK; restored below.
    unsafe { std::env::set_var("NOTIFI_CONFIG_ROOT", &root) };
    let store = FileConfigStore::from_root_env();
    let kernel_dir = ConfigResolver::channel_dir("acme", "email");
    unsafe { std::env::remove_var("NOTIFI_CONFIG_ROOT") };
    drop(_guard);

    let key = ConfigResolver::channel_key("acme", "email", "config.json");
    store
        .put_raw(&key, br#"{"port":587,"host":"smtp.example.com"}"#.to_vec())
        .await
        .expect("put channel config");

    let raw = store.load_raw(&key).await.expect("load back");
    let cfg: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(cfg["host"], "smtp.example.com");

    // The file landing spot must equal the kernel path helpers' target.
    assert_eq!(root.join(&key), kernel_dir.join("config.json"));

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn brand_template_assets_roundtrip() {
    let root = temp_root("templates");
    let store = FileConfigStore::new(&root);
    let html_key = ConfigResolver::template_key("acme", "welcome", "email.html");
    let txt_key = ConfigResolver::template_key("acme", "welcome", "sms.txt");

    store
        .put_raw(&html_key, b"<p>Hi {{name}}</p>".to_vec())
        .await
        .expect("put html");
    store
        .put_raw(&txt_key, b"Hi {{name}}".to_vec())
        .await
        .expect("put txt");

    let keys = store
        .list_keys("brands/acme/templates")
        .await
        .expect("list");
    assert_eq!(
        keys,
        vec![
            "brands/acme/templates/welcome/email.html".to_string(),
            "brands/acme/templates/welcome/sms.txt".to_string(),
        ]
    );

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn missing_key_is_an_explicit_error_not_a_panic() {
    let root = temp_root("missing");
    let store = FileConfigStore::new(&root);
    let err = store
        .load_raw("brands/acme/config/sms/config.json")
        .await
        .expect_err("missing key");
    assert!(err.contains("not found"), "got: {err}");
    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn brands_write_like_any_other_brand() {
    // Brand ids are just keys: nothing anywhere in the port special-cases
    // one (the struct-owned layout decides defaults; storage is neutral).
    let root = temp_root("default");
    let store = FileConfigStore::new(&root);
    let key = ConfigResolver::channel_key("the_team", "sms", "config.json");
    store
        .put_raw(&key, b"{}".to_vec())
        .await
        .expect("put brand key");
    assert!(
        root.join("brands/the_team/config/sms/config.json")
            .is_file()
    );
    std::fs::remove_dir_all(&root).ok();
}
