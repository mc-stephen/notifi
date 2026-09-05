use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use crate::domain::support::TicketService;
use super::dto::{CreateTicketRequest, SendReplyRequest, TicketDto, TicketMessageDto};
use super::super::auth::{CurrentUser, Problem};

const DEFAULT_LIMIT: i64 = 100;
const MAX_LIMIT: i64 = 200;

/// `POST /v1/support/tickets` — create a support ticket.
pub async fn create_ticket(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TicketService>>,
    Json(body): Json<CreateTicketRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let ticket = service
        .create(
            user.id,
            body.project_id.as_deref(),
            &body.subject,
            &body.category,
            &body.priority,
            &body.description,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "ticket": TicketDto::from(ticket) })),
    ))
}

/// `GET /v1/support/tickets` — list tickets visible to the caller.
pub async fn list_tickets(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TicketService>>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let limit = query
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);

    let tickets = service
        .list(
            user.id,
            query.get("project_id").map(String::as_str),
            query.get("status").map(String::as_str),
            limit + 1,
            query.get("before").map(String::as_str),
        )
        .await?;

    let has_more = tickets.len() > limit as usize;
    let dtos: Vec<TicketDto> = tickets
        .into_iter()
        .take(limit as usize)
        .map(TicketDto::from)
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "tickets": dtos, "hasMore": has_more })),
    ))
}

/// `GET /v1/support/tickets/:id` — one ticket.
pub async fn get_ticket(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TicketService>>,
    Path(ticket_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let ticket = service
        .get(user.id, &ticket_id)
        .await?
        .ok_or_else(|| crate::domain::auth::errors::AuthError::NotFound("ticket not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "ticket": TicketDto::from(ticket) })),
    ))
}

/// `GET /v1/support/tickets/:id/messages` — conversation thread for a ticket.
pub async fn list_messages(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TicketService>>,
    Path(ticket_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let messages = service.list_messages(user.id, &ticket_id).await?;
    let dtos: Vec<TicketMessageDto> = messages.into_iter().map(TicketMessageDto::from).collect();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "messages": dtos })),
    ))
}

/// `POST /v1/support/tickets/:id/messages` — send a reply.
pub async fn send_reply(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TicketService>>,
    Path(ticket_id): Path<String>,
    Json(body): Json<SendReplyRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let message = service.add_reply(user.id, &ticket_id, &body.body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "message": TicketMessageDto::from(message) })),
    ))
}
