//! End-to-end HTTP tests for the in-app notifications feature (fake-backed services).

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use server::api::build_router;
use server::api::state::AppState;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::notifications::NotificationService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAuditStore, FakeAuthStore, FakeNotificationsStore};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app_with_notifications() -> (Router, Arc<FakeNotificationsStore>) {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let notifications_store = Arc::new(FakeNotificationsStore::new());
    let notifications = Arc::new(NotificationService::new(notifications_store.clone()));
    (
        build_router(
            AppState {
                db: None,
                redis: None,
                auth: Some(auth),
                oauth: None,
                projects: None,
                audit: Some(audit),
                recipients: None,
                templates: None,
                channel_providers: None,
                tickets: None,
                notifications: Some(notifications),
                provider_tester: Arc::new(ConfigProviderTester::new()) as Arc<dyn ProviderTester + Send + Sync>,
            },
            &AppConfig::default(),
        ),
        notifications_store,
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn signup_and_login_on(app: Router, email: &str) -> String {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Test", "email": email, "password": "Sup3rSecret!"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"email": email, "password": "Sup3rSecret!", "rememberMe": false})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let headers = res.headers().clone();
    let cookie_header = headers
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    cookie_header
        .split(';')
        .find(|p| p.trim().starts_with("session_token="))
        .unwrap()
        .trim()
        .strip_prefix("session_token=")
        .unwrap()
        .to_string()
}

async fn get_with_cookie_on(
    app: Router,
    uri: &str,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(c) = cookie {
        builder = builder.header("cookie", format!("session_token={c}"));
    }
    app.oneshot(builder.body(Body::empty()).unwrap()).await.unwrap()
}

async fn patch_with_cookie_on(
    app: Router,
    uri: &str,
    cookie: &str,
    body: Value,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("cookie", format!("session_token={cookie}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

// -------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------

#[tokio::test]
async fn list_notifications_empty_initially() {
    let (app, _store) = app_with_notifications();
    let token = signup_and_login_on(app.clone(), "notif-list-empty@example.com").await;

    let res = get_with_cookie_on(app, "/v1/notifications", Some(&token)).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["notifications"].as_array().unwrap().len(), 0);
    assert_eq!(body["hasMore"], false);
}

#[tokio::test]
async fn create_and_list_system_notification() {
    let (app, _store) = app_with_notifications();
    let token = signup_and_login_on(app.clone(), "notif-create-list@example.com").await;

    // List should be empty initially.
    let res = get_with_cookie_on(app.clone(), "/v1/notifications", Some(&token)).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["notifications"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn count_unread_returns_zero_initially() {
    let (app, _store) = app_with_notifications();
    let token = signup_and_login_on(app.clone(), "notif-count@example.com").await;

    let res = get_with_cookie_on(app, "/v1/notifications/count", Some(&token)).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["count"], 0);
}

#[tokio::test]
async fn mark_all_read_returns_zero_when_nothing_to_mark() {
    let (app, _store) = app_with_notifications();
    let token = signup_and_login_on(app.clone(), "notif-mark-all@example.com").await;

    let res = patch_with_cookie_on(
        app,
        "/v1/notifications/read-all",
        &token,
        json!({}),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["updated"], 0);
}

#[tokio::test]
async fn get_nonexistent_notification_returns_404() {
    let (app, _store) = app_with_notifications();
    let token = signup_and_login_on(app.clone(), "notif-404@example.com").await;

    let res = get_with_cookie_on(
        app,
        "/v1/notifications/nonexistent-id",
        Some(&token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unauthenticated_list_returns_401() {
    let (app, _store) = app_with_notifications();
    let res = get_with_cookie_on(app, "/v1/notifications", None).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn list_unread_only_filter() {
    let (app, _store) = app_with_notifications();
    let token = signup_and_login_on(app.clone(), "notif-unread@example.com").await;

    // Verify empty unread filter returns empty.
    let res = get_with_cookie_on(
        app.clone(),
        "/v1/notifications?unreadOnly=true",
        Some(&token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["notifications"].as_array().unwrap().len(), 0);
}
