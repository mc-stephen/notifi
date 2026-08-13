//! Cross-origin resource sharing.
//!
//! Dev-permissive so the dashboard (`:3000`) can call the API (`:8080`).
//! Tighten to an allowlist in M8 (hardening).

use tower_http::cors::CorsLayer;

/// Permissive CORS for local development.
pub fn permissive() -> CorsLayer {
    CorsLayer::permissive()
}
