use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::super::auth::Problem;
use super::super::handlers::CurrentAdmin;
use super::dto::{AdminPlanDto, CreatePlanRequest, PlanSubscriberDto, UpdatePlanRequest};
use crate::domain::auth::errors::AuthError;
use crate::domain::billing::BillingService;

type MaybeBillingService = Option<Extension<Arc<BillingService>>>;

fn require_service(extension: MaybeBillingService) -> Result<Arc<BillingService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `GET /admin/billing/plans` — every plan with live subscriber counts.
pub async fn list_plans(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeBillingService,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let plans = service.list_all_plans().await?;
    let mut dtos = Vec::with_capacity(plans.len());
    for plan in plans {
        let count = service.subscriber_count(&plan.id).await?;
        dtos.push(AdminPlanDto::with_count(plan, count));
    }
    Ok((StatusCode::OK, Json(serde_json::json!({ "plans": dtos }))))
}

/// `POST /admin/billing/plans` — create a plan.
pub async fn create_plan(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeBillingService,
    Json(body): Json<CreatePlanRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let plan = service.create_plan(&admin, body.into()).await?;
    let count = service.subscriber_count(&plan.id).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "plan": AdminPlanDto::with_count(plan, count) })),
    ))
}

/// `PATCH /admin/billing/plans/{id}` — partial update (price, caps,
/// discount, or archive via `isActive: false`).
pub async fn update_plan(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeBillingService,
    Path(plan_id): Path<String>,
    Json(body): Json<UpdatePlanRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let stored = service
        .get_plan(&plan_id)
        .await?
        .ok_or_else(|| AuthError::NotFound("plan not found".into()))?;
    let plan = service
        .update_plan(&admin, &plan_id, body.apply_to(&stored))
        .await?;
    let count = service.subscriber_count(&plan.id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "plan": AdminPlanDto::with_count(plan, count) })),
    ))
}

/// `DELETE /admin/billing/plans/{id}` — hard delete. Refused (409)
/// while subscribers remain; archive instead.
pub async fn delete_plan(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeBillingService,
    Path(plan_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    service.delete_plan(&admin, &plan_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "status": "deleted" })),
    ))
}

/// `GET /admin/billing/plans/{id}/subscriptions` — who holds this plan.
pub async fn list_subscribers(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeBillingService,
    Path(plan_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let subscribers = service.list_subscribers(&plan_id).await?;
    let dtos: Vec<PlanSubscriberDto> = subscribers
        .into_iter()
        .map(PlanSubscriberDto::from)
        .collect();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "subscriptions": dtos })),
    ))
}

/// `GET /admin/billing/subscribers/history?months=12` — trailing
/// monthly active-subscriber snapshots, replayed from project births
/// + billing audit events (no ledger table).
pub async fn subscription_history(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeBillingService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let months = query
        .get("months")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(12);
    let points = service.subscription_history(months).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "points": points })),
    ))
}
