//! Audit-log HTTP handlers.
//!
//! The service is injected as a required axum Extension by the composition
//! root; the route is absent when the service is unavailable.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Query};
use axum::http::StatusCode;

use crate::domain::audit::AuditService;
use crate::ports::audit_store::AuditFilters;
use super::dto::LogDto;
use super::super::auth::{CurrentUser, Problem};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 500;

/// `GET /v1/logs` — audit entries the current user may see (their own plus
/// project-scoped actions), newest first. Supports cursor pagination via
/// `?limit` and `?before=<id>`.
pub async fn list_logs(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<AuditService>>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let limit = query
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);

    let filters = AuditFilters {
        event_type: query.get("eventType").map(String::as_str),
        project_id: query.get("projectId").map(String::as_str),
    };

    let entries = service
        .list(&user.id.to_string(), filters, limit, query.get("before").map(String::as_str))
        .await?;

    let logs: Vec<LogDto> = entries.into_iter().map(LogDto::from).collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "logs": logs }))))
}
