//! HTTP presentation layer: router, middleware wiring, error mapping, handlers.

pub mod error;
pub mod handlers;

use axum::Router;
use axum::http::header::HeaderName;
use axum::routing::get;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

use crate::state::AppState;

/// Request-id header propagated into responses and logs for correlation.
pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// Builds the application router with its full middleware stack.
///
/// Layers (outermost last): trace/logging, CORS (permissive for dev), request-id
/// propagation. The fallback renders RFC 9457 problem documents for 404/405.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::root))
        .route("/healthz", get(handlers::healthz))
        .route("/readyz", get(handlers::readyz))
        .fallback(handlers::not_found)
        .with_state(state)
        .layer(PropagateRequestIdLayer::new(REQUEST_ID_HEADER))
        .layer(SetRequestIdLayer::new(REQUEST_ID_HEADER, MakeRequestUuid))
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_app() -> Router {
        build_router(AppState {
            db: None,
            redis: None,
        })
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

    #[test]
    fn router_state_is_clonable() {
        let state = AppState {
            db: None,
            redis: None,
        };
        let _ = Arc::new(state);
    }
}
