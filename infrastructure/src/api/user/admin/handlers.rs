//! Admin HTTP handlers — bootstrap (first admin) and TOTP 2FA management.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, FromRequestParts, Path, Query};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde_json::json;
use totp_rs::{TOTP, Secret, Rfc6238};

use crate::domain::admin::AdminService;
use crate::domain::admin::entities::{AdminUser, AdminUserId};
use crate::domain::auth::errors::AuthError;
use super::dto::{
    AdminAccountDto, AdminStatusResponse, ApprovalDto, BootstrapRequest, BootstrapResponse,
    ChangePasswordRequest, CreateAdminRequest, ForgotPasswordRequest, ResetPasswordRequest,
    TotpSetupResponse, VerifyTotpRequest,
};
use super::super::auth::middleware::{ADMIN_SESSION_COOKIE, Problem};

type MaybeAdminService = Option<Extension<Arc<AdminService>>>;

fn require_admin_service(extension: MaybeAdminService) -> Result<Arc<AdminService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

/// Parses `?page=` / `?per_page=` (with legacy `?limit=` as the per-page
/// alias). Returns `(page, per_page, offset)`.
pub fn pagination(query: &HashMap<String, String>, default_per_page: i64) -> (i64, i64, i64) {
    let per_page = query
        .get("per_page")
        .or_else(|| query.get("limit"))
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default_per_page)
        .clamp(1, 200);
    let page = query
        .get("page")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1)
        .max(1);
    (page, per_page, (page - 1) * per_page)
}

/// Total pages for `total` items at `per_page` per page.
pub fn total_pages(total: i64, per_page: i64) -> i64 {
    (total + per_page - 1) / per_page
}

/// Extractor for admin endpoints that require an authenticated admin.
#[derive(Debug, Clone)]
pub struct CurrentAdmin(pub Arc<AdminUser>);

impl<S> FromRequestParts<S> for CurrentAdmin
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let service = parts
            .extensions
            .get::<Arc<AdminService>>()
            .cloned()
            .ok_or_else(|| {
                super::super::auth::middleware::problem_response(AuthError::NotConfigured)
            })?;

        let jar = CookieJar::from_request_parts(parts, _state)
            .await
            .map_err(|_| {
                super::super::auth::middleware::problem_response(AuthError::Unauthorized)
            })?;

        let raw_cookie = jar
            .get(ADMIN_SESSION_COOKIE)
            .map(|cookie| cookie.value().to_string())
            .ok_or_else(|| {
                super::super::auth::middleware::problem_response(AuthError::Unauthorized)
            })?;

        let admin = service
            .authenticate(&raw_cookie)
            .await
            .map_err(super::super::auth::middleware::problem_response)?;

        Ok(Self(Arc::new(admin)))
    }
}

/// Generates a random secret bytes and returns (secret_bytes, base32_string).
fn generate_totp_secret() -> Result<(Vec<u8>, String), AuthError> {
    let mut bytes = vec![0u8; 20];
    use rand::RngCore;
    rand::rng().fill_bytes(&mut bytes);
    let secret = Secret::Raw(bytes.clone());
    let encoded = secret.to_encoded();
    let base32 = encoded.to_string();
    Ok((bytes, base32))
}

/// Builds a TOTP instance from a base32 secret string.
fn build_totp_from_base32(base32: &str) -> Result<TOTP, AuthError> {
    let secret = Secret::Encoded(base32.to_string());
    let secret_raw = secret.to_raw().map_err(|e| {
        AuthError::Storage(format!("invalid TOTP secret: {e}"))
    })?;
    let rfc = Rfc6238::with_defaults(secret_raw.to_bytes().map_err(|e| {
        AuthError::Storage(format!("TOTP secret conversion failed: {e}"))
    })?).map_err(|e| {
        AuthError::Storage(format!("TOTP config failed: {e}"))
    })?;
    TOTP::from_rfc6238(rfc).map_err(|e| {
        AuthError::Storage(format!("TOTP creation failed: {e}"))
    })
}

