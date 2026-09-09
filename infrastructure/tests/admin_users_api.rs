//! End-to-end HTTP tests for the admin user-management surface.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use server::api::build_router;
use server::api::state::AppState;
use server::domain::admin::{AdminService, AdminUsersService};
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::projects::ProjectService;
use server::domain::support::TicketService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeNotificationsStore, FakeTicketsStore,
};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app_with_admin_users() -> Router {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(Box::new(FakeAdminStore::new()), true));
    let tickets_store = Arc::new(FakeTicketsStore::new());
    let tickets = Arc::new(TicketService::new(tickets_store.clone(), audit.clone()));
    let admin_users = Arc::new(AdminUsersService::new(
        auth_store.clone(),
        auth_store,
        Arc::new(FakeNotificationsStore::new()),
        tickets_store,
        audit.clone(),
    ));
    build_router(
        AppState {
            db: None,
            redis: None,
            auth: Some(auth),
            oauth: None,
            admin: Some(admin),
            admin_users: Some(admin_users),
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

async fn signup(app: Router, name: &str, email: &str) -> (String, String) {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/signup",
        Some(json!({"name": name, "email": email, "password": "Sup3rSecret!"})),
        "session_token", "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    let token = body["session"]["token"].as_str().unwrap().to_string();
    (token, user_id)
}

async fn bootstrap_admin(app: Router) -> String {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/bootstrap",
        Some(json!({"name": "Root", "email": "root@notifi.dev", "password": "Adm1n!Pass"})),
        "admin_session", "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    session_cookie_from(&res)
}

#[tokio::test]
async fn admin_lists_users_with_search_and_status_filters() {
    let app = app_with_admin_users();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (_, alice) = signup(app.clone(), "Alice Anderson", "alice@x.dev").await;
    signup(app.clone(), "Bob Brown", "bob@x.dev").await;
    signup(app.clone(), "Carol Clark", "carol@x.dev").await;

    // All three, newest first.
    let res = request_with_cookie(app.clone(), "GET", "/admin/users", None, "admin_session", &admin_cookie).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["users"].as_array().unwrap().len(), 3);
    assert_eq!(body["total"], 3);
    assert_eq!(body["totalPages"], 1);
    assert_eq!(body["users"][0]["status"], "active");
    assert_eq!(body["users"][0]["emailVerified"], false);

    // Search narrows to one.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users?search=alice",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["users"].as_array().unwrap().len(), 1);
    assert_eq!(body["users"][0]["id"], alice);

    // Suspend one, then filter by status.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/users/{alice}/status"),
        Some(json!({"status": "suspended"})),
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["user"]["status"], "suspended");

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users?status=suspended",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["users"].as_array().unwrap().len(), 1);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users?status=active",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(body_json(res).await["users"].as_array().unwrap().len(), 2);

    // Bad status values are rejected, not silently empty.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users?status=banned",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn admin_user_list_paginates() {
    let app = app_with_admin_users();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    signup(app.clone(), "U One", "u1@x.dev").await;
    signup(app.clone(), "U Two", "u2@x.dev").await;
    signup(app.clone(), "U Three", "u3@x.dev").await;

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users?per_page=2",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    let page1 = body["users"].as_array().unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(body["page"], 1);
    assert_eq!(body["perPage"], 2);
    assert_eq!(body["total"], 3);
    assert_eq!(body["totalPages"], 2);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users?per_page=2&page=2",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["users"].as_array().unwrap().len(), 1);
    assert_eq!(body["page"], 2);
    assert_eq!(body["total"], 3);
    assert_eq!(body["totalPages"], 2);
}

#[tokio::test]
async fn admin_user_detail_reports_stats() {
    let app = app_with_admin_users();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (user_cookie, user_id) = signup(app.clone(), "Stat User", "stats@x.dev").await;

    // One ticket for this user.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/support/tickets",
        Some(json!({
            "projectId": null,
            "subject": "Help",
            "category": "Other",
            "priority": "Low",
            "description": "please help",
        })),
        "session_token", &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/admin/users/{user_id}"),
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["user"]["email"], "stats@x.dev");
    assert_eq!(body["user"]["projectCount"], 0);
    assert_eq!(body["user"]["ticketTotal"], 1);
    assert_eq!(body["user"]["ticketsOpen"], 1);
    assert_eq!(body["user"]["notificationCount"], 0);
}

#[tokio::test]
async fn suspend_blocks_login_and_restore_reopens() {
    let app = app_with_admin_users();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (user_cookie, user_id) = signup(app.clone(), "Gone User", "gone@x.dev").await;

    // Suspend.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/users/{user_id}/status"),
        Some(json!({"status": "suspended"})),
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Old session is dead.
    let res = request_with_cookie(app.clone(), "GET", "/app/auth/me", None, "session_token", &user_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Password login is blocked with 403.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/login",
        Some(json!({"email": "gone@x.dev", "password": "Sup3rSecret!"})),
        "session_token", "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // Restore reopens login.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/users/{user_id}/status"),
        Some(json!({"status": "active"})),
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/login",
        Some(json!({"email": "gone@x.dev", "password": "Sup3rSecret!"})),
        "session_token", "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_and_user_sessions_coexist_in_one_jar() {
    let app = app_with_admin_users();

    // User login issues the dashboard cookie...
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/login",
        Some(json!({"email": "v@x.dev", "password": "Sup3rSecret!"})),
        "session_token",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let (user_cookie, _) = signup(app.clone(), "Both User", "both@x.dev").await;

    // ...admin bootstrap issues a differently-named cookie.
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
    let set_cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        set_cookie.starts_with("admin_session="),
        "admin bootstrap must set admin_session, got: {set_cookie}"
    );
    let admin_cookie = session_cookie_from(&res);

    // Both sessions authenticate their own surface after the other logged in.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/app/auth/me",
        None,
        "session_token",
        &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(app.clone(), "GET", "/admin/me", None, "admin_session", &admin_cookie).await;
    assert_eq!(res.status(), StatusCode::OK);

    // Admin logout does not disturb the dashboard session.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/logout",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(app.clone(), "GET", "/admin/me", None, "admin_session", &admin_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/app/auth/me",
        None,
        "session_token",
        &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn admin_users_endpoints_validate_input_and_auth() {
    let app = app_with_admin_users();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (user_cookie, user_id) = signup(app.clone(), "V User", "v@x.dev").await;

    // Customer sessions are rejected.
    let res = request_with_cookie(app.clone(), "GET", "/admin/users", None, "session_token", &user_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = request_with_cookie(app.clone(), "GET", "/admin/users", None, "admin_session", "").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Unknown user and malformed id.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users/01J00000000000000000000000",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/users/not-a-ulid",
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Unknown status value on write.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/admin/users/{user_id}/status"),
        Some(json!({"status": "banned"})),
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Revoke sessions kills the cookie but login still works.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/admin/users/{user_id}/sessions/revoke"),
        None,
        "admin_session", &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let res = request_with_cookie(app.clone(), "GET", "/app/auth/me", None, "session_token", &user_cookie).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/login",
        Some(json!({"email": "v@x.dev", "password": "Sup3rSecret!"})),
        "session_token",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}
