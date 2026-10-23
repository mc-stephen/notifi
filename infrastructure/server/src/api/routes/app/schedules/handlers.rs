//! Schedule CRUD plus manual fire. Owner doctrine: creating a schedule
//! creates its flow (named after it, trigger seeded); deleting the
//! schedule deletes that flow. Owned flows stay off the `/flows` list;
//! the builder stays reachable by id. Archive (PATCH `archived`) is the
//! non-destructive alternative.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{CreateScheduleRequest, RunNowRequest, ScheduleDto, UpdateScheduleRequest};
use crate::domain::error::AppError;
use crate::domain::schedules::entities::{ScheduleInput, ScheduleKind};
use crate::domain::schedules::services::{SchedulePatch, ScheduleService};
use std::collections::HashMap;

type MaybeScheduleService = Option<Extension<Arc<ScheduleService>>>;

fn require_service(extension: MaybeScheduleService) -> Result<Arc<ScheduleService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AppError::NotConfigured.into())
}

fn project_id(query: &HashMap<String, String>) -> Result<&String, Problem> {
    query
        .get("project_id")
        .ok_or_else(|| Problem::from(AppError::Validation("project_id is required".to_string())))
}

fn parse_kind(kind: &str) -> Result<ScheduleKind, Problem> {
    ScheduleKind::parse(kind).ok_or_else(|| {
        Problem::from(AppError::Validation(
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

/// `POST /app/schedules` — create a schedule and its flow (the flow is
/// born here, named after the schedule; never assigned).
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
                kind: parse_kind(&body.kind)?,
                run_at: body.run_at,
                interval_secs: body.interval_secs,
                days: body.days.unwrap_or_default(),
                run_time: body.run_time,
                timezone: body.timezone.unwrap_or_else(|| "UTC".to_string()),
                channel: body.channel.unwrap_or_default(),
                title: body.title.unwrap_or_default(),
                body: body.body.unwrap_or_default(),
                recipients: body.recipients.unwrap_or_default(),
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "schedule": ScheduleDto::from(schedule) })),
    ))
}

/// `PATCH /app/schedules/:id` — edit name, timing, payload, pause/resume,
/// or archive/restore (renames sync down to the owned flow). Null
/// `run_at`/`interval_secs` clears the field (kind switches).
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
                kind,
                run_at: body.run_at,
                interval_secs: body.interval_secs,
                days: body.days,
                run_time: body.run_time,
                timezone: body.timezone,
                channel: body.channel,
                title: body.title,
                body: body.body,
                recipients: body.recipients,
                active: body.active,
                archived: body.archived,
            },
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "schedule": ScheduleDto::from(schedule) })),
    ))
}

/// `DELETE /app/schedules/:id` — delete a schedule and its flow (the
/// flow never shows in `/flows`, so it is not released; archive first
/// when the flow should survive).
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
