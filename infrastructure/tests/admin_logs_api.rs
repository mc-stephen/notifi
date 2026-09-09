//! End-to-end HTTP tests for the admin audit-log surface.

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

fn app_with_admin_logs() -> Router {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(
        Box::new(FakeAdminStore::new()),
        true,
        audit.clone(),
    ));
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

async fn request_with_cookie(
    app: Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    cookie_name: &str,
    cookie: &str,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if !cookie.is_empty() {
        builder = builder.header("cookie", format!("{cookie_name}={cookie}"));
    }
    let body = body.map(|b| Body::from(b.to_string())).unwrap_or_else(Body::empty);
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

async fn bootstrap(app: Router) -> String {
    let res = request_with_cookie(
        app,
        "POST",
        "/admin/bootstrap",
        Some(json!({"name": "Root", "email": "root@notifi.dev", "password": "Adm1n!Pass"})),
        "admin_session",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    session_cookie_from(&res)
}

#[tokio::test]
async fn admin_logs_record_and_list_with_filters() {
    let app = app_with_admin_logs();
    let admin_cookie = bootstrap(app.clone()).await;

    // Login emits a second admin event.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/login",
        Some(json!({"email": "root@notifi.dev", "password": "Adm1n!Pass"})),
        "admin_session",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Full list shows both, newest first, with admin attribution.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/logs",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["total"], 2);
    assert_eq!(body["totalPages"], 1);
    let logs = body["logs"].as_array().unwrap();
    assert_eq!(logs[0]["eventType"], "admin.login");
    assert_eq!(logs[0]["actorType"], "admin");
    assert_eq!(logs[0]["actorName"], "Root");
    assert!(logs[0]["adminId"].as_str().is_some());
    assert!(logs[0]["userId"].is_null());
    assert_eq!(logs[1]["eventType"], "admin.created");

    // Event-type filter.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/logs?eventType=admin.login",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["logs"][0]["eventType"], "admin.login");

    // Actor-type filter.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/logs?actorType=user",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(body_json(res).await["total"], 0);

    // Search filter.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/logs?search=bootstrapped",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(body_json(res).await["total"], 1);

    // Pagination envelope.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/logs?per_page=1&page=2",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["logs"].as_array().unwrap().len(), 1);
    assert_eq!(body["page"], 2);
    assert_eq!(body["totalPages"], 2);
    assert_eq!(body["logs"][0]["eventType"], "admin.created");
}

#[tokio::test]
async fn admin_logs_require_admin_session() {
    let app = app_with_admin_logs();
    let _ = bootstrap(app.clone()).await;

    let res =
        request_with_cookie(app.clone(), "GET", "/admin/logs", None, "admin_session", "").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/logs",
        None,
        "session_token",
        "not-an-admin-cookie",
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_logs_reject_bad_actor_type() {
    let app = app_with_admin_logs();
    let admin_cookie = bootstrap(app.clone()).await;

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/logs?actorType=superuser",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
