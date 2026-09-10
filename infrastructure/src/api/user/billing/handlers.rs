use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{CancelRequest, PlanDto, SubscribeRequest, SubscriptionDto};
use crate::domain::auth::errors::AuthError;
use crate::domain::billing::BillingService;

type MaybeBillingService = Option<Extension<Arc<BillingService>>>;

fn require_service(extension: MaybeBillingService) -> Result<Arc<BillingService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `GET /app/billing/plans` — active plans for the pricing grid.
pub async fn list_plans(
    CurrentUser(_user): CurrentUser,
    service: MaybeBillingService,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let plans = service.list_plans().await?;
    let dtos: Vec<PlanDto> = plans.into_iter().map(PlanDto::from).collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "plans": dtos }))))
}

/// `GET /app/billing/subscription?project_id=` — the project's
/// subscription, creating the free one on first read.
pub async fn get_subscription(
    CurrentUser(user): CurrentUser,
    service: MaybeBillingService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let project_id = query.get("project_id").ok_or_else(|| {
        Problem::from(AuthError::Validation("project_id is required".to_string()))
    })?;
    let subscription = service.get_subscription(user.id, project_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "subscription": SubscriptionDto::from(subscription) })),
    ))
}

/// `POST /app/billing/subscriptions` — subscribe or change plan.
/// Record-only in v1 (no payment provider yet).
pub async fn subscribe(
    CurrentUser(user): CurrentUser,
    service: MaybeBillingService,
    Json(body): Json<SubscribeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let subscription = service
        .subscribe(
            user.id,
            &body.project_id,
            &body.plan_id,
            body.billing_cycle.as_deref().unwrap_or("monthly"),
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "subscription": SubscriptionDto::from(subscription) })),
    ))
}

/// `POST /app/billing/subscriptions/cancel` — cancel, keeping history.
pub async fn cancel(
    CurrentUser(user): CurrentUser,
    service: MaybeBillingService,
    Json(body): Json<CancelRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let subscription = service.cancel(user.id, &body.project_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "subscription": SubscriptionDto::from(subscription) })),
    ))
}

/// `POST /app/billing/subscriptions/renew` — restart the current period.
/// Refused (422) when the plan was retired.
pub async fn renew(
    CurrentUser(user): CurrentUser,
    service: MaybeBillingService,
    Json(body): Json<CancelRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let subscription = service.renew(user.id, &body.project_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "subscription": SubscriptionDto::from(subscription) })),
    ))
}
