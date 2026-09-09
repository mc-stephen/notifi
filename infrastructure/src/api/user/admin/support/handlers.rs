use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use crate::api::user::support::dto::{SendReplyRequest, TicketMessageDto};
use crate::domain::auth::errors::AuthError;
use crate::domain::support::TicketService;
use super::super::handlers::CurrentAdmin;
use super::super::super::auth::Problem;
use super::dto::{AdminTicketDto, AdminTicketMessageDto, SetStatusRequest};

const DEFAULT_LIMIT: i64 = 100;

type MaybeTicketService = Option<Extension<Arc<TicketService>>>;

fn require_tickets_service(extension: MaybeTicketService) -> Result<Arc<TicketService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `GET /admin/support/tickets` — list all tickets (admin view).
pub async fn list_tickets(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeTicketService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_tickets_service(service)?;
    let (page, per_page, offset) = super::super::handlers::pagination(&query, DEFAULT_LIMIT);

    let (tickets, total) = service
        .list_all_tickets(
            query.get("status").map(String::as_str),
            query.get("search").map(String::as_str),
            per_page,
            offset,
        )
        .await?;

    let dtos: Vec<AdminTicketDto> = tickets.into_iter().map(AdminTicketDto::from).collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "tickets": dtos,
            "page": page,
            "perPage": per_page,
            "total": total,
            "totalPages": super::super::handlers::total_pages(total, per_page),
        })),
    ))
}

/// `GET /admin/support/tickets/:id` — one ticket (admin view).
pub async fn get_ticket(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeTicketService,
    Path(ticket_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_tickets_service(service)?;
    let ticket = service
        .get_any_ticket(&ticket_id)
        .await?
        .ok_or_else(|| AuthError::NotFound("ticket not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "ticket": AdminTicketDto::from(ticket) })),
    ))
}

/// `GET /admin/support/tickets/:id/messages` — thread (admin view).
pub async fn list_messages(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeTicketService,
    Path(ticket_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_tickets_service(service)?;
    // 404 when the ticket does not exist (mirrors the customer surface).
    service
        .get_any_ticket(&ticket_id)
        .await?
        .ok_or_else(|| AuthError::NotFound("ticket not found".into()))?;
    let messages = service.list_any_messages(&ticket_id).await?;
    let dtos: Vec<AdminTicketMessageDto> =
        messages.into_iter().map(AdminTicketMessageDto::from).collect();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "messages": dtos })),
    ))
}

/// `POST /admin/support/tickets/:id/messages` — admin reply as `support`.
pub async fn send_reply(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeTicketService,
    Path(ticket_id): Path<String>,
    Json(body): Json<SendReplyRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_tickets_service(service)?;
    let message = service.add_admin_reply(&admin, &ticket_id, &body.body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "message": TicketMessageDto::from(message) })),
    ))
}

/// `PATCH /admin/support/tickets/:id` — set ticket status.
pub async fn set_status(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeTicketService,
    Path(ticket_id): Path<String>,
    Json(body): Json<SetStatusRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_tickets_service(service)?;
    let ticket = service
        .set_ticket_status(&admin, &ticket_id, &body.status)
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "ticket": AdminTicketDto::from(ticket) })),
    ))
}
