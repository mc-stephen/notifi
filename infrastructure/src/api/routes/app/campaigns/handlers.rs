//! Campaign CRUD plus lifecycle actions (`run-now`, `pause`, `resume`,
//! `cancel`). Campaigns are fully flow-less: delivery resolves per-row
//! against the project's connected providers for the row's channel.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{
    CampaignActionRequest, CampaignDto, CampaignStatsDto, CampaignWithStatsDto,
    CreateCampaignRequest, UpdateCampaignRequest,
};
use crate::domain::campaigns::{Campaign, CampaignService, entities::CampaignStatus};
use crate::domain::error::AppError;
use std::collections::HashMap;

type MaybeCampaignService = Option<Extension<Arc<CampaignService>>>;

fn require_service(extension: MaybeCampaignService) -> Result<Arc<CampaignService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AppError::NotConfigured.into())
}

fn project_id(query: &HashMap<String, String>) -> Result<&String, Problem> {
    query
        .get("project_id")
        .ok_or_else(|| Problem::from(AppError::Validation("project_id is required".to_string())))
}

/// Parses the wire audience mode; unknown values are a bad request.
fn parse_audience_mode(
    mode: Option<&str>,
) -> Result<crate::domain::campaigns::entities::AudienceMode, Problem> {
    match mode {
        None | Some("all") => Ok(crate::domain::campaigns::entities::AudienceMode::All),
        Some("custom") => Ok(crate::domain::campaigns::entities::AudienceMode::Custom),
        Some(other) => Err(AppError::Validation(format!(
            "unknown audience mode '{other}' — use 'all' or 'custom'"
        ))
        .into()),
    }
}

/// Wrap a campaign with derived stats; unexpanded campaigns also carry a
/// fire-time audience estimate.
async fn with_stats(
    service: &CampaignService,
    campaign: Campaign,
) -> Result<CampaignWithStatsDto, Problem> {
    let stats = service.stats_for(&campaign.id).await?;
    let estimated_audience = match campaign.status {
        CampaignStatus::Draft | CampaignStatus::Scheduled => {
            Some(service.estimate_audience(&campaign).await?)
        }
        _ => None,
    };
    Ok(CampaignWithStatsDto {
        campaign: CampaignDto::from(campaign),
        totals: CampaignStatsDto {
            total: stats.total,
            delivered: stats.delivered,
            failed: stats.failed,
            pending: stats.pending,
        },
        estimated_audience,
    })
}

/// `GET /app/campaigns?project_id=` — all campaigns, oldest first.
pub async fn list_campaigns(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaigns = service.list(user.id, project_id(&query)?).await?;
    let mut dtos = Vec::with_capacity(campaigns.len());
    for campaign in campaigns {
        dtos.push(with_stats(&service, campaign).await?);
    }
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "campaigns": dtos })),
    ))
}

/// `GET /app/campaigns/:id?project_id=` — one campaign with its stats.
pub async fn get_campaign(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaign = service.get(user.id, project_id(&query)?, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "campaign": with_stats(&service, campaign).await? })),
    ))
}

/// `POST /app/campaigns` — compose a campaign (draft, or scheduled when
/// `sendAt` rides along).
pub async fn create_campaign(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Json(body): Json<CreateCampaignRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaign = service
        .create(
            user.id,
            &body.project_id,
            crate::domain::campaigns::CampaignInput {
                name: body.name,
                description: body.description,
                template_id: body.template_id.unwrap_or_default(),
                channel: body.channel.unwrap_or_default(),
                provider_id: body.provider_id,
                audience_mode: parse_audience_mode(body.audience_mode.as_deref())?,
                selected_user_ids: body.selected_user_ids.unwrap_or_default(),
                send_at: body.send_at,
            },
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "campaign": with_stats(&service, campaign).await? })),
    ))
}

/// `PATCH /app/campaigns/:id` — edit a draft/scheduled campaign.
/// `templateId`/`channel` re-render the snapshot; `sendAt: null`
/// unschedules; the audience may be re-scoped.
pub async fn update_campaign(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Path(id): Path<String>,
    Json(body): Json<UpdateCampaignRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaign = service
        .update(
            user.id,
            &body.project_id,
            &id,
            crate::domain::campaigns::CampaignPatch {
                name: body.name,
                // Patch semantics: absent keeps; an explicit (possibly
                // empty) string re-trims into the field.
                description: body.description.map(Some),
                template_id: body.template_id,
                channel: body.channel,
                provider_id: body.provider_id,
                audience_mode: Some(parse_audience_mode(body.audience_mode.as_deref())?),
                selected_user_ids: body.selected_user_ids,
                send_at: body.send_at,
            },
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "campaign": with_stats(&service, campaign).await? })),
    ))
}

/// `DELETE /app/campaigns/:id` — delete (running/paused refuse: settle
/// them first). Expanded send rows keep their delivery history.
pub async fn delete_campaign(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
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

/// `POST /app/campaigns/:id/run-now` — fire now (expands + first batch).
pub async fn run_campaign_now(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Path(id): Path<String>,
    Json(body): Json<CampaignActionRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaign = service.run_now(user.id, &body.project_id, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "campaign": with_stats(&service, campaign).await? })),
    ))
}

/// `POST /app/campaigns/:id/pause` — stop draining (pending rows wait).
pub async fn pause_campaign(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Path(id): Path<String>,
    Json(body): Json<CampaignActionRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaign = service.pause(user.id, &body.project_id, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "campaign": with_stats(&service, campaign).await? })),
    ))
}

/// `POST /app/campaigns/:id/resume` — the sweep picks the campaign back up.
pub async fn resume_campaign(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Path(id): Path<String>,
    Json(body): Json<CampaignActionRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaign = service.resume(user.id, &body.project_id, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "campaign": with_stats(&service, campaign).await? })),
    ))
}

/// `POST /app/campaigns/:id/cancel` — drop pending rows, mark terminal.
pub async fn cancel_campaign(
    CurrentUser(user): CurrentUser,
    service: MaybeCampaignService,
    Path(id): Path<String>,
    Json(body): Json<CampaignActionRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let campaign = service.cancel(user.id, &body.project_id, &id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "campaign": with_stats(&service, campaign).await? })),
    ))
}
