//! Schedule CRUD plus manual fire. Assigned flows vanish from the
//! `/flows` list (managed here); the builder stays reachable by id.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{CreateScheduleRequest, RunNowRequest, ScheduleDto, UpdateScheduleRequest};
use crate::domain::auth::errors::AuthError;
use crate::domain::schedules::entities::{ScheduleInput, ScheduleKind};
use crate::domain::schedules::services::{SchedulePatch, ScheduleService};
use std::collections::HashMap;

type MaybeScheduleService = Option<Extension<Arc<ScheduleService>>>;

fn require_service(extension: MaybeScheduleService) -> Result<Arc<ScheduleService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

fn project_id(query: &HashMap<String, String>) -> Result<&String, Problem> {
    query
        .get("project_id")
        .ok_or_else(|| Problem::from(AuthError::Validation("project_id is required".to_string())))
}

fn parse_kind(kind: &str) -> Result<ScheduleKind, Problem> {
    ScheduleKind::parse(kind).ok_or_else(|| {
        Problem::from(AuthError::Validation(
            "kind must be 'once', 'recurring', or 'weekly'".to_string(),
        ))
    })
}

/// `GET /app/schedules?project_id=` — all schedules, oldest first.
pub async fn list_schedules(
    CurrentUser(user): CurrentUser,
    service: MaybeScheduleService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let schedules = service.list(user.id, project_id(&query)?).await?;
    let dtos: Vec<ScheduleDto> = schedules.into_iter().map(ScheduleDto::from).collect();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "schedules": dtos })),
    ))
}

/// `GET /app/schedules/:id?project_id=` — one schedule with its payload
/// (the table hides channel/recipients; the editor needs them).
pub async fn get_schedule(
    CurrentUser(user): CurrentUser,
    service: MaybeScheduleService,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let schedule = service.get(user.id, project_id(&query)?, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "schedule": ScheduleDto::from(schedule) })),
    ))
}

/// `POST /app/schedules` — attach a flow to a timing. The flow leaves the
/// `/flows` list from this point on.
pub async fn create_schedule(
    CurrentUser(user): CurrentUser,
    service: MaybeScheduleService,
    Json(body): Json<CreateScheduleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let schedule = service
        .create(
            user.id,
            &body.project_id,
            ScheduleInput {
                name: body.name,
                flow_id: body.flow_id,
                kind: parse_kind(&body.kind)?,
                run_at: body.run_at,
                interval_secs: body.interval_secs,
                days: body.days.unwrap_or_default(),
                run_time: body.run_time,
                timezone: body.timezone.unwrap_or_else(|| "UTC".to_string()),
                template_id: body.template_id,
                channel: body.channel.unwrap_or_default(),
                title: body.title.unwrap_or_default(),
                body: body.body.unwrap_or_default(),
                recipients: body.recipients,
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "schedule": ScheduleDto::from(schedule) })),
    ))
}

/// `PATCH /app/schedules/:id` — edit name, flow, timing, payload, or
/// pause/resume. Null `run_at`/`interval_secs` clears the field (kind
/// switches).
pub async fn update_schedule(
    CurrentUser(user): CurrentUser,
    service: MaybeScheduleService,
    Path(id): Path<String>,
    Json(body): Json<UpdateScheduleRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let kind = body.kind.as_deref().map(parse_kind).transpose()?;
    let schedule = service
        .update(
            user.id,
            &body.project_id,
            &id,
            SchedulePatch {
                name: body.name,
                flow_id: body.flow_id,
                kind,
                run_at: body.run_at,
                interval_secs: body.interval_secs,
                days: body.days,
                run_time: body.run_time,
                timezone: body.timezone,
                template_id: body.template_id,
                channel: body.channel,
                title: body.title,
                body: body.body,
                recipients: body.recipients,
                active: body.active,
            },
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "schedule": ScheduleDto::from(schedule) })),
    ))
}

/// `DELETE /app/schedules/:id` — delete a schedule. The assigned flow
/// automatically reappears in the `/flows` list.
pub async fn delete_schedule(
    CurrentUser(user): CurrentUser,
    service: MaybeScheduleService,
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

/// `POST /app/schedules/:id/run-now` — fire the assigned flow immediately
/// with the stored payload. Timing is untouched.
pub async fn run_schedule_now(
    CurrentUser(user): CurrentUser,
    service: MaybeScheduleService,
    Path(id): Path<String>,
    Json(body): Json<RunNowRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let schedule = service.run_now(user.id, &body.project_id, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "schedule": ScheduleDto::from(schedule) })),
    ))
}
