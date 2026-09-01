//! Projects HTTP handlers.
//!
//! The service is injected as a required axum Extension by the composition
//! root; it is only present when the database is configured. The route
//! itself is absent when the service is unavailable, so handlers can safely
//! assume the extension is present.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;

use crate::domain::projects::ProjectService;
use super::dto::{ProjectDto, UpdateEnvironmentRequest};
use super::super::auth::{CurrentUser, Problem};

/// `GET /v1/projects` — list all projects the current user owns or belongs to.
pub async fn list_projects(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let projects = service.list(user.id).await?;
    let dtos: Vec<ProjectDto> = projects.into_iter().map(ProjectDto::from).collect();
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "projects": dtos })),
    ))
}

/// `PATCH /v1/projects/:id/environment` — switch the project's environment gate.
pub async fn update_environment(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
    Path(project_id): Path<String>,
    Json(body): Json<UpdateEnvironmentRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let project = service.set_environment(user.id, &project_id, &body.environment).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "project": ProjectDto::from(project) })),
    ))
}
