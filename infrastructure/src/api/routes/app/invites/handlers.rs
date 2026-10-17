use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{ProjectInviteDto, ProjectMemberDto};
use crate::domain::auth::errors::AuthError;
use crate::domain::projects::ProjectMembersService;

type MaybeMembersService = Option<Extension<Arc<ProjectMembersService>>>;

fn require_members_service(
    extension: MaybeMembersService,
) -> Result<Arc<ProjectMembersService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `GET /app/invites/{token}` — preview a pending invite for the accept
/// page. Email-matched; everyone else gets 404.
pub async fn get_invite(
    CurrentUser(user): CurrentUser,
    service: MaybeMembersService,
    Path(token): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_members_service(service)?;
    let invite = service.get_invite(user.id, &token).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "invite": ProjectInviteDto::from(invite) })),
    ))
}

/// `POST /app/invites/{token}/accept` — join the project. Enforces the
/// project 2FA gate at this moment, not at invite time.
pub async fn accept_invite(
    CurrentUser(user): CurrentUser,
    service: MaybeMembersService,
    Path(token): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_members_service(service)?;
    let member = service.accept_invite(user.id, &token).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "member": ProjectMemberDto::from(member) })),
    ))
}

/// `POST /app/invites/{token}/decline` — decline quietly. Unknown tokens
/// succeed silently (no invite enumeration).
pub async fn decline_invite(
    CurrentUser(user): CurrentUser,
    service: MaybeMembersService,
    Path(token): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_members_service(service)?;
    service.decline_invite(user.id, &token).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}
