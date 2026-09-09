use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use crate::domain::admin::projects::AdminProjectsService;
use crate::domain::auth::errors::AuthError;
use super::super::handlers::CurrentAdmin;
use super::super::super::auth::Problem;
use super::dto::{AdminProjectDetailDto, AdminProjectDto};

const DEFAULT_LIMIT: i64 = 50;

type MaybeProjectsService = Option<Extension<Arc<AdminProjectsService>>>;

fn require_projects_service(
    extension: MaybeProjectsService,
) -> Result<Arc<AdminProjectsService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `GET /admin/projects` — list all platform projects (newest first).
pub async fn list_projects(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeProjectsService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_projects_service(service)?;
    let (page, per_page, offset) = super::super::handlers::pagination(&query, DEFAULT_LIMIT);

    let (projects, total) = service
        .list_projects(
            query.get("search").map(String::as_str),
            query.get("environment").map(String::as_str),
            per_page,
            offset,
        )
        .await?;

    let dtos: Vec<AdminProjectDto> = projects.into_iter().map(AdminProjectDto::from).collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "projects": dtos,
            "page": page,
            "perPage": per_page,
            "total": total,
            "totalPages": super::super::handlers::total_pages(total, per_page),
        })),
    ))
}

/// `GET /admin/projects/:id` — one project with owner + members.
pub async fn get_project(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeProjectsService,
    Path(project_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_projects_service(service)?;
    let detail = service
        .get_project_detail(&project_id)
        .await?
        .ok_or_else(|| AuthError::NotFound("project not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "project": AdminProjectDetailDto::from(detail) })),
    ))
}
