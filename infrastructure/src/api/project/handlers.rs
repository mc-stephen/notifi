//! Product send endpoint: record-first notification delivery.
//!
//! v0 authenticates with the dashboard session cookie (external API keys
//! land in M6 with rate limits). Instant sends execute inline through the
//! project's default flow; the returned record carries the outcome.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;

use super::dto::{InboundFireRequest, SendDto, SendRequest};
use crate::api::app::auth::{CurrentUser, Problem};
use crate::domain::auth::errors::AuthError;
use crate::domain::inbound::{InboundPayload, InboundService};
use crate::domain::sends::{SendInput, SendService};

type MaybeSendService = Option<Extension<Arc<SendService>>>;

fn require_service(extension: MaybeSendService) -> Result<Arc<SendService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `POST /v1/notifications` — record the send, execute the default flow
/// inline, return the completed/failed record.
pub async fn send_notification(
    CurrentUser(user): CurrentUser,
    service: MaybeSendService,
    Json(body): Json<SendRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let send = service
        .send(
            user.id,
            &body.project_id,
            SendInput {
                channel: body.channel,
                recipients: body.recipients,
                title: body.title,
                body: body.body,
                send_at: body.send_at,
            },
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "notification": SendDto::from(send) })),
    ))
}

type MaybeInboundService = Option<Extension<Arc<InboundService>>>;

fn require_inbound(extension: MaybeInboundService) -> Result<Arc<InboundService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `POST /v1/hooks/{token}` — inbound webhook fire. The token in the URL
/// is the credential (no session): unknown tokens read as missing.
/// Caller fields win; the trigger node's config and template fill gaps.
pub async fn fire_webhook(
    service: MaybeInboundService,
    Path(token): Path<String>,
    Json(body): Json<InboundFireRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_inbound(service)?;
    let send = service
        .fire(
            &token,
            InboundPayload {
                channel: body.channel,
                recipients: body.recipients,
                title: body.title,
                body: body.body,
                template_id: body.template_id,
            },
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "notification": SendDto::from(send) })),
    ))
}
