//! Project flow CRUD. Sends execute every flow carrying a
//! `notification.created` trigger (oldest first); no stored default flag.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{CreateFlowRequest, FlowDto, UpdateFlowRequest};
use crate::domain::auth::errors::AuthError;
use crate::domain::flows::FlowService;
use crate::domain::inbound::InboundService;
use crate::domain::schedules::ScheduleService;
use std::collections::HashMap;

type MaybeFlowService = Option<Extension<Arc<FlowService>>>;
type MaybeScheduleService = Option<Extension<Arc<ScheduleService>>>;
type MaybeInboundService = Option<Extension<Arc<InboundService>>>;

fn require_service(extension: MaybeFlowService) -> Result<Arc<FlowService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

fn project_id(query: &HashMap<String, String>) -> Result<&String, Problem> {
    query
        .get("project_id")
        .ok_or_else(|| Problem::from(AuthError::Validation("project_id is required".to_string())))
}

/// `GET /app/flows?project_id=` — all flows, oldest first, excluding
/// flows assigned to a schedule (those live on the Schedules page).
/// The schedule service is optional: without it (or with none wired)
/// nothing is hidden.
pub async fn list_flows(
    CurrentUser(user): CurrentUser,
    service: MaybeFlowService,
    schedules: MaybeScheduleService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let project = project_id(&query)?;
    let flows = service.list(user.id, project).await?;
    let hidden = match schedules {
        Some(Extension(schedules)) => schedules
            .scheduled_flow_ids(user.id, project)
            .await
            .unwrap_or_default(),
        None => Default::default(),
    };
    let dtos: Vec<FlowDto> = flows
        .into_iter()
        .filter(|f| !hidden.contains(&f.id))
        .map(FlowDto::from)
        .collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "flows": dtos }))))
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
