//! Product send endpoint: record-first notification delivery.
//!
//! v0 authenticates with the dashboard session cookie (external API keys
//! land in M6 with rate limits). Instant sends execute inline through the
//! project's default flow; the returned record carries the outcome.

use std::sync::Arc;

use axum::Json;
use axum::extract::{ConnectInfo, Extension, Path};
use axum::http::{HeaderMap, StatusCode};

use super::dto::{InboundFireRequest, SendDto, SendRequest};
use crate::api::routes::app::auth::{CurrentUser, Problem};
use crate::domain::error::AppError;
use crate::domain::inbound::{InboundPayload, InboundService};
use crate::domain::sends::{SendInput, SendService};

type MaybeSendService = Option<Extension<Arc<SendService>>>;

fn require_service(extension: MaybeSendService) -> Result<Arc<SendService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AppError::NotConfigured.into())
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
        .ok_or_else(|| AppError::NotConfigured.into())
}

/// `POST /v1/hooks/{token}` — inbound webhook fire. The token in the URL
/// is the credential (no session): unknown tokens read as missing.
/// Caller fields win; the trigger node's config and template fill gaps.
/// The node can also demand door auth: a matching `x-notifi-secret`
/// header (401 when wrong/missing) and caller IP inside its allow-list
/// (403) — proof in `webhook_fire_secret_*` / `webhook_fire_ip_*`
/// integration tests, open door otherwise. Authorized pings are
/// recorded (best-effort) as the door's last-ping probe — the
/// builder's "Incoming data" tab reads it back.
pub async fn fire_webhook(
    service: MaybeInboundService,
    Path(token): Path<String>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<InboundFireRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_inbound(service)?;
    // The probe captures what an authorized caller actually sent: caller
    // IP, whitelisted headers, and the parsed body re-serialized (the
    // request dto is Deserialize-only, so rebuild the JSON explicitly).
    // The service records it after the door's checks pass (best-effort).
    let header_str = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    };
    let body_json = serde_json::json!({
        "channel": body.channel,
        "recipients": body.recipients,
        "title": body.title,
        "body": body.body,
        "templateId": body.template_id,
    });
    let send = service
        .fire(
            &token,
            header_str("x-notifi-secret"),
            Some(peer.ip()),
            InboundPayload {
                channel: body.channel,
                recipients: body.recipients,
                title: body.title,
                body: body.body,
                template_id: body.template_id,
            },
            crate::domain::inbound::InboundProbe {
                client_ip: Some(peer.ip().to_string()),
                content_type: header_str("content-type"),
                user_agent: header_str("user-agent"),
                forwarded_for: header_str("x-forwarded-for"),
                body: crate::domain::inbound::entities::cap_probe_body(
                    serde_json::to_string(&body_json).ok(),
                ),
            },
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "notification": SendDto::from(send) })),
    ))
}
