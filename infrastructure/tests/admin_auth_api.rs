//! End-to-end HTTP tests for admin password recovery + TOTP setup semantics.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use server::api::build_router;
use server::api::state::AppState;
use server::domain::admin::AdminService;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::projects::ProjectService;
use server::domain::support::TicketService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeTicketsStore};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app_with_admin() -> Router {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(Box::new(FakeAdminStore::new()), true, audit.clone()));
    let tickets = Arc::new(TicketService::new(Arc::new(FakeTicketsStore::new()), audit.clone()));
    build_router(
        AppState {
            db: None,
            redis: None,
            auth: Some(auth),
            oauth: None,
            admin: Some(admin),
            admin_users: None,
            admin_projects: None,
            admin_notifications: None,
            projects: Some(projects),
            audit: Some(audit),
            recipients: None,
            templates: None,
            channel_providers: None,
            tickets: Some(tickets),
            notifications: None,
            provider_tester: Arc::new(ConfigProviderTester::new()) as Arc<dyn ProviderTester + Send + Sync>,
        },
        &AppConfig::default(),
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn session_cookie_from(res: &axum::response::Response) -> String {
    res.headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .strip_prefix("admin_session=")
        .unwrap_or("")
        .to_string()
}

async fn post(app: Router, uri: &str, body: Value, cookie: &str) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if !cookie.is_empty() {
        builder = builder.header("cookie", format!("admin_session={cookie}"));
    }
    app.oneshot(
        builder
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn bootstrap(app: Router) -> String {
    let res = post(
        app,
        "/admin/bootstrap",
        json!({"name": "Root", "email": "root@notifi.dev", "password": "Adm1n!Pass"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    session_cookie_from(&res)
}

#[tokio::test]
async fn admin_password_reset_flow() {
    let app = app_with_admin();
    let _cookie = bootstrap(app.clone()).await;

    // Forgot always answers 200, even for unknown addresses.
    let res = post(
        app.clone(),
        "/admin/password/forgot",
        json!({"email": "nobody@notifi.dev"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.get("resetToken").is_none());

    // Known address yields a dev token.
    let res = post(
        app.clone(),
        "/admin/password/forgot",
        json!({"email": "root@notifi.dev"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let token = body["resetToken"].as_str().unwrap().to_string();

    // Weak replacement password is rejected.
    let res = post(
        app.clone(),
        "/admin/password/reset",
        json!({"token": token, "password": "weak"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Reset works and the new password logs in.
    let res = post(
        app.clone(),
        "/admin/password/reset",
        json!({"token": token, "password": "N3w!Passw0rd"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = post(
        app.clone(),
        "/admin/login",
        json!({"email": "root@notifi.dev", "password": "N3w!Passw0rd"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // The token is single-use.
    let res = post(
        app.clone(),
        "/admin/password/reset",
        json!({"token": token, "password": "An0ther!Pass"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn admin_reset_with_unknown_token_is_400() {
    let app = app_with_admin();
    let _cookie = bootstrap(app.clone()).await;

    let res = post(
        app.clone(),
        "/admin/password/reset",
        json!({"token": "not-a-real-token", "password": "N3w!Passw0rd"}),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn admin_totp_setup_reuses_pending_secret() {
    let app = app_with_admin();
    let cookie = bootstrap(app.clone()).await;

    let res = post(app.clone(), "/admin/totp/setup", json!({}), &cookie).await;
    assert_eq!(res.status(), StatusCode::OK);
    let first = body_json(res).await["totpSecret"].as_str().unwrap().to_string();

    // Second call without ?regenerate returns the same pending secret.
    let res = post(app.clone(), "/admin/totp/setup", json!({}), &cookie).await;
    assert_eq!(res.status(), StatusCode::OK);
    let second = body_json(res).await["totpSecret"].as_str().unwrap().to_string();
    assert_eq!(first, second);

    // Explicit rotation issues a fresh secret.
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/totp/setup?regenerate=true")
                .header("content-type", "application/json")
                .header("cookie", format!("admin_session={cookie}"))
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let rotated = body_json(res).await["totpSecret"].as_str().unwrap().to_string();
    assert_ne!(first, rotated);
}
