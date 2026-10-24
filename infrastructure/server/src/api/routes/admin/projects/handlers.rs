use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use super::super::handlers::CurrentAdmin;
use super::dto::{AdminProjectDetailDto, AdminProjectDto};
use crate::api::routes::app::auth::Problem;
use crate::domain::admin::projects::AdminProjectsService;
use crate::domain::auth::entities::UserId;
use crate::domain::error::AppError;
use crate::domain::projects::ProjectImportService;

const DEFAULT_LIMIT: i64 = 50;

/// Import payload ceiling: exports build in memory, so the upload is
/// bounded by everything an export could plausibly carry (the sends cap
/// means history rides the json sections; files are assets).
pub(super) const IMPORT_BODY_LIMIT: usize = 32 * 1024 * 1024;

type MaybeProjectsService = Option<Extension<Arc<AdminProjectsService>>>;

fn require_projects_service(
    extension: MaybeProjectsService,
) -> Result<Arc<AdminProjectsService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AppError::NotConfigured.into())
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
        .list_projects(query.get("search").map(String::as_str), per_page, offset)
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
        .ok_or_else(|| AppError::NotFound("project not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "project": AdminProjectDetailDto::from(detail) })),
    ))
}

type MaybeImportService = Option<Extension<Arc<ProjectImportService>>>;

fn require_import_service(
    extension: MaybeImportService,
) -> Result<Arc<ProjectImportService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AppError::NotConfigured.into())
}

/// `POST /admin/projects/import?owner_id=…` — apply an exported project
/// ZIP for `owner_id`'s account (the clone's home). Admin-only by
/// surface; the audit row records the acting admin. The body is the raw
/// zip (32 MiB ceiling — see [`IMPORT_BODY_LIMIT`]).
pub async fn import_project(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeImportService,
    Query(query): Query<HashMap<String, String>>,
    body: Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_import_service(service)?;
    let owner_raw = query
        .get("owner_id")
        .map(|value| value.trim())
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            AppError::Validation(
                "owner_id is required — the account that will own the clone".into(),
            )
        })?;
    let owner: UserId = owner_raw
        .parse()
        .map_err(|_| AppError::Validation("owner_id must be a valid account id".into()))?;

    let report = service.import_zip(&admin, owner, &body).await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "import": report })),
    ))
}
