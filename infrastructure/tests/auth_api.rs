//! End-to-end HTTP tests for the auth feature (fake-backed service).
//!
//! Runs without a database: the router is built with an in-memory
//! [`FakeAuthStore`], exercising the full presentation → application path.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use server::api::build_router;
use server::api::state::AppState;
use server::domain::auth::{
    AuthToken as AuthTokenEntity, AuthTokenId, AuthService, TokenPurpose, hash_token,
};
use server::ports::auth_store::{AuthStore, BoxFut};
use server::ports::oauth::{
    AuthorizeStart, OAuthError, OAuthIdentityProvider, OAuthProfile, OAuthRuntime,
};
use server::testing::FakeAuthStore;
use serde_json::{Value, json};
use tower::ServiceExt;

use server::infra::config::AppConfig;

/// App wired with a fake-backed auth service (dev tokens exposed).
fn app() -> (Router, Arc<FakeAuthStore>) {
    let store = Arc::new(FakeAuthStore::new());
    let auth = Arc::new(AuthService::new(store.clone(), true));
    (
        build_router(
            AppState {
                db: None,
                redis: None,
                auth: Some(auth),
                oauth: None,
            },
            &AppConfig::default(),
        ),
        store,
    )
}

/// App with no services configured — protected routes must answer 503.
fn app_without_auth() -> Router {
    build_router(
        AppState {
            db: None,
            redis: None,
            auth: None,
            oauth: None,
        },
        &AppConfig::default(),
    )
}

/// Same, plus the OAuth runtime wired through `AppState` — the exact
/// production path (`v1_router`'s conditional Extension layer), so a layer/
/// extractor type mismatch cannot hide here.
fn app_with_oauth() -> (Router, Arc<FakeAuthStore>) {
    let store = Arc::new(FakeAuthStore::new());
    let auth = Arc::new(AuthService::new(store.clone(), true));
    let oauth = Arc::new(OAuthRuntime {
        provider: Arc::new(StubOAuthProvider),
        dashboard_url: "http://localhost:3000".to_string(),
    });
    (
        build_router(
            AppState {
                db: None,
                redis: None,
                auth: Some(auth),
                oauth: Some(oauth),
            },
            &AppConfig::default(),
        ),
        store,
    )
}

struct StubOAuthProvider;

impl OAuthIdentityProvider for StubOAuthProvider {
    fn authorize_url(
        &self,
        _provider: &str,
        csrf_state: &str,
    ) -> Result<AuthorizeStart, OAuthError> {
        Ok(AuthorizeStart {
            url: format!("https://provider.example/authorize?state={csrf_state}"),
            pkce_verifier: "test-verifier".to_string(),
        })
    }

    fn exchange_code(
        &self,
        _provider: &str,
        _code: &str,
        _pkce_verifier: &str,
    ) -> BoxFut<'_, Result<OAuthProfile, OAuthError>> {
        Box::pin(async move {
            Ok(OAuthProfile {
                subject: "subject-1".to_string(),
                email: "oauth.user@example.com".to_string(),
                email_verified: true,
                name: Some("OAuth User".to_string()),
                avatar_url: None,
            })
        })
    }
}

async fn post_json(app: Router, uri: &str, body: Value) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn post_json_with_cookie(
    app: Router,
    uri: &str,
    body: Value,
    cookie: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn get_with_cookie_on(
    app: Router,
    uri: &str,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    app.oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// All `set-cookie` headers of a response as raw strings.
fn set_cookies(response: &axum::response::Response) -> Vec<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|value| value.to_str().ok().map(str::to_string))
        .collect()
}

/// Value of `<name>=…` from a list of raw set-cookie strings.
fn cookie_value(cookies: &[String], name: &str) -> Option<String> {
    cookies
        .iter()
        .find_map(|c| c.split(';').next().and_then(|pair| pair.strip_prefix(&format!("{name}=")).map(str::to_string)))
}

