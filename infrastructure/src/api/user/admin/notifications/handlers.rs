use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::super::auth::Problem;
use super::super::handlers::{CurrentAdmin, pagination, total_pages};
use super::dto::{AudienceRequest, BroadcastCreatedDto, BroadcastDto, BroadcastRequest};
use crate::domain::admin::notifications::{AdminNotificationsService, Audience, BroadcastInput};
use crate::domain::auth::entities::UserId;
use crate::domain::auth::errors::AuthError;
use crate::domain::notifications::entities::NotificationType;
use crate::ports::notifications_store::BroadcastStatus;

const DEFAULT_LIMIT: i64 = 25;

type MaybeNotificationsService = Option<Extension<Arc<AdminNotificationsService>>>;

fn require_notifications_service(
    extension: MaybeNotificationsService,
) -> Result<Arc<AdminNotificationsService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

fn parse_audience(request: &AudienceRequest) -> Result<Audience, Problem> {
    match request.kind.trim() {
        "all" => Ok(Audience::All),
        "users" => {
            if request.user_ids.is_empty() {
                return Err(AuthError::Validation(
                    "audience.users requires at least one user id".to_string(),
                )
                .into());
            }
            let ids = request
                .user_ids
                .iter()
                .map(|raw| {
                    UserId::from_str(raw).map_err(|_| {
                        Problem::from(AuthError::Validation(format!("invalid user id '{raw}'")))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Audience::Users(ids))
        }
        "projects" => {
            if request.project_ids.is_empty() {
                return Err(AuthError::Validation(
                    "audience.projects requires at least one project id".to_string(),
                )
                .into());
            }
            Ok(Audience::Projects(request.project_ids.clone()))
        }
        other => Err(AuthError::Validation(format!(
            "invalid audience kind '{other}' (expected all, users, or projects)"
        ))
        .into()),
    }
}

/// `POST /admin/notifications` — broadcast now, or schedule with `sendAt`.
pub async fn broadcast(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeNotificationsService,
    Json(body): Json<BroadcastRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_notifications_service(service)?;
    let notification_type =
        NotificationType::from_str(body.notification_type.trim()).map_err(|_| {
            Problem::from(AuthError::Validation(
                "invalid notification type".to_string(),
            ))
        })?;
    let result = service
        .broadcast(
            &admin,
            BroadcastInput {
                title: body.title,
                content: body.content,
                notification_type,
                channels: body.channels,
                audience: parse_audience(&body.audience)?,
                scheduled_for: body.scheduled_for,
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "broadcast": BroadcastCreatedDto::from(result) })),
    ))
}

/// `GET /admin/notifications` — broadcast history (newest first).
pub async fn list_broadcasts(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeNotificationsService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_notifications_service(service)?;
    let (page, per_page, offset) = pagination(&query, DEFAULT_LIMIT);

    let status = query
        .get("status")
        .map(|s| {
            BroadcastStatus::from_str(s.trim()).map_err(|_| {
                Problem::from(AuthError::Validation(
                    "invalid status (expected scheduled, sending, sent, or cancelled)".to_string(),
                ))
            })
        })
        .transpose()?;

    let (broadcasts, total) = service
        .history(
            status,
            query.get("type").map(String::as_str),
            per_page,
            offset,
        )
        .await?;

    let mut dtos = Vec::with_capacity(broadcasts.len());
    for broadcast in broadcasts {
        let (_, stats) = service
            .detail(&broadcast.id)
            .await?
            .ok_or_else(|| AuthError::NotFound("broadcast not found".into()))?;
        dtos.push(BroadcastDto::with_stats(broadcast, stats));
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "broadcasts": dtos,
            "page": page,
            "perPage": per_page,
            "total": total,
            "totalPages": total_pages(total, per_page),
        })),
    ))
}

/// `GET /admin/notifications/:id` — one broadcast with read stats.
pub async fn get_broadcast(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeNotificationsService,
    Path(broadcast_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_notifications_service(service)?;
    let (broadcast, stats) = service
        .detail(&broadcast_id)
        .await?
        .ok_or_else(|| AuthError::NotFound("broadcast not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "broadcast": BroadcastDto::with_stats(broadcast, stats) })),
    ))
}

/// `DELETE /admin/notifications/scheduled/:id` — cancel a scheduled broadcast.
pub async fn cancel_scheduled(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeNotificationsService,
    Path(broadcast_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_notifications_service(service)?;
    service.cancel_broadcast(&admin, &broadcast_id).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}
