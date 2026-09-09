use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;

use crate::domain::admin::users::AdminUsersService;
use crate::domain::auth::entities::{UserId, UserStatus};
use crate::domain::auth::errors::AuthError;
use super::super::handlers::CurrentAdmin;
use super::super::super::auth::Problem;
use super::dto::{AdminUserDetailDto, AdminUserDto, SetUserStatusRequest};

const DEFAULT_LIMIT: i64 = 50;

type MaybeUsersService = Option<Extension<Arc<AdminUsersService>>>;

fn require_users_service(extension: MaybeUsersService) -> Result<Arc<AdminUsersService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

fn parse_user_id(raw: &str) -> Result<UserId, Problem> {
    UserId::from_str(raw).map_err(|_| {
        AuthError::Validation("invalid user id".to_string()).into()
    })
}

/// `GET /admin/users` — list platform users (newest first).
pub async fn list_users(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeUsersService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_users_service(service)?;
    let (page, per_page, offset) = super::super::handlers::pagination(&query, DEFAULT_LIMIT);

    let status = query
        .get("status")
        .map(|s| {
            UserStatus::from_str(s.trim()).map_err(|_| {
                Problem::from(AuthError::Validation(
                    "invalid status (expected active or suspended)".to_string(),
                ))
            })
        })
        .transpose()?;

    let (users, total) = service
        .list_users(
            query.get("search").map(String::as_str),
            status,
            per_page,
            offset,
        )
        .await?;

    let dtos: Vec<AdminUserDto> = users.into_iter().map(AdminUserDto::from).collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "users": dtos,
            "page": page,
            "perPage": per_page,
            "total": total,
            "totalPages": super::super::handlers::total_pages(total, per_page),
        })),
    ))
}

/// `GET /admin/users/:id` — one user with stats.
pub async fn get_user(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeUsersService,
    Path(user_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_users_service(service)?;
    let detail = service
        .get_user_detail(parse_user_id(&user_id)?)
        .await?
        .ok_or_else(|| AuthError::NotFound("user not found".into()))?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "user": AdminUserDetailDto::from(detail) })),
    ))
}

/// `PATCH /admin/users/:id/status` — suspend or restore an account.
pub async fn set_user_status(
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeUsersService,
    Path(user_id): Path<String>,
    Json(body): Json<SetUserStatusRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_users_service(service)?;
    let status = UserStatus::from_str(body.status.trim()).map_err(|_| {
        Problem::from(AuthError::Validation(
            "invalid status (expected active or suspended)".to_string(),
        ))
    })?;
    let detail = service
        .set_user_status(&admin, parse_user_id(&user_id)?, status)
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "user": AdminUserDetailDto::from(detail) })),
    ))
}

/// `POST /admin/users/:id/sessions/revoke` — revoke all sessions.
pub async fn revoke_user_sessions(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeUsersService,
    Path(user_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_users_service(service)?;
    service
        .revoke_user_sessions(parse_user_id(&user_id)?)
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok" })),
    ))
}
