//! Device HTTP handlers.
//!
//! The service is injected as a required axum Extension by the composition
//! root; the route is absent when the service is unavailable.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{DeviceDto, RegisterDeviceRequest};
use crate::domain::devices::DeviceService;

/// `POST /app/projects/:project_id/devices` — register (or refresh) a
/// device token. The same `(project, destination, token)` refreshing
/// updates instead of duplicating.
pub async fn register_device(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<DeviceService>>,
    Path(project_id): Path<String>,
    Json(body): Json<RegisterDeviceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let device = service
        .register(
            user.id,
            &project_id,
            crate::ports::devices_store::DeviceRegistration {
                destination: body.destination,
                vendor: body.vendor,
                token: body.token,
                label: body.label,
                recipient_id: body.recipient_id,
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "device": DeviceDto::from(device) })),
    ))
}

/// `GET /app/projects/:project_id/devices` — list, newest first, with an
/// optional `recipientId` filter.
pub async fn list_devices(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<DeviceService>>,
    Path(project_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let devices = service
        .list(
            user.id,
            &project_id,
            query.get("recipientId").map(String::as_str),
        )
        .await?;

    let dtos: Vec<DeviceDto> = devices.into_iter().map(DeviceDto::from).collect();

    Ok((StatusCode::OK, Json(serde_json::json!({ "devices": dtos }))))
}

/// `GET /app/projects/:project_id/devices/:id` — one device.
pub async fn get_device(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<DeviceService>>,
    Path((project_id, device_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let device = service
        .get(user.id, &project_id, &device_id)
        .await?
        .ok_or_else(|| crate::domain::error::AppError::NotFound("device not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "device": DeviceDto::from(device) })),
    ))
}

/// `DELETE /app/projects/:project_id/devices/:id` — soft-revoke (status
/// flip; the row stays for delivery history).
pub async fn revoke_device(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<DeviceService>>,
    Path((project_id, device_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    service
        .revoke(user.id, &project_id, &device_id)
        .await?
        .ok_or_else(|| crate::domain::error::AppError::NotFound("device not found".into()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}
