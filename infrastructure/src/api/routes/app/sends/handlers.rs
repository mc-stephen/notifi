//! Send history for the deliveries view: newest-first sends per project.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::SendHistoryDto;
use crate::domain::error::AppError;
use crate::domain::sends::SendService;

type MaybeSendService = Option<Extension<Arc<SendService>>>;

fn require_service(extension: MaybeSendService) -> Result<Arc<SendService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AppError::NotConfigured.into())
}

/// `GET /app/sends?project_id=&limit=` — newest-first sends (delivery
/// history). `limit` clamps to 1..100, default 50.
pub async fn list_sends(
    CurrentUser(user): CurrentUser,
    service: MaybeSendService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let project_id = query
        .get("project_id")
        .ok_or_else(|| Problem::from(AppError::Validation("project_id is required".to_string())))?;
    let limit = query
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(50);
    let sends = service.list(user.id, project_id, limit).await?;
    let dtos: Vec<SendHistoryDto> = sends.into_iter().map(SendHistoryDto::from).collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "sends": dtos }))))
}
