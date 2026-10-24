//! End-to-end HTTP tests for the devices feature (fake-backed services).
//!
//! Runs without a database. Follows the recipients_api flow: sign up + log
//! in, create a project via onboarding, then exercise the device
//! endpoints scoped to that project.

mod common;

use std::sync::Arc;

use crate::common::{FakeAuditStore, FakeAuthStore, FakeDevicesStore};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use server::api::build_router;
use server::api::state::AppState;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::devices::DeviceDestinationCatalog;
use server::domain::devices::DeviceService;
use server::domain::projects::ProjectService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use tower::ServiceExt;

/// App wired with fake auth + devices services.
fn app_with_devices() -> (Router, Arc<FakeDevicesStore>, Arc<FakeAuditStore>) {
    let store = Arc::new(FakeAuthStore::new());
    let audit_store = Arc::new(FakeAuditStore::new());
    let audit = Arc::new(AuditService::new(audit_store.clone()));
    let auth = Arc::new(AuthService::new(store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(store.clone(), audit.clone()));
    let devices_store = Arc::new(FakeDevicesStore::new());
    let mut channels = std::collections::HashMap::new();
    channels.insert("android".to_string(), vec!["fcm".to_string()]);
    channels.insert("apple".to_string(), vec!["apns".to_string()]);
    let devices = Arc::new(DeviceService::new(
        devices_store.clone(),
        audit.clone(),
        DeviceDestinationCatalog::new(channels),
    ));
    (
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
                projects: Some(projects),
                project_exports: None,
                project_imports: None,
                project_members: None,
                audit: Some(audit),
                recipients: None,
                devices: Some(devices),
                api_keys: None,
                templates: None,
                channel_providers: None,
                tickets: None,
                notifications: None,
                billing: None,
                schedules: None,
                webhooks: None,
                campaigns: None,
                inbound: None,
                provider_tester: Arc::new(ConfigProviderTester::new())
                    as Arc<dyn ProviderTester + Send + Sync>,
                config_store: Arc::new(server::infra::FileConfigStore::new("configs")),
                flows: None,
                sends: None,
            },
            &AppConfig::default(),
        ),
        devices_store,
        audit_store,
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn signup_and_login_on(app: Router, email: &str) -> (String, String) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Jane", "email": email, "password": "Sup3rSecret!"}).to_string(),
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
                .uri("/app/auth/login")
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
    let body = body_json(res).await;
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    let token = body["session"]["token"].as_str().unwrap().to_string();
    (token, user_id)
}

