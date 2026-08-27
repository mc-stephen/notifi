//! Cross-origin resource sharing.
//!
//! Cookie-based auth requires an explicit origin allowlist (browsers refuse
//! `Access-Control-Allow-Origin: *` together with credentials), so the
//! permissive dev shortcut is gone. Origins come from
//! `NOTIFI_CORS_ORIGINS` (comma-separated; defaults to the dashboard's
//! `http://localhost:3000`).

use axum::http::{Method, header::CONTENT_TYPE};
use tower_http::cors::CorsLayer;

/// Builds the CORS layer for the configured browser origins.
pub fn layer(origins: &[String]) -> CorsLayer {
    let allowed: Vec<_> = origins.iter().filter_map(|o| o.parse().ok()).collect();

    CorsLayer::new()
        .allow_origin(allowed)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(600))
}