/// `GET /admin/status` — returns whether any admin user exists.
/// No authentication required (used by the setup flow).
pub async fn admin_status(service: MaybeAdminService) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let admin_exists = service.admin_exists().await.map_err(Problem::from)?;
    Ok(Json(json!(AdminStatusResponse { admin_exists })).into_response())
}

/// `POST /admin/bootstrap` — creates the first admin user with TOTP.
/// Only works when no admin exists yet.
pub async fn bootstrap(
    jar: CookieJar,
    service: MaybeAdminService,
    Json(request): Json<BootstrapRequest>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;

    // Ensure no admin already exists
    if service.admin_exists().await.map_err(Problem::from)? {
        return Err(AuthError::Validation(
            "An admin account already exists. Use the login flow instead.".to_string(),
        )
        .into());
    }

    // Generate TOTP secret
    let (_secret_bytes, totp_secret) = generate_totp_secret()?;

    // Build TOTP URI for QR code
    let mut totp = build_totp_from_base32(&totp_secret)?;
    totp.issuer = Some("Notifi".to_string());
    totp.account_name = request.email.clone();
    let totp_uri = totp.get_url().to_string();

    // Create the admin
    let admin = service
        .create_admin(&request.name, &request.email, &request.password)
        .await
        .map_err(Problem::from)?;

    // Store the TOTP secret
    service
        .set_totp_secret(admin.id, totp_secret.clone())
        .await
        .map_err(Problem::from)?;

    // Issue a real session so the admin can complete TOTP setup
    let session_token = service
        .issue_session(admin.id)
        .await
        .map_err(Problem::from)?;

    let cookie = Cookie::build((ADMIN_SESSION_COOKIE, session_token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(1));
    let jar = jar.add(cookie);

    Ok((
        StatusCode::CREATED,
        jar,
        Json(json!(BootstrapResponse {
            user_id: admin.id.to_string(),
            totp_secret,
            totp_uri,
        })),
    )
        .into_response())
}

/// `POST /admin/login` — authenticates an admin with email/password and optional TOTP code.
pub async fn login(
    jar: CookieJar,
    service: MaybeAdminService,
    Json(request): Json<LoginRequest>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;

    let admin = service
        .verify_credentials(&request.email, &request.password)
        .await
        .map_err(Problem::from)?;

    // TOTP not yet completed: secret exists but never verified. Issue a
    // session (so the admin can call totp/setup + totp/verify) and force the
    // admin to finish 2FA setup before gaining full access.
    if admin.totp_secret.is_some() && !admin.totp_enabled {
        let session_token = service
            .issue_session(admin.id)
            .await
            .map_err(Problem::from)?;
        let cookie = Cookie::build((ADMIN_SESSION_COOKIE, session_token))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .max_age(time::Duration::days(1));
        let jar = jar.add(cookie);

        return Ok((
            StatusCode::OK,
            jar,
            Json(json!({
                "requires_totp_setup": true,
                "user_id": admin.id.to_string(),
            })),
        )
            .into_response());
    }

    // TOTP enabled but no code supplied: ask for the code.
    if admin.totp_enabled && request.totp_code.is_none() {
        return Ok((
            StatusCode::OK,
            Json(json!({
                "requires_totp": true,
                "user_id": admin.id.to_string(),
            })),
        )
            .into_response());
    }

    // TOTP enabled: verify the supplied code.
    if admin.totp_enabled {
        let totp_secret = admin.totp_secret.as_ref().ok_or_else(|| {
            AuthError::Validation("TOTP has not been set up yet".to_string())
        })?;
        let totp = build_totp_from_base32(totp_secret)?;
        let code = request.totp_code.as_deref().ok_or_else(|| {
            AuthError::Validation("TOTP code required".to_string())
        })?;
        if !totp.check_current(code).map_err(|e| {
            AuthError::Storage(format!("TOTP verification failed: {e}"))
        })? {
            return Err(AuthError::TotpInvalid.into());
        }
    }

    // Issue a fresh session.
    let session_token = service
        .issue_session(admin.id)
        .await
        .map_err(Problem::from)?;

    let cookie = Cookie::build((ADMIN_SESSION_COOKIE, session_token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(1));
    let jar = jar.add(cookie);

    Ok((
        StatusCode::OK,
        jar,
        Json(json!({
            "user_id": admin.id.to_string(),
        })),
    )
        .into_response())
}

/// `POST /admin/totp/setup` — returns the TOTP URI for QR code generation.
/// Get-or-create: reuses the pending secret when 2FA isn't enabled yet (so a
/// refresh or the bootstrap handoff never orphans a displayed QR code).
/// Pass `?regenerate=true` to rotate to a fresh secret instead.
pub async fn totp_setup(
    current_admin: CurrentAdmin,
    service: MaybeAdminService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let admin = current_admin.0;
    let regenerate = query
        .get("regenerate")
        .is_some_and(|v| v == "true" || v == "1");

    let totp_secret = match (&admin.totp_secret, admin.totp_enabled, regenerate) {
        (Some(secret), false, false) => secret.clone(),
        _ => {
            let (_secret_bytes, secret) = generate_totp_secret()?;
            service
                .set_totp_secret(admin.id, secret.clone())
                .await
                .map_err(Problem::from)?;
            secret
        }
    };

    let mut totp = build_totp_from_base32(&totp_secret)?;
    totp.issuer = Some("Notifi".to_string());
    totp.account_name = admin.email.to_string();
    let totp_uri = totp.get_url().to_string();

    Ok(Json(json!(TotpSetupResponse {
        totp_secret,
        totp_uri,
    }))
    .into_response())
}

/// `POST /admin/totp/verify` — verifies a TOTP code and enables 2FA.
pub async fn totp_verify(
    current_admin: CurrentAdmin,
    service: MaybeAdminService,
    Json(request): Json<VerifyTotpRequest>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let admin = current_admin.0;

    let totp_secret = admin
        .totp_secret
        .as_ref()
        .ok_or_else(|| AuthError::Validation("TOTP has not been set up yet".to_string()))?;

    let totp = build_totp_from_base32(totp_secret)?;

    if !totp.check_current(&request.code).map_err(|e| {
        AuthError::Storage(format!("TOTP verification failed: {e}"))
    })? {
        return Err(AuthError::TotpInvalid.into());
    }

    service
        .enable_totp(admin.id)
        .await
        .map_err(Problem::from)?;

    Ok(Json(json!({ "status": "ok" })).into_response())
}

/// `GET /admin/me` — returns the authenticated admin (session check).
pub async fn me(CurrentAdmin(admin): CurrentAdmin) -> Result<Response, Problem> {
    Ok(Json(json!({
        "id": admin.id.to_string(),
        "name": admin.name,
        "email": admin.email.to_string(),
        "totpEnabled": admin.totp_enabled,
        "isSuperAdmin": admin.is_super_admin,
        "status": admin.status.as_str(),
    }))
    .into_response())
}

/// `POST /admin/password/forgot` — always 200; never reveals whether the
/// account exists. The reset token is only exposed in local dev mode.
pub async fn forgot_password(
    service: MaybeAdminService,
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let raw_token = service
        .forgot_password(&request.email)
        .await
        .map_err(Problem::from)?;

    let mut body = json!({ "status": "ok" });
    if let (true, Some(token)) = (service.exposes_dev_tokens(), raw_token) {
        body["resetToken"] = json!(token);
    }
    Ok(Json(body).into_response())
}

/// `POST /admin/password/reset` — consumes the token, rotates the password,
/// revokes all existing sessions for the account.
pub async fn reset_password(
    service: MaybeAdminService,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    service
        .reset_password(&request.token, &request.password)
        .await
        .map_err(Problem::from)?;

    Ok(Json(json!({ "status": "ok" })).into_response())
}

/// `POST /admin/logout` — revokes the session behind the cookie.
pub async fn logout(jar: CookieJar, service: MaybeAdminService) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    if let Some(raw) = jar.get(ADMIN_SESSION_COOKIE).map(|cookie| cookie.value()) {
        service.logout(raw).await.map_err(Problem::from)?;
    }

    // Clear the cookie even when there was nothing to revoke (idempotent).
    let removal = Cookie::build(ADMIN_SESSION_COOKIE).path("/").build();
    Ok((jar.remove(removal), Json(json!({ "status": "ok" }))).into_response())
}

/// `POST /admin/password/change` — rotates the caller's own password.
/// Verifies the current password, then revokes every session and issues a
/// fresh one so the caller stays signed in.
pub async fn change_password(
    jar: CookieJar,
    CurrentAdmin(admin): CurrentAdmin,
    service: MaybeAdminService,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let session_token = service
        .change_password(&admin, &request.current_password, &request.new_password)
        .await
        .map_err(Problem::from)?;

    let cookie = Cookie::build((ADMIN_SESSION_COOKIE, session_token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(1));
    let jar = jar.add(cookie);

    Ok((jar, Json(json!({ "status": "ok" }))).into_response())
}

/// `GET /admin/admins` — lists all admin accounts.
pub async fn list_admins(
    CurrentAdmin(_admin): CurrentAdmin,
    service: MaybeAdminService,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let admins = service.list_admins().await.map_err(Problem::from)?;
    let dtos: Vec<AdminAccountDto> = admins.into_iter().map(AdminAccountDto::from).collect();
    Ok(Json(json!({ "admins": dtos })).into_response())
}

/// `POST /admin/admins` — creates another admin. Super-admin creations go
/// active immediately; everyone else's land pending with an approval request.
pub async fn create_admin(
    CurrentAdmin(caller): CurrentAdmin,
    service: MaybeAdminService,
    Json(request): Json<CreateAdminRequest>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let admin = service
        .invite_admin(&caller, &request.name, &request.email, &request.password)
        .await
        .map_err(Problem::from)?;
    let pending = admin.status != crate::domain::admin::entities::AdminStatus::Active;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "admin": AdminAccountDto::from(admin),
            "pendingApproval": pending,
        })),
    )
        .into_response())
}

