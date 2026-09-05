use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use crate::domain::notifications::NotificationService;
use super::dto::{InAppNotificationDto, ListNotificationsQuery, SetReadRequest};
use super::super::auth::{CurrentUser, Problem};

const MAX_LIMIT: i64 = 200;

/// `GET /v1/notifications` — list in-app notifications (newest first).
pub async fn list_notifications(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<NotificationService>>,
    Query(query): Query<ListNotificationsQuery>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let limit = query.limit.clamp(1, MAX_LIMIT);

    let notifications = service
        .list(user.id, query.unread_only, limit + 1, query.before.as_deref())
        .await?;

    let has_more = notifications.len() > limit as usize;
    let dtos: Vec<InAppNotificationDto> = notifications
        .into_iter()
        .take(limit as usize)
        .map(InAppNotificationDto::from)
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "notifications": dtos, "hasMore": has_more })),
    ))
}

/// `GET /v1/notifications/count` — unread count for the bell badge.
pub async fn count_unread(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<NotificationService>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let count = service.count_unread(user.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "count": count })),
    ))
}

/// `GET /v1/notifications/:id` — one notification.
pub async fn get_notification(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<NotificationService>>,
    Path(notification_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let notification = service
        .get(user.id, &notification_id)
        .await?
        .ok_or_else(|| {
            crate::domain::auth::errors::AuthError::NotFound("notification not found".into())
        })?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "notification": InAppNotificationDto::from(notification) })),
    ))
}

/// `PATCH /v1/notifications/:id/read` — mark one notification read/unread.
pub async fn set_read(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<NotificationService>>,
    Path(notification_id): Path<String>,
    Json(body): Json<SetReadRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let notification = service
        .set_read(user.id, &notification_id, body.read)
        .await?
        .ok_or_else(|| {
            crate::domain::auth::errors::AuthError::NotFound("notification not found".into())
        })?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "notification": InAppNotificationDto::from(notification) })),
    ))
}

/// `PATCH /v1/notifications/read-all` — mark all as read.
pub async fn mark_all_read(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<NotificationService>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let count = service.mark_all_read(user.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "updated": count })),
    ))
}

/// `DELETE /v1/notifications/:id` — soft-delete a notification.
pub async fn delete_notification(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<NotificationService>>,
    Path(notification_id): Path<String>,
) -> Result<StatusCode, Problem> {
    let deleted = service.delete(user.id, &notification_id).await?;
    if !deleted {
        return Err(crate::domain::auth::errors::AuthError::NotFound("notification not found".into()).into());
    }
    Ok(StatusCode::NO_CONTENT)
}
