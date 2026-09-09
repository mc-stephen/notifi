use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Query};
use axum::http::StatusCode;

use crate::domain::audit::AuditService;
use crate::domain::auth::errors::AuthError;
use crate::ports::audit_store::AdminAuditFilters;
use super::super::handlers::{CurrentAdmin, pagination, total_pages};
use super::super::super::auth::Problem;
use super::dto::AdminLogDto;

const DEFAULT_LIMIT: i64 = 50;

type MaybeAuditService = Option<Extension<Arc<AuditService>>>;

fn require_audit_service(extension: MaybeAuditService) -> Result<Arc<AuditService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `GET /admin/logs` — every audit entry (admin view), newest first.
pub async fn list_logs(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeAuditService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_audit_service(service)?;
    let (page, per_page, offset) = pagination(&query, DEFAULT_LIMIT);

    if let Some(t) = query.get("actorType")
        && !matches!(t.trim(), "user" | "admin" | "system")
    {
        return Err(AuthError::Validation(
            "invalid actorType (expected user, admin, or system)".to_string(),
        )
        .into());
    }

    let (entries, total) = service
        .list_all(
            AdminAuditFilters {
                event_type: query.get("eventType").map(String::as_str),
                project_id: query.get("projectId").map(String::as_str),
                actor_type: query.get("actorType").map(String::as_str),
                actor_id: query.get("actorId").map(String::as_str),
                search: query.get("search").map(String::as_str),
            },
            per_page,
            offset,
        )
        .await?;

    let dtos: Vec<AdminLogDto> = entries.into_iter().map(AdminLogDto::from).collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "logs": dtos,
            "page": page,
            "perPage": per_page,
            "total": total,
            "totalPages": total_pages(total, per_page),
        })),
    ))
}
