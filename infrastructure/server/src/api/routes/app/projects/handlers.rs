//! Projects HTTP handlers.
//!
//! The service is injected as a required axum Extension by the composition
//! root; it is only present when the database is configured. The route
//! itself is absent when the service is unavailable, so handlers can safely
//! assume the extension is present.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use super::super::auth::{CurrentUser, Problem};
use super::dto::{
    AddMemberRequest, CreateProjectRequest, ProjectDto, ProjectInviteDto, ProjectMemberDto,
    SetRequire2faRequest, UpdateActiveRequest,
};
use crate::domain::error::AppError;
use crate::domain::projects::{ProjectExportService, ProjectMembersService, ProjectService};

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

/// `PATCH /app/projects/:id/active` — flip the project's active switch.
pub async fn update_active(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
    Path(project_id): Path<String>,
    Json(body): Json<UpdateActiveRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let project = service
        .set_active(user.id, &project_id, body.active)
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
        .ok_or_else(|| AppError::NotConfigured.into())
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

/// `DELETE /app/projects/{id}` — request deletion. Owner or admin only.
/// The project freezes immediately (invisible to all delivery and future
/// traffic), starts a two-week countdown, and keeps serving its members
/// data downloads and a cancel path until the worker purges it.
pub async fn request_deletion(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
    Path(project_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let project = service.request_deletion(user.id, &project_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "project": ProjectDto::from(project),
            "graceDays": crate::domain::projects::PURGE_GRACE.num_days(),
        })),
    ))
}

/// `POST /app/projects/{id}/deletion/cancel` — cancel a pending deletion
/// inside the countdown. Owner or admin only. A cancel is a full undo:
/// nothing was destroyed, so deleting the tombstone restores everything.
pub async fn cancel_deletion(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
    Path(project_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let project = service.cancel_deletion(user.id, &project_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "project": ProjectDto::from(project) })),
    ))
}

/// `GET /app/projects/{id}/export` — one data-export download: rows as
/// JSON plus project files, as a ZIP. Any member; limited to
/// [`EXPORTS_PER_DAY`] downloads per project per rolling 24h (the guard
/// runs before the bytes are assembled, so spam never reaches the store).
pub async fn export_zip(
    CurrentUser(user): CurrentUser,
    Extension(service): Extension<Arc<ProjectService>>,
    Extension(export): Extension<Arc<ProjectExportService>>,
    Path(project_id): Path<String>,
) -> Result<Response, Problem> {
    service.guard_export(user.id, &project_id).await?;

    let bundle = export.build_zip(user.id, &project_id).await?;
    let mut response = bundle.bytes.into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    let filename = HeaderValue::from_str(&format!("attachment; filename=\"{}\"", bundle.filename))
        .map_err(|_| AppError::Storage("export filename broke header encoding".into()))?;
    response.headers_mut().insert(CONTENT_DISPOSITION, filename);
    Ok(response)
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
    let (invite, invite_token) = service
        .invite_member(user.id, &project_id, &body.email, &body.role)
        .await?;
    let mut payload = serde_json::json!({ "invite": ProjectInviteDto::from(invite) });
    // Dev only: raw token so the invite link is clickable without email.
    if let Some(token) = invite_token {
        payload["inviteToken"] = serde_json::json!(token);
    }
    Ok((StatusCode::CREATED, Json(payload)))
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
