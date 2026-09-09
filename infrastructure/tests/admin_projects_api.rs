//! End-to-end HTTP tests for the admin project surface.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use server::api::build_router;
use server::api::state::AppState;
use server::domain::admin::{AdminProjectsService, AdminService};
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::projects::ProjectService;
use server::domain::support::TicketService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeTicketsStore,
};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app_with_admin_projects() -> Router {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(Box::new(FakeAdminStore::new()), true));
    let tickets_store = Arc::new(FakeTicketsStore::new());
    let tickets = Arc::new(TicketService::new(tickets_store, audit.clone()));
    let admin_projects = Arc::new(AdminProjectsService::new(auth_store));
    build_router(
        AppState {
            db: None,
            redis: None,
            auth: Some(auth),
            oauth: None,
            admin: Some(admin),
            admin_users: None,
            admin_projects: Some(admin_projects),
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

async fn signup(app: Router, name: &str, email: &str) -> (String, String) {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/signup",
        Some(json!({"name": name, "email": email, "password": "Sup3rSecret!"})),
        "session_token",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    let token = body["session"]["token"].as_str().unwrap().to_string();
    (token, user_id)
}

async fn create_project_as(app: Router, cookie: &str, name: &str) -> String {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/projects",
        Some(json!({"name": name, "description": null})),
        "session_token",
        cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["project"]["id"].as_str().unwrap().to_string()
}

async fn bootstrap_admin(app: Router) -> String {
    let res = request_with_cookie(
        app.clone(),
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
async fn admin_lists_all_projects_with_owner_info() {
    let app = app_with_admin_projects();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (alice_cookie, _) = signup(app.clone(), "Alice Anderson", "alice@x.dev").await;
    let (bob_cookie, _) = signup(app.clone(), "Bob Brown", "bob@x.dev").await;
    create_project_as(app.clone(), &alice_cookie, "Alice App").await;
    create_project_as(app.clone(), &bob_cookie, "Bob App").await;

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/projects",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let projects = body["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    assert_eq!(body["total"], 2);
    assert_eq!(body["totalPages"], 1);
    // Newest first.
    assert_eq!(projects[0]["name"], "Bob App");
    assert_eq!(projects[0]["ownerEmail"], "bob@x.dev");
    assert_eq!(projects[0]["ownerName"], "Bob Brown");
    assert_eq!(projects[1]["ownerEmail"], "alice@x.dev");

    // Search narrows to one.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/projects?search=alice",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(body_json(res).await["projects"].as_array().unwrap().len(), 1);

    // Environment filter (all start as development).
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/projects?environment=production",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(body_json(res).await["projects"].as_array().unwrap().len(), 0);

    // Pagination.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/projects?per_page=1",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["projects"].as_array().unwrap().len(), 1);
    assert_eq!(body["total"], 2);
    assert_eq!(body["totalPages"], 2);
    assert_eq!(body["page"], 1);
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/projects?per_page=1&page=2",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["projects"].as_array().unwrap().len(), 1);
    assert_eq!(body["page"], 2);
    assert_eq!(body["totalPages"], 2);
}

#[tokio::test]
async fn admin_project_detail_and_auth() {
    let app = app_with_admin_projects();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (alice_cookie, _) = signup(app.clone(), "Alice Anderson", "alice@x.dev").await;
    let project_id = create_project_as(app.clone(), &alice_cookie, "Alice App").await;

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/admin/projects/{project_id}"),
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["project"]["name"], "Alice App");
    assert_eq!(body["project"]["ownerEmail"], "alice@x.dev");
    assert_eq!(body["project"]["environment"], "development");

    // Customer sessions are rejected; unknown ids 404.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/admin/projects/{project_id}"),
        None,
        "session_token",
        &alice_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/projects/01J00000000000000000000000",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
