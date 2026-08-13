//! HTTP presentation layer: router assembly, middleware wiring, error mapping.

pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;

use axum::Router;

use crate::state::AppState;

/// Builds the application router: mounts the full route surface
/// ([`routes::all`]) and wraps it in the global middleware chain.
pub fn build_router(state: AppState) -> Router {
    middleware::apply(routes::all(state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
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
