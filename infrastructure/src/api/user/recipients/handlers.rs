//! Recipient HTTP handlers.
//!
//! The service is injected as a required axum Extension by the composition
//! root; the route is absent when the service is unavailable.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use crate::domain::recipients::RecipientService;
use super::dto::{CreateRecipientRequest, RecipientDto, UpdateRecipientRequest};
use super::super::auth::{CurrentUser, Problem};

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

/// `POST /v1/projects/:project_id/recipients` — create a recipient.
pub async fn create_recipient(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<RecipientService>>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateRecipientRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let contact_default = serde_json::Value::Null;
    let recipient = service
        .create(
            user.id,
            &project_id,
            &body.user_id,
            &body.name,
            body.contacts.unwrap_or(contact_default),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "recipient": RecipientDto::from(recipient) })),
    ))
}

/// `GET /v1/projects/:project_id/recipients` — list, newest first.
pub async fn list_recipients(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<RecipientService>>,
    Path(project_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let limit = query
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);

    let recipients = service
        .list(
            user.id,
            &project_id,
            query.get("search").map(String::as_str),
            limit + 1,
            query.get("before").map(String::as_str),
        )
        .await?;

    let has_more = recipients.len() > limit as usize;
    let dtos: Vec<RecipientDto> = recipients
        .into_iter()
        .take(limit as usize)
        .map(RecipientDto::from)
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "recipients": dtos, "hasMore": has_more })),
    ))
}

/// `GET /v1/projects/:project_id/recipients/:id` — one recipient.
pub async fn get_recipient(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<RecipientService>>,
    Path((project_id, recipient_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let recipient = service
        .get(user.id, &project_id, &recipient_id)
        .await?
        .ok_or_else(|| crate::domain::auth::errors::AuthError::NotFound("recipient not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "recipient": RecipientDto::from(recipient) })),
    ))
}

/// `PATCH /v1/projects/:project_id/recipients/:id` — update name/contacts.
pub async fn update_recipient(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<RecipientService>>,
    Path((project_id, recipient_id)): Path<(String, String)>,
    Json(body): Json<UpdateRecipientRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let recipient = service
        .update(
            user.id,
            &project_id,
            &recipient_id,
            body.name.as_deref(),
            body.contacts,
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "recipient": RecipientDto::from(recipient) })),
    ))
}

/// `DELETE /v1/projects/:project_id/recipients/:id` — soft delete.
pub async fn delete_recipient(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<RecipientService>>,
    Path((project_id, recipient_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    service.remove(user.id, &project_id, &recipient_id).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}