/// Creates a project for the session (onboarding-complete path, same as
/// the recipients tests — the id rides `GET /app/projects`).
async fn onboard_project_on(
    app: Router,
    devices_store: &Arc<FakeDevicesStore>,
    email: &str,
    project_name: &str,
) -> (String, String, String) {
    let (token, user_id) = signup_and_login_on(app.clone(), email).await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/onboarding/complete",
        Some(json!({"project": {"name": project_name}})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(app.clone(), "GET", "/app/projects", None, &token).await;
    let project_id = body_json(res).await["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    devices_store.seed_visible(&user_id, &project_id);
    (token, user_id, project_id)
}

async fn request_with_cookie(
    app: Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    cookie: &str,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", format!("session_token={cookie}"));
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    app.oneshot(
        builder
            .body(Body::from(body.map(|v| v.to_string()).unwrap_or_default()))
            .unwrap(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn register_creates_an_active_device_record() {
    let (app, devices, _) = app_with_devices();
    let (token, _user_id, project) =
        onboard_project_on(app.clone(), &devices, "devices@x.dev", "device proj").await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/devices"),
        Some(json!({
            "destination": "android",
            "vendor": "fcm",
            "token": "tok-123",
            "label": "QA Pixel 8",
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["device"]["destination"], "android");
    assert_eq!(body["device"]["vendor"], "fcm");
    assert_eq!(body["device"]["token"], "tok-123");
    assert_eq!(body["device"]["status"], "active");
}

#[tokio::test]
async fn re_registering_the_same_token_refreshes_instead_of_duplicating() {
    let (app, devices, _) = app_with_devices();
    let (token, _user_id, project) =
        onboard_project_on(app.clone(), &devices, "again@x.dev", "device proj").await;

    let uri = format!("/app/projects/{project}/devices");
    for body in [
        json!({"destination": "android", "vendor": "fcm", "token": "tok-9"}),
        json!({"destination": "android", "vendor": "fcm", "token": "tok-9", "label": "moved"}),
    ] {
        let res = request_with_cookie(app.clone(), "POST", &uri, Some(body), &token).await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    let res = request_with_cookie(app.clone(), "GET", &uri, None, &token).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["devices"].as_array().unwrap().len(), 1);
    assert_eq!(body["devices"][0]["label"], "moved");
}

#[tokio::test]
async fn register_rejects_pairs_outside_the_catalog() {
    let (app, devices, _) = app_with_devices();
    let (token, _user_id, project) =
        onboard_project_on(app.clone(), &devices, "reject@x.dev", "device proj").await;

    let uri = format!("/app/projects/{project}/devices");

    // Unknown destination (email is not a device family member).
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &uri,
        Some(json!({"destination": "email", "vendor": "fcm", "token": "t"})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert!(
        body_json(res).await["detail"]
            .as_str()
            .unwrap()
            .contains("unknown device destination"),
    );

    // Vendor the registry never lists under that destination.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &uri,
        Some(json!({"destination": "android", "vendor": "apns", "token": "t"})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    assert!(
        body_json(res).await["detail"]
            .as_str()
            .unwrap()
            .contains("not registered under"),
    );
}

#[tokio::test]
async fn register_rejects_an_empty_token() {
    let (app, devices, _) = app_with_devices();
    let (token, _user_id, project) =
        onboard_project_on(app.clone(), &devices, "empty@x.dev", "device proj").await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/devices"),
        Some(json!({"destination": "android", "vendor": "fcm", "token": "   "})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn foreign_projects_are_hidden_from_device_enumeration() {
    let (app, devices, _) = app_with_devices();
    let (token, _user_id, project) =
        onboard_project_on(app.clone(), &devices, "filter@x.dev", "device proj").await;
    let (other, _other_id) = signup_and_login_on(app.clone(), "stranger@x.dev").await;

    // The stranger's session can't enumerate our devices — the fake store
    // reports the project invisible and the service renders it a 404
    // (same no-enumeration contract as recipients).
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/devices"),
        None,
        &other,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    // And registering through another project's door fails the same way.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/devices"),
        Some(json!({"destination": "android", "vendor": "fcm", "token": "t"})),
        &other,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let uri = format!("/app/projects/{project}/devices");
    request_with_cookie(
        app.clone(),
        "POST",
        &uri,
        Some(json!({"destination": "android", "vendor": "fcm", "token": "t1"})),
        &token,
    )
    .await;
    request_with_cookie(
        app.clone(),
        "POST",
        &uri,
        Some(json!({"destination": "apple", "vendor": "apns", "token": "t2"})),
        &token,
    )
    .await;
    let res = request_with_cookie(app.clone(), "GET", &uri, None, &token).await;
    assert_eq!(body_json(res).await["devices"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn revoke_flips_status_and_keeps_the_row() {
    let (app, devices, audit) = app_with_devices();
    let (token, _user_id, project) =
        onboard_project_on(app.clone(), &devices, "revoke@x.dev", "device proj").await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/devices"),
        Some(json!({"destination": "android", "vendor": "fcm", "token": "tok-r"})),
        &token,
    )
    .await;
    let device_id = body_json(res).await["device"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/devices/{device_id}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/devices/{device_id}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["device"]["status"], "revoked");

    // Audit captured both lifecycle events.
    let actions: Vec<String> = audit
        .all()
        .into_iter()
        .map(|e| e.event_type.to_string())
        .collect();
    assert!(
        actions.iter().any(|a| a == "device.registered"),
        "{actions:?}"
    );
    assert!(actions.iter().any(|a| a == "device.revoked"), "{actions:?}");
}

#[tokio::test]
async fn unknown_device_id_answers_404() {
    let (app, devices, _) = app_with_devices();
    let (token, _user_id, project) =
        onboard_project_on(app.clone(), &devices, "missed@x.dev", "device proj").await;

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/devices/dev_nope"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/devices/dev_nope"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
