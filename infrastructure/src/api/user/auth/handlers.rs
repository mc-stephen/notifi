//! Auth HTTP handlers.
//!
//! The service is injected as an axum Extension by the composition root;
//! when absent (no database), every route answers 503 problem documents.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::{Cookie, SameSite};
use serde_json::{Value, json};

use crate::domain::auth::AuthService;
use crate::domain::auth::entities::User;
use crate::domain::auth::errors::AuthError;
use crate::ports::oauth::{OAuthError, OAuthRuntime};
use crate::domain::auth::value_objects::new_token;
use super::dto::{
    CompleteOnboardingRequest, ForgotPasswordRequest, LoginRequest, ResendVerificationRequest,
    ResetPasswordRequest, SessionDto, SignupRequest, UserDto, VerifyEmailRequest,
};
use super::middleware::{CurrentUser, Problem, SESSION_COOKIE};

type MaybeService = Option<Extension<Arc<AuthService>>>;
type MaybeOAuth = Option<Extension<Arc<OAuthRuntime>>>;

/// How long the CSRF-state / PKCE-verifier / mode cookies live — plenty for
/// a consent detour, short enough to be useless for replay.
const OAUTH_TEMP_COOKIE_MAX_AGE_SECONDS: i64 = 600;

fn require_service(extension: MaybeService) -> Result<Arc<AuthService>, Problem> {
    extension
        .map(|Extension(service)| service)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

fn require_oauth(extension: MaybeOAuth) -> Result<Arc<OAuthRuntime>, Problem> {
    extension
        .map(|Extension(runtime)| runtime)
        .ok_or_else(|| AuthError::NotConfigured.into())
}

fn ok_json(body: Value) -> Response {
    (StatusCode::OK, Json(body)).into_response()
}

/// Resolves the "can skip onboarding" flag (org + project ownership).
/// Storage failures read as `false` so users are guided through onboarding
/// rather than blocked from it.
async fn onboarding_flag(service: &AuthService, user: &User) -> bool {
    service.onboarding_completed(user.id).await.unwrap_or(false)
}

// -- oauth ------------------------------------------------------------------

fn valid_oauth_provider(provider: &str) -> bool {
    matches!(provider, "github" | "google")
}

fn oauth_temp_cookie(provider: &str, kind: &str) -> String {
    format!("oauth_{kind}_{provider}")
}

/// Maps port errors onto problem documents (unconfigured → 503).
fn oauth_problem(err: OAuthError) -> Problem {
    match err {
        OAuthError::NotConfigured(_) => AuthError::NotConfigured.into(),
        OAuthError::Flow(m) => AuthError::Storage(m).into(),
    }
}

fn origin_of(url: &str) -> String {
    let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    format!("{scheme}://{host}")
}

/// The tiny page rendered inside the popup: hands the result back to the
/// opener over postMessage (origin-locked to the dashboard) and closes.
fn popup_html(dashboard_url: &str, payload: &Value) -> String {
    let origin = origin_of(dashboard_url);
    format!(
        r#"<!DOCTYPE html><html><body><script>
(function() {{
  try {{ window.opener.postMessage({payload}, "{origin}"); }} catch (e) {{}}
  window.close();
}})();
</script></body></html>"#
    )
}

fn oauth_failure(dashboard_url: &str, mode: &str) -> Response {
    if mode == "popup" {
        Html(popup_html(dashboard_url, &json!({ "type": "oauth:error" }))).into_response()
    } else {
        Redirect::to(&format!(
            "{}/auth/login?error=oauth_failed",
            dashboard_url.trim_end_matches('/')
        ))
        .into_response()
    }
}

/// `GET /v1/auth/oauth/{provider}` — redirects the browser to the provider's
/// consent screen. `?popup=1` switches the callback into postMessage mode.
pub async fn oauth_start(
    Path(provider): Path<String>,
    jar: CookieJar,
    Query(query): Query<HashMap<String, String>>,
    oauth: MaybeOAuth,
) -> Result<Response, Problem> {
    if !valid_oauth_provider(&provider) {
        return Err(AuthError::Validation(format!(
            "unsupported OAuth provider '{provider}'"
        ))
        .into());
    }
    let runtime = require_oauth(oauth)?;

    let state = new_token().0;
    let start = runtime
        .provider
        .authorize_url(&provider, &state)
        .map_err(oauth_problem)?;

    let mode = if query.get("popup").map(String::as_str) == Some("1") {
        "popup"
    } else {
        "redirect"
    };
    let cookie_path = format!("/v1/auth/oauth/{provider}/callback");
    let temp_cookie = |kind: &str, value: String| {
        Cookie::build((oauth_temp_cookie(&provider, kind), value))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path(cookie_path.clone())
            .max_age(time::Duration::seconds(OAUTH_TEMP_COOKIE_MAX_AGE_SECONDS))
    };
    let jar = jar
        .add(temp_cookie("state", state))
        .add(temp_cookie("verifier", start.pkce_verifier))
        .add(temp_cookie("mode", mode.to_string()));

    Ok((jar, Redirect::to(start.url.as_str())).into_response())
}

/// `GET /v1/auth/oauth/{provider}/callback` — exchanges the code, signs the
/// person in, sets the session cookie, then either postMessages the popup
/// opener or redirects back to the dashboard.
#[allow(clippy::too_many_lines)]
pub async fn oauth_callback(
    Path(provider): Path<String>,
    jar: CookieJar,
    Query(query): Query<HashMap<String, String>>,
    service: MaybeService,
    oauth: MaybeOAuth,
) -> Result<Response, Problem> {
    let runtime = require_oauth(oauth)?;
    let service = require_service(service)?;

    let fail = |mode: &str| Ok(oauth_failure(&runtime.dashboard_url, mode));

    if !valid_oauth_provider(&provider) {
        return fail("redirect");
    }

    let state_cookie = jar.get(oauth_temp_cookie(&provider, "state").as_str());
    let verifier_cookie = jar.get(oauth_temp_cookie(&provider, "verifier").as_str());
    let mode = jar
        .get(oauth_temp_cookie(&provider, "mode").as_str())
        .map(|cookie| cookie.value().to_string())
        .unwrap_or_else(|| "redirect".to_string());

    // Provider-side denial (user clicked "cancel") or missing cookies.
    if query.contains_key("error") {
        return fail(&mode);
    }
    let (Some(state_cookie), Some(verifier_cookie)) = (state_cookie, verifier_cookie) else {
        return fail(&mode);
    };

    // CSRF check: the state we parked must equal what the provider echoed.
    if query.get("state").map(String::as_str) != Some(state_cookie.value()) {
        return fail(&mode);
    }
    let Some(code) = query.get("code") else {
        return fail(&mode);
    };

    let profile = runtime
        .provider
        .exchange_code(&provider, code, verifier_cookie.value())
        .await
        .map_err(oauth_problem)?;
    let issued = service
        .login_with_oauth(&provider, profile)
        .await
        .map_err(Problem::from)?;

    // Session cookie (signup/login semantics: rememberMe=false → 1 day).
    let session_cookie = Cookie::build((SESSION_COOKIE, issued.raw_token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(1));
    let clear = |kind: &str| {
        Cookie::build(oauth_temp_cookie(&provider, kind))
            .path(format!("/v1/auth/oauth/{provider}/callback"))
            .build()
    };
    let jar = jar
        .add(session_cookie)
        .remove(clear("state"))
        .remove(clear("verifier"))
        .remove(clear("mode"));

    if mode == "popup" {
        Ok((
            jar,
            Html(popup_html(
                &runtime.dashboard_url,
                &json!({ "type": "oauth:success" }),
            )),
        )
            .into_response())
    } else {
        Ok((
            jar,
            Redirect::to(runtime.dashboard_url.trim_end_matches('/')),
        )
            .into_response())
    }
}

/// `POST /v1/auth/signup` — creates the account, starts a session
/// (rememberMe=false → 1-day cookie), and a verification token.
pub async fn signup(
    jar: CookieJar,
    service: MaybeService,
    Json(request): Json<SignupRequest>,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let outcome = service
        .signup(&request.name, &request.email, &request.password)
        .await
        .map_err(Problem::from)?;

    // The signup session lets the browser continue into onboarding and the
    // dashboard without a detour through the login page.
    let cookie = Cookie::build((SESSION_COOKIE, outcome.raw_token.clone()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(1));
    let jar = jar.add(cookie);

    let user_dto = UserDto::from(&outcome.user);
    let mut body = json!({
        "user": user_dto.clone(),
        "session": SessionDto {
            user: user_dto,
            token: outcome.raw_token,
            expires_at: outcome.session.expires_at,
            onboarding_completed: onboarding_flag(&service, &outcome.user).await,
        },
    });
    if service.exposes_dev_tokens() {
        // Local-dev only until M4 wires real email delivery.
        body["verificationToken"] = json!(outcome.verification_token);
    }
    Ok((StatusCode::CREATED, jar, Json(body)).into_response())
}

/// `POST /v1/auth/login` — verifies credentials, starts a session, sets the
/// httpOnly `session_token` cookie (30 days with `rememberMe`, else 1 day).
pub async fn login(
    jar: CookieJar,
    service: MaybeService,
    Json(request): Json<LoginRequest>,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let issued = service
        .login(&request.email, &request.password, request.remember_me)
        .await
        .map_err(Problem::from)?;

    let ttl_days = if request.remember_me { 30 } else { 1 };
    let cookie = Cookie::build((SESSION_COOKIE, issued.raw_token.clone()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(ttl_days));
    let jar = jar.add(cookie);

    let user_dto = UserDto::from(&issued.user);
    let body = json!({
        "user": user_dto.clone(),
        "session": SessionDto {
            user: user_dto,
            token: issued.raw_token,
            expires_at: issued.session.expires_at,
            onboarding_completed: onboarding_flag(&service, &issued.user).await,
        },
    });
    Ok((jar, Json(body)).into_response())
}

/// `POST /v1/auth/logout` — revokes the session behind the cookie.
pub async fn logout(jar: CookieJar, service: MaybeService) -> Result<Response, Problem> {
    let service = require_service(service)?;
    if let Some(raw) = jar.get(SESSION_COOKIE).map(|cookie| cookie.value()) {
        service.logout(raw).await.map_err(Problem::from)?;
    }

    // clear the cookie even when there was nothing to revoke (idempotent)
    let removal = Cookie::build(SESSION_COOKIE).path("/").build();
    Ok((jar.remove(removal), ok_json(json!({"status": "ok"}))).into_response())
}

/// `GET /v1/auth/me` — the signed-in user (requires a valid session cookie).
pub async fn me(
    current_user: CurrentUser,
    service: MaybeService,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let user = current_user.0;
    let flag = onboarding_flag(&service, &user).await;
    Ok(ok_json(
        json!({
            "user": UserDto::from(user.as_ref()),
            "onboardingCompleted": flag,
        }),
    ))
}

/// `POST /v1/auth/onboarding/complete` — persists the first organization +
/// project collected by the dashboard flow. Idempotent: completing again
/// answers 200 with `alreadyCompleted`.
pub async fn complete_onboarding(
    current_user: CurrentUser,
    service: MaybeService,
    Json(request): Json<CompleteOnboardingRequest>,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let user = current_user.0;

    if service
        .onboarding_completed(user.id)
        .await
        .map_err(Problem::from)?
    {
        return Ok(ok_json(json!({ "status": "ok", "alreadyCompleted": true })));
    }

    service
        .complete_onboarding(user.id, request.into_input())
        .await
        .map_err(Problem::from)?;

    Ok(ok_json(json!({ "status": "ok" })))
}

/// `POST /v1/auth/password/forgot` — always 200; never reveals whether the
/// account exists. The reset token is only exposed in local dev mode.
pub async fn forgot_password(
    service: MaybeService,
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let raw_token = service
        .forgot_password(&request.email)
        .await
        .map_err(Problem::from)?;

    let mut body = json!({ "status": "ok" });
    if let (true, Some(token)) = (service.exposes_dev_tokens(), raw_token) {
        body["resetToken"] = json!(token);
    }
    Ok(ok_json(body))
}

/// `POST /v1/auth/password/reset` — consumes the token, rotates the password,
/// revokes all existing sessions for the account.
pub async fn reset_password(
    service: MaybeService,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    service
        .reset_password(&request.token, &request.password)
        .await
        .map_err(Problem::from)?;

    Ok(ok_json(json!({ "status": "ok" })))
}

/// `POST /v1/auth/verify-email` — confirms an email via its one-time token.
/// Expired tokens answer 410 with "expired" in the detail (the dashboard
/// branches on that word); unknown/used tokens answer 400.
pub async fn verify_email(
    service: MaybeService,
    Json(request): Json<VerifyEmailRequest>,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    service
        .verify_email(&request.token)
        .await
        .map_err(Problem::from)?;

    Ok(ok_json(json!({ "status": "ok" })))
}

/// `POST /v1/auth/verify-email/resend` — re-issues the verification email.
/// Always 200 regardless of whether the account exists (no enumeration).
pub async fn resend_verification(
    service: MaybeService,
    Json(request): Json<ResendVerificationRequest>,
) -> Result<Response, Problem> {
    let service = require_service(service)?;
    let raw_token = service
        .resend_verification(&request.email)
        .await
        .map_err(Problem::from)?;

    let mut body = json!({ "status": "ok" });
    if let (true, Some(token)) = (service.exposes_dev_tokens(), raw_token) {
        body["verificationToken"] = json!(token);
    }
    Ok(ok_json(body))
}
