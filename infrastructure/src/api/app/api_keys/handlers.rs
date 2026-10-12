//! Project API-key HTTP handlers.
//!
//! The service is injected as an optional axum Extension by the composition
//! root (present only when the database is configured); absent service
//! answers 503, mirroring the invites surface.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;

use super::super::auth::{CurrentUser, Problem};
use super::dto::{
    AddIpRequest, ApiKeyDto, CreateApiKeyRequest, IpAllowEntryDto, UpdateApiKeyRequest,
};
use crate::domain::api_keys::ApiKeyService;
use crate::domain::auth::errors::AuthError;

type MaybeApiKeyService = Option<Extension<Arc<ApiKeyService>>>;

fn require_service(extension: MaybeApiKeyService) -> Result<Arc<ApiKeyService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// `POST /app/projects/:project_id/api-keys` — issue a key. The raw secret
/// is returned exactly once (`secret`); afterwards only the prefix is visible.
pub async fn create_key(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path(project_id): Path<String>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let (key, secret) = service
        .create(
            user.id,
            &project_id,
            &body.name,
            &body.environment,
            &body.permissions,
            body.expires_at,
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "key": ApiKeyDto::from(key), "secret": secret })),
    ))
}

/// `GET /app/projects/:project_id/api-keys` — list keys, newest first.
pub async fn list_keys(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path(project_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let keys = service.list(user.id, &project_id).await?;
    let dtos: Vec<ApiKeyDto> = keys.into_iter().map(ApiKeyDto::from).collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "keys": dtos }))))
}

/// `GET /app/projects/:project_id/api-keys/:key_id` — one key (no secret).
pub async fn get_key(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path((project_id, key_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let key = service.get(user.id, &project_id, &key_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "key": ApiKeyDto::from(key) })),
    ))
}

/// `PATCH /app/projects/:project_id/api-keys/:key_id` — rename and/or
/// enable/disable. At least one field is required.
pub async fn update_key(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path((project_id, key_id)): Path<(String, String)>,
    Json(body): Json<UpdateApiKeyRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    if body.name.is_none() && body.enabled.is_none() {
        return Err(AuthError::Validation(
            "nothing to update (expected name and/or enabled)".into(),
        )
        .into());
    }
    let service = require_service(service)?;
    if let Some(name) = body.name.as_deref() {
        service.rename(user.id, &project_id, &key_id, name).await?;
    }
    if let Some(enabled) = body.enabled {
        service
            .set_enabled(user.id, &project_id, &key_id, enabled)
            .await?;
    }
    let key = service.get(user.id, &project_id, &key_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "key": ApiKeyDto::from(key) })),
    ))
}

/// `POST /app/projects/:project_id/api-keys/:key_id/revoke` — permanently
/// revoke. Idempotent: revoking twice still answers 200.
pub async fn revoke_key(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path((project_id, key_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let key = service.revoke(user.id, &project_id, &key_id).await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "key": ApiKeyDto::from(key) })),
    ))
}

/// `GET /app/projects/:project_id/api-keys/:key_id/ips` — allowlist entries.
pub async fn list_ips(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path((project_id, key_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let entries = service.list_ips(user.id, &project_id, &key_id).await?;
    let dtos: Vec<IpAllowEntryDto> = entries.into_iter().map(IpAllowEntryDto::from).collect();
    Ok((StatusCode::OK, Json(serde_json::json!({ "ips": dtos }))))
}

/// `POST .../ips` — allow an IP or CIDR range on the key.
pub async fn add_ip(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path((project_id, key_id)): Path<(String, String)>,
    Json(body): Json<AddIpRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    let entry = service
        .add_ip(
            user.id,
            &project_id,
            &key_id,
            &body.cidr,
            body.label.as_deref().unwrap_or(""),
        )
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "ip": IpAllowEntryDto::from(entry) })),
    ))
}

/// `DELETE .../ips/:entry_id` — remove an allowlist entry. Removing the
/// last entry unrestricts the key again.
pub async fn remove_ip(
    CurrentUser(user): CurrentUser,
    service: MaybeApiKeyService,
    Path((project_id, key_id, entry_id)): Path<(String, String, String)>,
) -> Result<(StatusCode, Json<serde_json::Value>), Problem> {
    let service = require_service(service)?;
    service
        .remove_ip(user.id, &project_id, &key_id, &entry_id)
        .await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
}
