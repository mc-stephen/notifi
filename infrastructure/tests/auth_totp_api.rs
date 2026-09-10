//! End-to-end HTTP tests for user TOTP two-factor authentication.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use server::api::build_router;
use server::api::state::AppState;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAuditStore, FakeAuthStore};
use totp_rs::{Rfc6238, Secret, TOTP};
use tower::ServiceExt;

fn app() -> Router {
    let store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(store, true, audit.clone()));
    build_router(
        AppState {
            db: None,
            redis: None,
            auth: Some(auth),
            oauth: None,
            admin: None,
            admin_users: None,
            admin_projects: None,
            admin_notifications: None,
            projects: None,
            project_members: None,
            audit: Some(audit),
            recipients: None,
            templates: None,
            channel_providers: None,
            tickets: None,
            notifications: None,
            billing: None,
            provider_tester: std::sync::Arc::new(ConfigProviderTester::new())
                as std::sync::Arc<dyn ProviderTester + Send + Sync>,
        },
        &AppConfig::default(),
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn post(
    app: Router,
    uri: &str,
    body: Value,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    app.oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn get(app: Router, uri: &str, cookie: Option<&str>) -> axum::response::Response {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    app.oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// A currently-valid TOTP code for a base32 secret.
fn current_code(secret_base32: &str) -> String {
    let raw = Secret::Encoded(secret_base32.to_string()).to_raw().unwrap();
    let rfc = Rfc6238::with_defaults(raw.to_bytes().unwrap()).unwrap();
    TOTP::from_rfc6238(rfc).unwrap().generate_current().unwrap()
}

async fn signup(app: Router, email: &str) -> String {
    let res = post(
        app,
        "/app/auth/signup",
        json!({"name": "Tess", "email": email, "password": "Sup3rSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["user"]["totpEnabled"], false);
    body["session"]["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn totp_lifecycle_gates_login() {
    let app = app();
    let token = signup(app.clone(), "totp@x.dev").await;

    // Setup returns a secret + URI.
    let res = post(app.clone(), "/app/auth/totp/setup", json!({}), Some(&token)).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let secret = body["totpSecret"].as_str().unwrap().to_string();
    assert!(body["totpUri"].as_str().unwrap().starts_with("otpauth://"));

    // Wrong code is rejected.
    let res = post(
        app.clone(),
        "/app/auth/totp/verify",
        json!({"code": "000000"}),
        Some(&token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Right code enables 2FA.
    let res = post(
        app.clone(),
        "/app/auth/totp/verify",
        json!({"code": current_code(&secret)}),
        Some(&token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = get(app.clone(), "/app/auth/me", Some(&token)).await;
    assert_eq!(body_json(res).await["user"]["totpEnabled"], true);

    // Password login without a code now asks for the second step.
    let res = post(
        app.clone(),
        "/app/auth/login",
        json!({"email": "totp@x.dev", "password": "Sup3rSecret!", "rememberMe": false}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["requiresTotp"], true);

    // Wrong code fails; right code (single call) signs in.
    let res = post(
        app.clone(),
        "/app/auth/login",
        json!({"email": "totp@x.dev", "password": "Sup3rSecret!", "totpCode": "000000"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = post(
        app.clone(),
        "/app/auth/login",
        json!({
            "email": "totp@x.dev",
            "password": "Sup3rSecret!",
            "totpCode": current_code(&secret),
        }),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let fresh = body_json(res).await["session"]["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Challenge endpoint completes login with email + code only.
    let res = post(
        app.clone(),
        "/app/auth/totp/challenge",
        json!({"email": "totp@x.dev", "code": current_code(&secret)}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Disable needs the password plus a valid code.
    let res = post(
        app.clone(),
        "/app/auth/totp/disable",
        json!({"password": "WrongPass1!", "code": current_code(&secret)}),
        Some(&fresh),
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = post(
        app.clone(),
        "/app/auth/totp/disable",
        json!({"password": "Sup3rSecret!", "code": current_code(&secret)}),
        Some(&fresh),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Plain login works again, and setup can start over.
    let res = post(
        app.clone(),
        "/app/auth/login",
        json!({"email": "totp@x.dev", "password": "Sup3rSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.get("requiresTotp").is_none());
}

#[tokio::test]
async fn totp_setup_refused_while_enabled() {
    let app = app();
    let token = signup(app.clone(), "totp2@x.dev").await;

    let res = post(app.clone(), "/app/auth/totp/setup", json!({}), Some(&token)).await;
    let secret = body_json(res).await["totpSecret"]
        .as_str()
        .unwrap()
        .to_string();
    let res = post(
        app.clone(),
        "/app/auth/totp/verify",
        json!({"code": current_code(&secret)}),
        Some(&token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Setting up again while enabled is refused.
    let res = post(app.clone(), "/app/auth/totp/setup", json!({}), Some(&token)).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Verifying again is refused too.
    let res = post(
        app.clone(),
        "/app/auth/totp/verify",
        json!({"code": current_code(&secret)}),
        Some(&token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
