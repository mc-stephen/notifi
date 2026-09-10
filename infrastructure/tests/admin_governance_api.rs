//! End-to-end HTTP tests for admin change-password + super-admin governance.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
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
use tower::ServiceExt;

fn app_with_admin() -> Router {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(
        Box::new(FakeAdminStore::new()),
        true,
        audit.clone(),
    ));
    let tickets = Arc::new(TicketService::new(
        Arc::new(FakeTicketsStore::new()),
        audit.clone(),
    ));
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
            project_members: None,
            audit: Some(audit),
            recipients: None,
            templates: None,
            channel_providers: None,
            tickets: Some(tickets),
            notifications: None,
            billing: None,
            provider_tester: Arc::new(ConfigProviderTester::new())
                as Arc<dyn ProviderTester + Send + Sync>,
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
        .header("content-type", "application/json");
    if !cookie.is_empty() {
        builder = builder.header("cookie", format!("admin_session={cookie}"));
    }
    let body = body
        .map(|b| Body::from(b.to_string()))
        .unwrap_or_else(Body::empty);
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

async fn bootstrap(app: Router) -> (String, String) {
    let res = request_with_cookie(
        app,
        "POST",
        "/admin/bootstrap",
        Some(json!({"name": "Root", "email": "root@notifi.dev", "password": "Adm1n!Pass"})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let cookie = session_cookie_from(&res);
    let body = body_json(res).await;
    (cookie, body["userId"].as_str().unwrap().to_string())
}

async fn login_cookie(app: Router, email: &str, password: &str) -> (StatusCode, String) {
    let res = request_with_cookie(
        app,
        "POST",
        "/admin/login",
        Some(json!({"email": email, "password": password})),
        "",
    )
    .await;
    (res.status(), session_cookie_from(&res))
}

async fn create_admin_as(
    app: Router,
    cookie: &str,
    name: &str,
    email: &str,
) -> (StatusCode, Value) {
    let res = request_with_cookie(
        app,
        "POST",
        "/admin/admins",
        Some(json!({"name": name, "email": email, "password": "Oth3r!Pass"})),
        cookie,
    )
    .await;
    let status = res.status();
    (status, body_json(res).await)
}

#[tokio::test]
async fn bootstrap_admin_is_super() {
    let app = app_with_admin();
    let (cookie, _) = bootstrap(app.clone()).await;

    let res = request_with_cookie(app.clone(), "GET", "/admin/me", None, &cookie).await;
    let body = body_json(res).await;
    assert_eq!(body["isSuperAdmin"], true);
    assert_eq!(body["status"], "active");

    let res = request_with_cookie(app.clone(), "GET", "/admin/admins", None, &cookie).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let admins = body["admins"].as_array().unwrap().to_owned();
    assert_eq!(admins.len(), 1);
    assert_eq!(admins[0]["isSuperAdmin"], true);
    assert_eq!(body["total"], 1);
    assert_eq!(body["totalPages"], 1);
    assert_eq!(body["page"], 1);

    // Second page is empty but well-formed.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/admins?per_page=1&page=2",
        None,
        &cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["admins"].as_array().unwrap().len(), 0);
    assert_eq!(body["total"], 1);
    assert_eq!(body["page"], 2);
}

#[tokio::test]
async fn change_password_roundtrip() {
    let app = app_with_admin();
    let (cookie, _) = bootstrap(app.clone()).await;

    // Wrong current password.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/password/change",
        Some(json!({"currentPassword": "nope", "newPassword": "N3w!Passw0rd"})),
        &cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Weak replacement.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/password/change",
        Some(json!({"currentPassword": "Adm1n!Pass", "newPassword": "weak"})),
        &cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Success rotates sessions but keeps the caller signed in.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/password/change",
        Some(json!({"currentPassword": "Adm1n!Pass", "newPassword": "N3w!Passw0rd"})),
        &cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let fresh_cookie = session_cookie_from(&res);
    assert!(!fresh_cookie.is_empty());

    let res = request_with_cookie(app.clone(), "GET", "/admin/me", None, &fresh_cookie).await;
    assert_eq!(res.status(), StatusCode::OK);

    // Old password dead, old session dead.
    let (status, _) = login_cookie(app.clone(), "root@notifi.dev", "Adm1n!Pass").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let res = request_with_cookie(app.clone(), "GET", "/admin/me", None, &cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // New password works.
    let (status, _) = login_cookie(app.clone(), "root@notifi.dev", "N3w!Passw0rd").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn only_super_admin_can_create_admins() {
    let app = app_with_admin();
    let (super_cookie, _) = bootstrap(app.clone()).await;

    // Super creates directly: active immediately.
    let (status, _) = create_admin_as(app.clone(), &super_cookie, "Second", "second@x.dev").await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, second_cookie) = login_cookie(app.clone(), "second@x.dev", "Oth3r!Pass").await;
    assert_eq!(status, StatusCode::OK);
    assert!(!second_cookie.is_empty());

    // Non-super creation is forbidden outright.
    let (status, _) = create_admin_as(app.clone(), &second_cookie, "Third", "third@x.dev").await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Approval endpoints are gone entirely.
    let res =
        request_with_cookie(app.clone(), "GET", "/admin/approvals", None, &second_cookie).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/approvals?status=pending",
        None,
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn removal_is_super_only_with_protection() {
    let app = app_with_admin();
    let (super_cookie, super_id) = bootstrap(app.clone()).await;
    let (_, body) = create_admin_as(app.clone(), &super_cookie, "Second", "second@x.dev").await;
    let second_id = body["admin"]["id"].as_str().unwrap().to_string();
    let (_, second_cookie) = login_cookie(app.clone(), "second@x.dev", "Oth3r!Pass").await;

    // Super admin cannot be removed — by anyone.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/admins/{super_id}/remove"),
        None,
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Non-super callers are rejected before any other check (even self-removal).
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/admins/{second_id}/remove"),
        None,
        &second_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Non-super removal is forbidden outright (no request flow) —
    // authz runs before target checks.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/admins/{super_id}/remove"),
        None,
        &second_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let (_, body) = create_admin_as(app.clone(), &super_cookie, "Third", "third@x.dev").await;
    let third_id = body["admin"]["id"].as_str().unwrap().to_string();

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/admins/{third_id}/remove"),
        None,
        &second_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // No remove-kind requests exist anymore (endpoint is gone).
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/approvals?status=pending",
        None,
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Super removes directly: login dead, list hides.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/admins/{third_id}/remove"),
        None,
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let (status, _) = login_cookie(app.clone(), "third@x.dev", "Oth3r!Pass").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let res = request_with_cookie(app.clone(), "GET", "/admin/admins", None, &super_cookie).await;
    let admins = body_json(res).await["admins"]
        .as_array()
        .unwrap()
        .to_owned();
    assert!(!admins.iter().any(|a| a["id"] == third_id));

    // Super removes second directly too.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/admins/{second_id}/remove"),
        None,
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let (status, _) = login_cookie(app.clone(), "second@x.dev", "Oth3r!Pass").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn suspend_restore_is_super_only() {
    let app = app_with_admin();
    let (super_cookie, super_id) = bootstrap(app.clone()).await;
    let (_, body) = create_admin_as(app.clone(), &super_cookie, "Second", "second@x.dev").await;
    let second_id = body["admin"]["id"].as_str().unwrap().to_string();
    let (_, second_cookie) = login_cookie(app.clone(), "second@x.dev", "Oth3r!Pass").await;

    // Non-super suspend attempts are forbidden.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/admins/{second_id}/status"),
        Some(json!({"status": "suspended"})),
        &second_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Bad status values rejected.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/admins/{second_id}/status"),
        Some(json!({"status": "banned"})),
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Super target and self are protected.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/admins/{super_id}/status"),
        Some(json!({"status": "suspended"})),
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Suspend blocks login and kills sessions.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/admins/{second_id}/status"),
        Some(json!({"status": "suspended"})),
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let (status, _) = login_cookie(app.clone(), "second@x.dev", "Oth3r!Pass").await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Suspend revokes sessions, so the old cookie is simply dead (401).
    let res = request_with_cookie(app.clone(), "GET", "/admin/me", None, &second_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Restore reopens login.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/admins/{second_id}/status"),
        Some(json!({"status": "active"})),
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let (status, _) = login_cookie(app.clone(), "second@x.dev", "Oth3r!Pass").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn non_super_remove_creates_nothing() {
    let app = app_with_admin();
    let (super_cookie, _) = bootstrap(app.clone()).await;
    let _ = create_admin_as(app.clone(), &super_cookie, "Second", "second@x.dev").await;
    let (_, second_cookie) = login_cookie(app.clone(), "second@x.dev", "Oth3r!Pass").await;

    let (_, body) = create_admin_as(app.clone(), &super_cookie, "Third", "third@x.dev").await;
    let third_id = body["admin"]["id"].as_str().unwrap().to_string();

    // Forbidden outright — and the approvals surface is gone entirely.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/admins/{third_id}/remove"),
        None,
        &second_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/approvals?status=pending",
        None,
        &super_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Target untouched.
    let (status, _) = login_cookie(app.clone(), "third@x.dev", "Oth3r!Pass").await;
    assert_eq!(status, StatusCode::OK);
}
