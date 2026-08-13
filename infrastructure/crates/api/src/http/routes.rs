//! Central route registry — every route the API exposes is declared here.
//!
//! Feature routes are DEFINED inside their own crate
//! (`<feature>/presentation/routes.rs` per `docs/ARCHITECTURE.md` §3) and
//! MOUNTED here. To add a route: declare it in [`ROUTES`], then mount it in
//! [`all`]. Both live in this file, so drift is visible immediately.

use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use serde_json::{Value, json};

use crate::http::handlers;
use crate::state::AppState;

/// One exposed route, as listed in the catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RouteInfo {
    pub method: &'static str,
    pub path: &'static str,
    /// Owning feature: `core` for API infrastructure, or the domain name
    /// (`auth`, `notifications`, ...) for feature routes.
    pub feature: &'static str,
    pub description: &'static str,
}

/// The full route catalog: drives `GET /routes` and mirrors [`all`].
pub const ROUTES: &[RouteInfo] = &[
    RouteInfo {
        method: "GET",
        path: "/",
        feature: "core",
        description: "service identification",
    },
    RouteInfo {
        method: "GET",
        path: "/healthz",
        feature: "core",
        description: "liveness",
    },
    RouteInfo {
        method: "GET",
        path: "/readyz",
        feature: "core",
        description: "readiness",
    },
    RouteInfo {
        method: "GET",
        path: "/routes",
        feature: "core",
        description: "this route catalog",
    },
];

/// Builds the router for the whole API surface (no middleware; applied by
/// [`crate::http::build_router`]).
pub fn all(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::root))
        .route("/healthz", get(handlers::healthz))
        .route("/readyz", get(handlers::readyz))
        .route("/routes", get(catalog))
        .fallback(handlers::not_found)
        .with_state(state)
}

/// `GET /routes` — returns the route catalog as JSON.
#[allow(clippy::unused_async)]
pub async fn catalog() -> Json<Value> {
    Json(json!({
        "service": env!("CARGO_PKG_VERSION"),
        "routes": ROUTES,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_every_core_route() {
        let paths: Vec<&str> = ROUTES.iter().map(|r| r.path).collect();
        assert!(paths.contains(&"/"));
        assert!(paths.contains(&"/healthz"));
        assert!(paths.contains(&"/readyz"));
        assert!(paths.contains(&"/routes"));
    }

    #[test]
    fn catalog_entries_are_unique() {
        let mut seen = Vec::new();
        for route in ROUTES {
            let key = (route.method, route.path);
            assert!(!seen.contains(&key), "duplicate route: {key:?}");
            seen.push(key);
        }
    }
}
