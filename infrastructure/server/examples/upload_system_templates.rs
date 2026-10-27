//! One-time ops helper: upload system template bodies into the bucket's
//! `templates/` folder. The server never writes templates — bodies are
//! put there by ops (dashboard drag-drop, or this).
//!
//! Usage — pass one or more `.html` files; the file name becomes the
//! template name (`verify-email.html` → renders as `verify-email`):
//!
//! ```sh
//! cargo run -p server --example upload_system_templates -- <file.html>...
//! ```
//!
//! Env: the same `NOTIFI_R2_*` quartet the config store uses.

use std::sync::Arc;

use server::infra::R2ConfigStore;
use server::infra::config::R2Config;
use server::ports::ConfigStore;

fn r2_from_env() -> R2Config {
    R2Config {
        account_id: std::env::var("NOTIFI_R2_ACCOUNT_ID").ok(),
        access_key_id: std::env::var("NOTIFI_R2_ACCESS_KEY_ID").ok(),
        secret_access_key: std::env::var("NOTIFI_R2_SECRET_ACCESS_KEY").ok(),
        bucket: std::env::var("NOTIFI_R2_BUCKET").ok(),
    }
}

async fn run(paths: Vec<String>) {
    let store = Arc::new(
        R2ConfigStore::from_config(&r2_from_env()).expect("NOTIFI_R2_* must be configured"),
    );

    for path in &paths {
        let body = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let name = std::path::Path::new(path)
            .file_name()
            .expect("file path")
            .to_string_lossy();
        let key = format!("templates/{name}");
        let len = body.len();
        store
            .put_raw(&key, body)
            .await
            .unwrap_or_else(|e| panic!("put {key}: {e}"));
        println!("PUT {key} ({len} bytes)");
    }

    println!(
        "templates/ now holds: {:?}",
        store.list_keys("templates").await
    );
}

fn main() {
    dotenvy::dotenv().ok();
    let paths: Vec<String> = std::env::args().skip(1).collect();
    assert!(!paths.is_empty(), "pass one or more .html files");
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(run(paths));
}
