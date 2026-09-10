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

use super::super::auth::{CurrentUser, Problem};
use super::dto::{
    AddMemberRequest, CreateProjectRequest, ProjectDto, ProjectMemberDto, SetRequire2faRequest,
    UpdateEnvironmentRequest,
};
use crate::domain::auth::errors::AuthError;
use crate::domain::projects::{ProjectMembersService, ProjectService};

/// `GET /app/projects` — list all projects the current user owns or belongs to.
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

/// `POST /app/projects` — create a new project.
pub async fn create_project(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let project = service
        .create(user.id, &body.name, body.description.as_deref())
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "project": ProjectDto::from(project) })),
    ))
}

/// `PATCH /app/projects/:id/environment` — switch the project's environment gate.
pub async fn update_environment(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
    Path(project_id): Path<String>,
    Json(body): Json<UpdateEnvironmentRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let project = service
        .set_environment(user.id, &project_id, &body.environment)
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "project": ProjectDto::from(project) })),
    ))
}

type MaybeMembersService = Option<Extension<Arc<ProjectMembersService>>>;

fn require_members_service(
    extension: MaybeMembersService,
) -> Result<Arc<ProjectMembersService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `PATCH /app/projects/{id}/require-2fa` — flip the "new members must
/// have 2FA" flag. Owner or admin only.
pub async fn set_require_2fa(
    CurrentUser(user): CurrentUser,
    service: MaybeMembersService,
    Path(project_id): Path<String>,
    Json(body): Json<SetRequire2faRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_members_service(service)?;
    let enabled = service
        .set_require_2fa(user.id, &project_id, body.enabled)
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "require2fa": enabled })),
    ))
}

/// `POST /app/projects/{id}/members` — add an existing account to the
/// team. Owner or admin only; refused when the project requires 2FA and
/// the invited account lacks it.
pub async fn add_member(
    CurrentUser(user): CurrentUser,
    service: MaybeMembersService,
    Path(project_id): Path<String>,
    Json(body): Json<AddMemberRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_members_service(service)?;
    let member = service
        .add_member(user.id, &project_id, &body.email, &body.role)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "member": ProjectMemberDto::from(member) })),
    ))
}

/// `GET /app/projects/{id}/members` — team roster with 2FA standing.
pub async fn list_members(
    CurrentUser(user): CurrentUser,
    service: MaybeMembersService,
    Path(project_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_members_service(service)?;
    let members = service.list_members(user.id, &project_id).await?;
    let dtos: Vec<ProjectMemberDto> = members.into_iter().map(ProjectMemberDto::from).collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "members": dtos }))))
}
