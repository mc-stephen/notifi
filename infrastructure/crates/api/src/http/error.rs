//! RFC 9457 problem-details rendering for the unified error model.
//!
//! [`ApiError`] lives in `notifi_core` (framework-free); the orphan rule keeps
//! us from implementing axum's `IntoResponse` for it here, so we wrap it in
//! the local [`ApiProblem`] newtype.
//!
//! This is presentation-layer code by design: the error *model* stays in the
//! kernel (`crates/core/src/error.rs`) so every crate can share it without
//! depending on axum (see `infrastructure/docs/ARCHITECTURE.md`).

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use notifi_core::error::ApiError;
use serde_json::{Value, json};

/// Problem-document wrapper around [`ApiError`], renderable as an HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiProblem(pub ApiError);

impl From<ApiError> for ApiProblem {
    fn from(err: ApiError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiProblem {
    fn into_response(self) -> Response {
        let err = self.0;
        let status = StatusCode::from_u16(err.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // correlation_id is serialized as null when absent; the RFC 9457
        // content type lets clients negotiate machine-readable errors.
        let problem: Value = json!({
            "type": err.type_url,
            "title": err.title,
            "status": err.status,
            "detail": err.detail,
            "correlation_id": err.correlation_id,
        });

        (
            status,
            [("content-type", "application/problem+json")],
            Json(problem),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn api_problem_renders_a_problem_document() {
        let res = ApiProblem(ApiError::bad_request("bad input")).into_response();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
        let content_type = res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(content_type.starts_with("application/problem+json"));

        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["title"], "Bad Request");
        assert_eq!(body["status"], 400);
        assert_eq!(body["detail"], "bad input");
        assert_eq!(body["type"], "about:blank");
    }

    #[tokio::test]
    async fn correlation_id_is_included_when_set() {
        let res =
            ApiProblem(ApiError::not_found("nope").with_correlation_id("abc-123")).into_response();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["correlation_id"], "abc-123");
    }
}
