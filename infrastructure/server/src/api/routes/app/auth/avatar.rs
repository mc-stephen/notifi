//! The user folder's first resident: one avatar image per dashboard
//! user, stored in the config store under `users/{id}/avatar.{ext}` and
//! referenced by the row's `avatar_url` — which is either that storage
//! key or an external OAuth provider URL (imported at login), served
//! as a redirect. Originals are kept as-is: no server-side resizing.

use std::sync::Arc;

use axum::Extension;
use axum::body::Bytes;
use axum::http::HeaderMap;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{IntoResponse, Redirect, Response};
use serde_json::json;

use super::handlers::{MaybeService, ok_json, require_service};
use super::middleware::{CurrentUser, Problem};
use crate::domain::error::AppError;
use crate::ports::config_store::ConfigStore;

/// The upload cap — small enough for profile pictures, generous enough
/// for a real photo.
const MAX_AVATAR_BYTES: usize = 2 * 1024 * 1024;

/// The image kinds the app speaks: content type in, file extension out.
fn image_kind(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" | "image/jpg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

fn ext_to_content_type(ext: &str) -> &'static str {
    match ext {
        "jpg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

type MaybeStore = Option<Extension<Arc<dyn ConfigStore + Send + Sync>>>;

fn require_store(extension: MaybeStore) -> Result<Arc<dyn ConfigStore + Send + Sync>, Problem> {
    extension
        .map(|Extension(store)| store)
        .ok_or_else(|| AppError::NotConfigured.into())
}

/// `POST /app/auth/profile/avatar` — the user folder's one avatar file:
/// clean out the previous stored image when its kind differs, keep the
/// new original bytes, point the row at it.
pub async fn upload(
    current_user: CurrentUser,
    service: MaybeService,
    store: MaybeStore,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let store = require_store(store)?;
    let user = current_user.0;

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let ext = image_kind(content_type).ok_or_else(|| {
        Problem::from(AppError::Validation(
            "avatar must be a jpeg, png or webp image".to_string(),
        ))
    })?;
    if bytes.is_empty() {
        return Err(Problem::from(AppError::Validation(
            "avatar upload was empty".to_string(),
        )));
    }
    if bytes.len() > MAX_AVATAR_BYTES {
        return Err(Problem::from(AppError::Validation(
            "avatar image must be 2 MiB or smaller".to_string(),
        )));
    }

    let key = format!("users/{}/avatar.{ext}", user.id);
    // The previous file goes first — the new original replaces it,
    // best-effort so a stale file never blocks a fresh save.
    let _ = store
        .delete_raw(user.avatar_url.as_deref().unwrap_or(&key))
        .await;
    store
        .put_raw(&key, bytes.to_vec())
        .await
        .map_err(AppError::Storage)?;
    service.set_avatar_url(user.id, Some(key)).await?;

    Ok(ok_json(json!({"status": "ok"})))
}

/// `GET /app/auth/profile/avatar` — the avatar reader: store bytes for
/// app-stored keys, a redirect for external OAuth provider URLs.
pub async fn fetch(
    current_user: CurrentUser,
    service: MaybeService,
    store: MaybeStore,
) -> Result<Response, Problem> {
    require_service(service)?;
    let user = current_user.0;
    let reference = user
        .avatar_url
        .as_deref()
        .ok_or_else(|| Problem::from(AppError::NotFound("no avatar set".to_string())))?;
    if !reference.starts_with("users/") {
        return Ok(Redirect::temporary(reference).into_response());
    }
    let store = require_store(store)?;
    let bytes = store.load_raw(reference).await.map_err(AppError::Storage)?;
    let ext = reference.rsplit('.').next().unwrap_or("png");
    Ok((
        [
            (CONTENT_TYPE, ext_to_content_type(ext)),
            (CACHE_CONTROL, "public, max-age=3600"),
        ],
        bytes,
    )
        .into_response())
}

/// `DELETE /app/auth/profile/avatar` — clear the reference, and remove
/// the stored file when we hold it (external OAuth URLs leave it up to
/// the provider).
pub async fn remove(
    current_user: CurrentUser,
    service: MaybeService,
    store: MaybeStore,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let user = current_user.0;

    if let Some(previous) = &user.avatar_url
        && previous.starts_with("users/")
        && let Some(Extension(store)) = store
    {
        let _ = store.delete_raw(previous).await;
    }
    service.set_avatar_url(user.id, None).await?;

    Ok(ok_json(json!({"status": "ok"})))
}