/// `POST /admin/admins/:id/remove` — requests (or, for the super admin,
/// immediately applies) the removal of another admin.
pub async fn remove_admin(
    CurrentAdmin(caller): CurrentAdmin,
    service: MaybeAdminService,
    Path(admin_id): Path<String>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let target = parse_admin_id(&admin_id)?;
    let applied = service
        .request_removal(&caller, target)
        .await
        .map_err(Problem::from)?;
    Ok(Json(json!({ "status": "ok", "applied": applied })).into_response())
}

/// `GET /admin/approvals` — lists approval requests (super admin only).
pub async fn list_approvals(
    CurrentAdmin(caller): CurrentAdmin,
    service: MaybeAdminService,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    let views = service
        .list_approval_views(&caller, query.get("status").map(String::as_str))
        .await
        .map_err(Problem::from)?;
    let dtos: Vec<ApprovalDto> = views.into_iter().map(ApprovalDto::from).collect();
    Ok(Json(json!({ "approvals": dtos })).into_response())
}

/// `POST /admin/approvals/:id/approve` — approves a request (super only).
pub async fn approve_request(
    CurrentAdmin(caller): CurrentAdmin,
    service: MaybeAdminService,
    Path(request_id): Path<String>,
) -> Result<Response, Problem> {
    decide_request(caller, service, &request_id, true).await
}

/// `POST /admin/approvals/:id/reject` — rejects a request (super only).
pub async fn reject_request(
    CurrentAdmin(caller): CurrentAdmin,
    service: MaybeAdminService,
    Path(request_id): Path<String>,
) -> Result<Response, Problem> {
    decide_request(caller, service, &request_id, false).await
}

async fn decide_request(
    caller: Arc<AdminUser>,
    service: MaybeAdminService,
    request_id: &str,
    approve: bool,
) -> Result<Response, Problem> {
    let service = require_admin_service(service)?;
    service
        .decide_approval(&caller, request_id, approve)
        .await
        .map_err(Problem::from)?;
    Ok(Json(json!({ "status": "ok" })).into_response())
}

fn parse_admin_id(raw: &str) -> Result<AdminUserId, Problem> {
    use std::str::FromStr;
    AdminUserId::from_str(raw).map_err(|_| {
        AuthError::Validation("invalid admin id".to_string()).into()
    })
}

/// Login request body.
#[derive(serde::Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub totp_code: Option<String>,
}
