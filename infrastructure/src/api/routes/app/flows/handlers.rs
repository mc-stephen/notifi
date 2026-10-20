//! Project flow CRUD. Owner-managed flows (schedules/webhooks) never
//! list here; sends execute every flow carrying a `notification.created`
//! trigger (oldest first); no stored default flag.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{CreateFlowRequest, FlowDto, SOURCE_SCHEDULE, SOURCE_WEBHOOK, UpdateFlowRequest};
use crate::domain::error::AppError;
use crate::domain::flows::FlowService;
use crate::domain::inbound::InboundService;
use crate::domain::schedules::ScheduleService;
use crate::domain::webhooks::WebhookService;

type MaybeFlowService = Option<Extension<Arc<FlowService>>>;
type MaybeScheduleService = Option<Extension<Arc<ScheduleService>>>;
type MaybeInboundService = Option<Extension<Arc<InboundService>>>;
type MaybeWebhookService = Option<Extension<Arc<WebhookService>>>;

fn require_service(extension: MaybeFlowService) -> Result<Arc<FlowService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AppError::NotConfigured.into())
}

fn project_id(query: &HashMap<String, String>) -> Result<&String, Problem> {
    query
        .get("project_id")
        .ok_or_else(|| Problem::from(AppError::Validation("project_id is required".to_string())))
}

/// The ids hidden from the flows list by an owner. Both owner services
/// are optional: without one nothing from it is hidden.
async fn hidden_flow_ids(
    schedules: &MaybeScheduleService,
    webhooks: &MaybeWebhookService,
    user: crate::domain::auth::entities::UserId,
    project: &str,
) -> std::collections::HashSet<String> {
    let mut hidden = std::collections::HashSet::new();
    if let Some(Extension(schedules)) = schedules {
        hidden.extend(
            schedules
                .scheduled_flow_ids(user, project)
                .await
                .unwrap_or_default(),
        );
    }
    if let Some(Extension(webhooks)) = webhooks {
        hidden.extend(
            webhooks
                .owned_flow_ids(user, project)
                .await
                .unwrap_or_default(),
        );
    }
    hidden
}

/// `GET /app/flows?project_id=` — flows created on the flows page,
/// oldest first. Owner-created flows (schedules/webhooks) are excluded:
/// they live on their owner's page.
pub async fn list_flows(
    CurrentUser(user): CurrentUser,
    service: MaybeFlowService,
    schedules: MaybeScheduleService,
    webhooks: MaybeWebhookService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let project = project_id(&query)?.to_string();
    let flows = service.list(user.id, &project).await?;
    let hidden = hidden_flow_ids(&schedules, &webhooks, user.id, &project).await;
    let dtos: Vec<FlowDto> = flows
        .into_iter()
        .filter(|f| !hidden.contains(&f.id))
        .map(FlowDto::from)
        .collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "flows": dtos }))))
}

/// `GET /app/flows/:id?project_id=` — one flow, even when an owner
/// (schedule/webhook) hides it from the list; the builder loads by id.
/// The response tags `source` so the builder scopes the trigger UI to
/// the owner.
pub async fn get_flow(
    CurrentUser(user): CurrentUser,
    service: MaybeFlowService,
    schedules: MaybeScheduleService,
    webhooks: MaybeWebhookService,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let project = project_id(&query)?;
    let flow = service.get(user.id, project, &id).await?;
    let mut dto = FlowDto::from(flow);
    if let Some(Extension(schedules)) = &schedules
        && schedules
            .scheduled_flow_ids(user.id, project)
            .await
            .unwrap_or_default()
            .contains(&dto.id)
    {
        dto.source = SOURCE_SCHEDULE;
    }
    if let Some(Extension(webhooks)) = &webhooks
        && webhooks
            .owned_flow_ids(user.id, project)
            .await
            .unwrap_or_default()
            .contains(&dto.id)
    {
        dto.source = SOURCE_WEBHOOK;
    }
    Ok((StatusCode::OK, Json(serde_json::json!({ "flow": dto }))))
}

/// `POST /app/flows` — create a flow. Omitted definition starts empty
/// (the builder's validator asks for a trigger). Webhook
/// trigger tokens sync best-effort (a failed sync retries on next save).
pub async fn create_flow(
    CurrentUser(user): CurrentUser,
    service: MaybeFlowService,
    inbound: MaybeInboundService,
    Json(body): Json<CreateFlowRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let flow = service
        .create(
            user.id,
            &body.project_id,
            body.name.as_deref().unwrap_or(""),
            body.definition,
        )
        .await?;
    if let Some(Extension(inbound)) = inbound {
        inbound
            .sync_flow_tokens(&body.project_id, &flow.id, &flow.definition)
            .await;
    }
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "flow": FlowDto::from(flow) })),
    ))
}

/// `PUT /app/flows/:id` — replace name/definition (builder save path).
/// Webhook trigger tokens sync best-effort (a failed sync retries).
pub async fn update_flow(
    CurrentUser(user): CurrentUser,
    service: MaybeFlowService,
    inbound: MaybeInboundService,
    Path(id): Path<String>,
    Json(body): Json<UpdateFlowRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let flow = service
        .update(user.id, &body.project_id, &id, &body.name, &body.definition)
        .await?;
    if let Some(Extension(inbound)) = inbound {
        inbound
            .sync_flow_tokens(&body.project_id, &flow.id, &flow.definition)
            .await;
    }
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "flow": FlowDto::from(flow) })),
    ))
}

/// `DELETE /app/flows/:id` — delete a flow. Idempotent; sends fail closed
/// (409) once no qualifying flow remains.
pub async fn delete_flow(
    CurrentUser(user): CurrentUser,
    service: MaybeFlowService,
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