/// Signs a user up and logs them in on `app`; returns (raw session token, user id).
async fn signup_and_login_on(app: Router, email: &str) -> (String, String) {
    let res = post_json(
        app.clone(),
        "/v1/auth/signup",
        json!({"name": "Jane", "email": email, "password": "Sup3rSecret!"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = post_json(
        app,
        "/v1/auth/login",
        json!({"email": email, "password": "Sup3rSecret!", "rememberMe": false}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    let token = body["session"]["token"].as_str().unwrap().to_string();
    (token, user_id)
}

#[tokio::test]
async fn signup_returns_201_with_user_and_dev_verification_token() {
    let (app, _store) = app();
    let res = post_json(
        app,
        "/v1/auth/signup",
        json!({"name": "Jane", "email": "jane@example.com", "password": "Sup3rSecret!"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    // signup starts a session: the httpOnly cookie is set right away
    let set_cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();
    assert!(set_cookie.starts_with("session_token="));
    assert!(set_cookie.contains("HttpOnly"));

    let body = body_json(res).await;
    assert_eq!(body["user"]["email"], "jane@example.com");
    assert!(!body["user"]["emailVerified"].as_bool().unwrap());
    assert!(body["session"]["token"].is_string());
    assert!(body["session"]["expiresAt"].is_string());
    assert!(body["verificationToken"].is_string());
}

#[tokio::test]
async fn signup_session_authenticates_without_login() {
    let (app, _store) = app();
    let res = post_json(
        app.clone(),
        "/v1/auth/signup",
        json!({"name": "Jane", "email": "nosession@example.com", "password": "Sup3rSecret!"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    let cookie = body["session"]["token"].as_str().unwrap().to_string();

    // the signup cookie alone is enough for the protected endpoint
    let res = get_with_cookie_on(app, "/v1/auth/me", Some(&cookie)).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["user"]["email"],
        "nosession@example.com"
    );
}

#[tokio::test]
async fn onboarding_complete_flips_the_flag_and_is_idempotent() {
    let (app, _store) = app();
    let (token, _uid) = signup_and_login_on(app.clone(), "onboard@example.com").await;

    // fresh account: flag false in login payload and /me
    let res = post_json(
        app.clone(),
        "/v1/auth/login",
        json!({"email": "onboard@example.com", "password": "Sup3rSecret!", "rememberMe": false}),
    )
    .await;
    assert_eq!(body_json(res).await["session"]["onboardingCompleted"], false);
    let res = get_with_cookie_on(app.clone(), "/v1/auth/me", Some(&token)).await;
    assert_eq!(body_json(res).await["onboardingCompleted"], false);

    // completing onboarding persists org + project
    let payload = json!({
        "organization": {"name": "Acme Corp", "region": "us-east-1", "timezone": "America/New_York"},
        "project": {"name": "My App", "description": "first project", "environment": "development"},
    });
    let res = post_json_with_cookie(
        app.clone(),
        "/v1/auth/onboarding/complete",
        payload.clone(),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = get_with_cookie_on(app.clone(), "/v1/auth/me", Some(&token)).await;
    assert_eq!(body_json(res).await["onboardingCompleted"], true);

    // next login reports the completed flag
    let res = post_json(
        app.clone(),
        "/v1/auth/login",
        json!({"email": "onboard@example.com", "password": "Sup3rSecret!", "rememberMe": false}),
    )
    .await;
    assert_eq!(body_json(res).await["session"]["onboardingCompleted"], true);

    // idempotent replay
    let res = post_json_with_cookie(app, "/v1/auth/onboarding/complete", payload, &token)
        .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["alreadyCompleted"], true);
}

#[tokio::test]
async fn invited_member_with_org_and_project_skips_onboarding() {
    let (app, store) = app();

    // simulate an invitation accepted before first sign-in: the member row
    // exists under an org that already has a project.
    let res = post_json(
        app.clone(),
        "/v1/auth/signup",
        json!({"name": "Guest", "email": "invited@example.com", "password": "Sup3rSecret!"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    store.seed_onboarded(user_id.parse().unwrap());

    let res = post_json(
        app,
        "/v1/auth/login",
        json!({"email": "invited@example.com", "password": "Sup3rSecret!", "rememberMe": false}),
    )
    .await;
    assert_eq!(body_json(res).await["session"]["onboardingCompleted"], true);
}

#[tokio::test]
async fn signup_rejects_duplicates_weak_passwords_and_bad_emails() {
    let (app, _store) = app();
    let payload = json!({"name": "Jane", "email": "dup@example.com", "password": "Sup3rSecret!"});
    assert_eq!(
        post_json(app.clone(), "/v1/auth/signup", payload.clone())
            .await
            .status(),
        StatusCode::CREATED
    );

    let res = post_json(app, "/v1/auth/signup", payload).await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
    let body = body_json(res).await;
    assert_eq!(body["detail"], "An account with this email already exists.");
}

#[tokio::test]
async fn login_sets_session_cookie_and_returns_contract_shape() {
    let (app, _store) = app();
    post_json(
        app.clone(),
        "/v1/auth/signup",
        json!({"name": "Jane", "email": "cookie@example.com", "password": "Sup3rSecret!"}),
    )
    .await;

    // wrong password first
    let res = post_json(
        app.clone(),
        "/v1/auth/login",
        json!({"email": "cookie@example.com", "password": "WrongPass1!", "rememberMe": false}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(res).await["detail"], "Invalid email or password.");

    // correct password
    let res = post_json(
        app,
        "/v1/auth/login",
        json!({"email": "cookie@example.com", "password": "Sup3rSecret!", "rememberMe": true}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let set_cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();
    assert!(set_cookie.starts_with("session_token="));
    assert!(set_cookie.contains("HttpOnly"));

    let body = body_json(res).await;
    let cookie_value = set_cookie
        .split(';')
        .next()
        .unwrap()
        .trim_start_matches("session_token=")
        .to_string();
    assert_eq!(body["session"]["token"], cookie_value.as_str());
    assert!(body["session"]["expiresAt"].is_string());
}

#[tokio::test]
async fn me_requires_and_honors_the_session_cookie() {
    let (app, _store) = app();
    let (token, _uid) = signup_and_login_on(app.clone(), "me@example.com").await;

    // no cookie → 401 problem doc
    let res = get_with_cookie_on(app.clone(), "/v1/auth/me", None).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // valid cookie → current user
    let res = get_with_cookie_on(app, "/v1/auth/me", Some(&token)).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["user"]["email"], "me@example.com");
}

#[tokio::test]
async fn verify_email_consumes_once_and_reports_expiry_properly() {
    let (app, store) = app();

    let res = post_json(
        app.clone(),
        "/v1/auth/signup",
        json!({"name": "Jane", "email": "verify@example.com", "password": "Sup3rSecret!"}),
    )
    .await;
    let token = body_json(res).await["verificationToken"]
        .as_str()
        .unwrap()
        .to_string();

    // first use succeeds
    let res = post_json(
        app.clone(),
        "/v1/auth/verify-email",
        json!({"token": token}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // replay is rejected as invalid (not expired)
    let res = post_json(
        app.clone(),
        "/v1/auth/verify-email",
        json!({"token": token}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert!(
        body_json(res).await["detail"]
            .as_str()
            .unwrap()
            .contains("already been used")
    );

    // a genuinely expired link answers 410 with the load-bearing word
    let raw2 = "expired-raw-token";
    let (raw2_real, _) = server::domain::auth::new_token();
    let _ = raw2;
    store.seed_token(AuthTokenEntity {
        id: AuthTokenId::new(),
        user_id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap(),
        purpose: TokenPurpose::EmailVerification,
        token_hash: hash_token(&raw2_real),
        expires_at: Utc::now() - Duration::hours(1),
        consumed_at: None,
        created_at: Utc::now() - Duration::hours(25),
    });
    let res = post_json(app, "/v1/auth/verify-email", json!({"token": raw2_real})).await;
    assert_eq!(res.status(), StatusCode::GONE);
    assert!(
        body_json(res).await["detail"]
            .as_str()
            .unwrap()
            .contains("expired")
    );
}

#[tokio::test]
async fn password_reset_flow_rotates_and_revokes_sessions() {
    let (app, _store) = app();
    let (old_session, _uid) = signup_and_login_on(app.clone(), "reset@example.com").await;

    // forgot always answers 200, even for unknown emails (no enumeration)
    let res = post_json(
        app.clone(),
        "/v1/auth/password/forgot",
        json!({"email": "nobody@example.com"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.get("resetToken").is_none());

    // known email exposes the dev reset token
    let res = post_json(
        app.clone(),
        "/v1/auth/password/forgot",
        json!({"email": "reset@example.com"}),
    )
    .await;
    let reset_token = body_json(res).await["resetToken"]
        .as_str()
        .unwrap()
        .to_string();

    // weak replacement rejected
    let res = post_json(
        app.clone(),
        "/v1/auth/password/reset",
        json!({"token": reset_token, "password": "weak"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // strong replacement works and revokes prior sessions
    let res = post_json(
        app.clone(),
        "/v1/auth/password/reset",
        json!({"token": reset_token, "password": "N3wPassword!"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = get_with_cookie_on(app.clone(), "/v1/auth/me", Some(&old_session)).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // login with the new password succeeds
    let res = post_json(
        app,
        "/v1/auth/login",
        json!({"email": "reset@example.com", "password": "N3wPassword!", "rememberMe": false}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn logout_revokes_and_clears_the_cookie() {
    let app = app().0;
    let (token, _uid) = {
        let res = post_json(
            app.clone(),
            "/v1/auth/signup",
            json!({"name": "Jane", "email": "logout@example.com", "password": "Sup3rSecret!"}),
        )
        .await;
        assert_eq!(res.status(), StatusCode::CREATED);
        let res = post_json(
            app.clone(),
            "/v1/auth/login",
            json!({"email": "logout@example.com", "password": "Sup3rSecret!", "rememberMe": false}),
        )
        .await;
        let body = body_json(res).await;
        (
            body["session"]["token"].as_str().unwrap().to_string(),
            String::new(),
        )
    };

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/logout")
                .header("cookie", format!("session_token={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // session is gone
    let res = get_with_cookie_on(app, "/v1/auth/me", Some(&token)).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn auth_routes_answer_503_when_unconfigured() {
    let res = post_json(
        app_without_auth(),
        "/v1/auth/signup",
        json!({"name": "Jane", "email": "x@example.com", "password": "Sup3rSecret!"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    let content_type = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    assert!(content_type.starts_with("application/problem+json"));
}

// -- oauth ------------------------------------------------------------------

#[tokio::test]
async fn oauth_start_redirects_and_parks_temp_cookies() {
    let (app, _store) = app_with_oauth();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/auth/oauth/github?popup=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);

    let location = res.headers().get("location").and_then(|v| v.to_str().ok());
    assert!(
        location
            .unwrap()
            .starts_with("https://provider.example/authorize?")
    );

    let cookies = set_cookies(&res);
    assert!(cookie_value(&cookies, "oauth_state_github").is_some());
    assert!(
        cookies
            .iter()
            .any(|c| c.starts_with("oauth_mode_github=popup"))
    );
}

#[tokio::test]
async fn oauth_callback_creates_user_sets_session_and_messages_opener() {
    let (app, _store) = app_with_oauth();

    // start the flow to obtain the parked state cookie
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/auth/oauth/github?popup=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let cookies = set_cookies(&res);
    let state = cookie_value(&cookies, "oauth_state_github").unwrap();
    let verifier = cookie_value(&cookies, "oauth_verifier_github").unwrap();

    // provider calls back with code + state; a real browser replays every
    // cookie the start response parked on the callback path
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/auth/oauth/github/callback?code=the-code&state={state}"
                ))
                .header(
                    "cookie",
                    format!(
                        "oauth_state_github={state}; oauth_verifier_github={verifier}; oauth_mode_github=popup"
                    ),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // …the session cookie authenticates immediately
    let session = set_cookies(&res);
    assert!(
        session
            .iter()
            .any(|c| c.starts_with("session_token=") && c.contains("HttpOnly"))
    );

    // …and the popup page hands a success message to the opener
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&bytes).to_string();
    assert!(html.contains("oauth:success"), "html was: {html}");
    assert!(html.contains("postMessage"));
}

#[tokio::test]
async fn oauth_callback_links_matching_password_account() {
    let (app, store) = app_with_oauth();

    // an existing password account with the email the OAuth profile will use
    let res = post_json(
        app.clone(),
        "/v1/auth/signup",
        json!({"name": "Original", "email": "oauth.user@example.com", "password": "Sup3rSecret!"}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let original_id = body_json(res).await["user"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/auth/oauth/github")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let cookies = set_cookies(&res);
    let state = cookie_value(&cookies, "oauth_state_github").unwrap();
    let verifier = cookie_value(&cookies, "oauth_verifier_github").unwrap();

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/auth/oauth/github/callback?code=c&state={state}"
                ))
                .header(
                    "cookie",
                    format!("oauth_state_github={state}; oauth_verifier_github={verifier}"),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let location = res.headers().get("location").and_then(|v| v.to_str().ok());
    assert_eq!(location, Some("http://localhost:3000"));

    // no duplicate account was created — the same user id comes back
    let found = store
        .find_user_by_email("oauth.user@example.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id.to_string(), original_id);
    assert_eq!(found.oauth_provider.as_deref(), Some("github"));
}

#[tokio::test]
async fn oauth_callback_rejects_state_mismatch() {
    let (app, _store) = app_with_oauth();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/auth/oauth/github/callback?code=c&state=tampered")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SEE_OTHER);
    let location = res.headers().get("location").and_then(|v| v.to_str().ok());
    assert!(location.unwrap().contains("/auth/login?error=oauth_failed"));
}

#[tokio::test]
async fn oauth_routes_answer_503_without_runtime() {
    let (app, _store) = app();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/auth/oauth/github")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}
