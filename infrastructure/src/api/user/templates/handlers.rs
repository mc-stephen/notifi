//! Template HTTP handlers.
//!
//! The service is injected as a required axum Extension by the composition
//! root; the route is absent when the service is unavailable.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use crate::domain::auth::errors::AuthError;
use crate::domain::templates::TemplateService;
use crate::ports::templates_store::AttachmentInput;

use super::dto::{
    AttachmentInputDto, CreateTemplateRequest, TemplateDto, UpdateTemplateRequest,
};
use super::super::auth::{CurrentUser, Problem};

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;

fn to_attachment_inputs(dtos: Vec<AttachmentInputDto>) -> Vec<AttachmentInput> {
    dtos.into_iter()
        .map(|a| AttachmentInput {
            name: a.name,
            mime_type: a.mime_type,
            size_bytes: a.size_bytes,
            url: a.url,
        })
        .collect()
}

/// `POST /v1/projects/:project_id/templates` — create a template.
pub async fn create_template(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TemplateService>>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let content = body.content.unwrap_or(serde_json::Value::Null);
    let template = service
        .create(
            user.id,
            &project_id,
            &body.name,
            body.description.as_deref(),
            &body.channel,
            content,
            to_attachment_inputs(body.attachments),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "template": TemplateDto::from(template) })),
    ))
}

/// `GET /v1/projects/:project_id/templates` — list, newest first.
pub async fn list_templates(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TemplateService>>,
    Path(project_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let limit = query
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);

    let templates = service
        .list(
            user.id,
            &project_id,
            query.get("search").map(String::as_str),
            limit + 1,
            query.get("before").map(String::as_str),
        )
        .await?;

    let has_more = templates.len() > limit as usize;
    let dtos: Vec<TemplateDto> = templates
        .into_iter()
        .take(limit as usize)
        .map(TemplateDto::from)
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "templates": dtos, "hasMore": has_more })),
    ))
}

/// `GET /v1/projects/:project_id/templates/:id` — one template with attachments.
pub async fn get_template(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TemplateService>>,
    Path((project_id, template_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let template = service
        .get(user.id, &project_id, &template_id)
        .await?
        .ok_or_else(|| AuthError::NotFound("template not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "template": TemplateDto::from(template) })),
    ))
}

/// `PATCH /v1/projects/:project_id/templates/:id` — update content/attachments.
pub async fn update_template(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TemplateService>>,
    Path((project_id, template_id)): Path<(String, String)>,
    Json(body): Json<UpdateTemplateRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let content = body.content.unwrap_or(serde_json::Value::Null);
    let attachments = body.attachments.map(to_attachment_inputs);
    let template = service
        .update(
            user.id,
            &project_id,
            &template_id,
            &body.name,
            body.description.as_deref(),
            &body.channel,
            content,
            attachments,
        )
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "template": TemplateDto::from(template) })),
    ))
}

/// `DELETE /v1/projects/:project_id/templates/:id` — soft delete.
pub async fn delete_template(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<TemplateService>>,
    Path((project_id, template_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    service.remove(user.id, &project_id, &template_id).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}
