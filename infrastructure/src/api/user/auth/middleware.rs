//! Session authentication: the [`CurrentUser`] extractor.
//!
//! The service is injected by the composition root as an axum `Extension`
//! (`Arc<AuthService>`), which keeps this crate decoupled from the api's
//! `AppState` while still allowing any feature to require an authenticated
//! user simply by taking a `CurrentUser` parameter.

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use notifi_core::error::IntoApiError;
use serde_json::json;

use crate::domain::auth::AuthService;
use crate::domain::auth::entities::User;
use crate::domain::auth::errors::AuthError;

/// Cookie name shared with the dashboard proxy (`proxy.ts`).
pub const SESSION_COOKIE: &str = "session_token";

/// Renders an [`AuthError`] as an RFC 9457 problem document.
///
/// Local twin of the api crate's `ApiProblem` (the kernel error model must
/// stay framework-free, and cross-crate impls hit the orphan rule); both are
/// consolidated into a shared http-support crate when the next domain lands.
pub fn problem_response(err: AuthError) -> Response {
    let problem = err.to_api_error();
    // Server-side log keeps the underlying cause (e.g. raw sqlx error
    // strings inside `Storage`) — the wire detail stays generic on purpose.
    if problem.status >= 500 {
        tracing::error!(status = problem.status, error = %err, "auth request failed");
    }
    let status = axum::http::StatusCode::from_u16(problem.status)
        .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let body = json!({
        "type": problem.type_url,
        "title": problem.title,
        "status": problem.status,
        "detail": problem.detail,
        "correlation_id": problem.correlation_id,
    });
    (
        status,
        [("content-type", "application/problem+json")],
        axum::Json(body),
    )
        .into_response()
}

/// Handler-error wrapper around a pre-rendered problem response.
///
/// Boxed so handler signatures can be `Result<_, Problem>` without tripping
/// clippy's `result_large_err` (a bare `Response` is too large).
pub struct Problem(Box<Response>);

impl From<AuthError> for Problem {
    fn from(err: AuthError) -> Self {
        Self(Box::new(problem_response(err)))
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        *self.0
    }
}

/// Extractor for endpoints that require a signed-in user.
///
/// Fails with 401 when the session cookie is missing/invalid and with 503
/// when auth itself is unavailable (no database configured).
#[derive(Debug, Clone)]
pub struct CurrentUser(pub Arc<User>);

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let service = parts
            .extensions
            .get::<Arc<AuthService>>()
            .cloned()
            .ok_or_else(|| problem_response(AuthError::NotConfigured))?;

        let jar = CookieJar::from_request_parts(parts, _state)
            .await
            .map_err(|_| problem_response(AuthError::Unauthorized))?;

        let raw_cookie = jar
            .get(SESSION_COOKIE)
            .map(|cookie| cookie.value().to_string())
            .ok_or_else(|| problem_response(AuthError::Unauthorized))?;

        let user = service
            .authenticate(&raw_cookie)
            .await
            .map_err(problem_response)?;

        Ok(Self(Arc::new(user)))
    }
}
