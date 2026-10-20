//! Webhook CRUD. Owner doctrine: creating a webhook creates its flow
//! (named after it, webhook trigger seeded with a fresh token); renaming
//! the webhook renames the flow; deleting the webhook deletes that flow.
//! Owned flows stay off the `/flows` list; the builder stays reachable
//! by id. Archive (PATCH `archived`) is the non-destructive alternative.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{CreateWebhookRequest, UpdateWebhookRequest, WebhookDto};
use crate::domain::webhooks::WebhookService;
use crate::domain::webhooks::entities::{WebhookInput, WebhookPatch};
use std::collections::HashMap;

type MaybeWebhookService = Option<Extension<Arc<WebhookService>>>;

fn require_service(extension: MaybeWebhookService) -> Result<Arc<WebhookService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| crate::domain::error::AppError::NotConfigured.into())
}

fn project_id(query: &HashMap<String, String>) -> Result<&String, Problem> {
    query.get("project_id").ok_or_else(|| {
        Problem::from(crate::domain::error::AppError::Validation(
            "project_id is required".to_string(),
        ))
    })
}

/// `GET /app/webhooks?project_id=` — all webhooks, oldest first.
pub async fn list_webhooks(
    CurrentUser(user): CurrentUser,
    service: MaybeWebhookService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let webhooks = service.list(user.id, project_id(&query)?).await?;
    let dtos: Vec<WebhookDto> = webhooks.into_iter().map(WebhookDto::from).collect();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "webhooks": dtos })),
    ))
}

/// `GET /app/webhooks/:id?project_id=` — one webhook with its config.
pub async fn get_webhook(
    CurrentUser(user): CurrentUser,
    service: MaybeWebhookService,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let webhook = service.get(user.id, project_id(&query)?, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "webhook": WebhookDto::from(webhook) })),
    ))
}

/// `POST /app/webhooks` — create a webhook and its flow (the flow is
/// born here, named after the webhook; never assigned).
pub async fn create_webhook(
    CurrentUser(user): CurrentUser,
    service: MaybeWebhookService,
    Json(body): Json<CreateWebhookRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let webhook = service
        .create(
            user.id,
            &body.project_id,
            WebhookInput {
                name: body.name,
                description: body.description.unwrap_or_default(),
                events: body.events.unwrap_or_default(),
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "webhook": WebhookDto::from(webhook) })),
    ))
}

/// `PATCH /app/webhooks/:id` — edit name, description, events,
/// pause/resume, or archive/restore (renames sync down to the owned
/// flow; event changes rewrite the locked trigger node, token intact).
pub async fn update_webhook(
    CurrentUser(user): CurrentUser,
    service: MaybeWebhookService,
    Path(id): Path<String>,
    Json(body): Json<UpdateWebhookRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let webhook = service
        .update(
            user.id,
            &body.project_id,
            &id,
            WebhookPatch {
                name: body.name,
                description: body.description,
                events: body.events,
                active: body.active,
                archived: body.archived,
            },
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "webhook": WebhookDto::from(webhook) })),
    ))
}

/// `DELETE /app/webhooks/:id` — delete a webhook and its flow (the flow
/// never shows in `/flows`, so it is not released; archive first when
/// the flow should survive).
pub async fn delete_webhook(
    CurrentUser(user): CurrentUser,
    service: MaybeWebhookService,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let deleted = service.delete(user.id, project_id(&query)?, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "deleted": deleted })),
    ))
}
