//! HTTP presentation layer: router assembly, middleware wiring, error mapping.
//!
//! Two independent API surfaces:
//! * **project** — the product API customers integrate with, served at the
//!   root ([`project`]). Evolves independently; no version promise.
//! * **user** — the dashboard backend, versioned under `/v1`
//!   ([`user`]); `/v2` can later coexist without breaking v1.
//!
//! Ops routes (`/`, `/healthz`, `/readyz`, `/routes`) live outside both
//! surfaces. See `catalog.rs` for the full route registry.

pub mod catalog;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod project;
pub mod state;
pub use state::AppState;
pub mod user;

use axum::Router;

use crate::infra::config::AppConfig;



/// Builds the application router: ops routes at the root, the project
/// surface merged in, and the versioned user surface nested under `/v1` —
/// all wrapped in the global middleware chain.
pub fn build_router(state: AppState, config: &AppConfig) -> Router {
    let router = Router::new()
        .route("/", axum::routing::get(handlers::root))
        .route("/healthz", axum::routing::get(handlers::healthz))
        .route("/readyz", axum::routing::get(handlers::readyz))
        .route("/routes", axum::routing::get(handlers::catalog))
        .merge(project::router())
        .nest("/v1", user::v1_router(&state))
        .fallback(handlers::not_found)
        .with_state(state);
    middleware::apply(router, &config.server.cors_origins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_app() -> Router {
        build_router(
            AppState {
                db: None,
                redis: None,
                auth: None,
                oauth: None,
                projects: None,
            },
            &AppConfig::default(),
        )
    }

    #[tokio::test]
    async fn root_returns_service_info() {
        let res = test_app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn healthz_returns_ok() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn routes_catalog_is_served() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/routes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_path_is_a_problem_document() {
        let res = test_app()
            .oneshot(Request::builder().uri("/nope").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(content_type.starts_with("application/problem+json"));
    }

    #[tokio::test]
    async fn readyz_reports_503_without_dependencies() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/readyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn responses_carry_request_id() {
        let res = test_app()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(res.headers().contains_key("x-request-id"));
    }
}
