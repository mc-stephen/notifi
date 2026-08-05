//! Handlers for the API root and health endpoints.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use notifi_core::error::ApiError;
use serde_json::{Value, json};

use crate::http::error::ApiProblem;
use crate::state::AppState;

/// Fallback for unknown routes and wrong methods: uniform 404 problem document.
#[allow(clippy::unused_async)]
pub async fn not_found() -> ApiProblem {
    ApiProblem::from(ApiError::not_found("no route matches the requested path"))
}

/// `GET /` — service identification.
#[allow(clippy::unused_async)]
pub async fn root() -> Json<Value> {
    Json(json!({
        "service": "notifi-api",
        "version": env!("CARGO_PKG_VERSION"),
        "milestone": "M1",
    }))
}

/// `GET /healthz` — liveness: the process is up and serving.
#[allow(clippy::unused_async)]
pub async fn healthz() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// `GET /readyz` — readiness: are external dependencies reachable?
///
/// 200 when both Postgres and Redis are configured AND healthy; otherwise a
/// 503 problem document reporting exactly which dependency is down.
pub async fn readyz(State(state): State<AppState>) -> Response {
    let database = match &state.db {
        Some(pool) => sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(pool)
            .await
            .is_ok(),
        None => false,
    };

    let redis = match &state.redis {
        Some(client) => match client.get_multiplexed_async_connection().await {
            Ok(mut conn) => redis::cmd("PING")
                .query_async::<String>(&mut conn)
                .await
                .is_ok(),
            Err(_) => false,
        },
        None => false,
    };

    if database && redis {
        return Json(json!({
            "status": "ready",
            "database": true,
            "redis": true,
        }))
        .into_response();
    }

    ApiProblem::from(ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE.as_u16(),
        "about:blank",
        "Service Unavailable",
        format!("dependency check failed — database: {database}, redis: {redis}"),
    ))
    .into_response()
}
