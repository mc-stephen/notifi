//! End-to-end HTTP tests for the admin notification broadcast surface.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use server::api::build_router;
use server::api::state::AppState;
use server::domain::admin::{AdminNotificationsService, AdminService};
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::notifications::NotificationService;
use server::domain::projects::ProjectService;
use server::domain::support::TicketService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeNotificationsStore, FakeTicketsStore,
};
use tower::ServiceExt;

fn app_with_admin_notifications() -> Router {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(
        Box::new(FakeAdminStore::new()),
        true,
        audit.clone(),
    ));
    let tickets_store = Arc::new(FakeTicketsStore::new());
    let tickets = Arc::new(TicketService::new(tickets_store, audit.clone()));
    let notifications_store = Arc::new(FakeNotificationsStore::new());
    let admin_notifications = Arc::new(AdminNotificationsService::new(
        auth_store.clone(),
        auth_store,
        notifications_store.clone(),
        audit.clone(),
    ));
    let notifications = Arc::new(NotificationService::new(notifications_store));
    build_router(
        AppState {
            db: None,
            redis: None,
            auth: Some(auth),
            oauth: None,
            admin: Some(admin),
            admin_users: None,
            admin_projects: None,
            admin_notifications: Some(admin_notifications),
            projects: Some(projects),
            project_members: None,
            audit: Some(audit),
            recipients: None,
            templates: None,
            channel_providers: None,
            tickets: Some(tickets),
            notifications: Some(notifications),
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
    let body = body
        .map(|b| Body::from(b.to_string()))
        .unwrap_or_else(Body::empty);
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
async fn admin_broadcast_reaches_user_bell() {
    let app = app_with_admin_notifications();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (user_cookie, user_id) = signup(app.clone(), "Bell User", "bell@x.dev").await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Hello",
            "content": "<p>World</p>",
            "notificationType": "system",
            "channels": ["in_app"],
            "audience": {"kind": "users", "userIds": [user_id]},
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["broadcast"]["recipientCount"], 1);
    assert_eq!(body["broadcast"]["scheduled"], false);
    let broadcast_id = body["broadcast"]["id"].as_str().unwrap().to_string();

    // The user sees it in their bell.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/app/notifications",
        None,
        "session_token",
        &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let items = body["notifications"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "Hello");

    // History + detail report it with read stats.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/notifications",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    let body = body_json(res).await;
    assert_eq!(body["total"], 1);
    assert_eq!(body["broadcasts"][0]["id"], broadcast_id);
    assert_eq!(body["broadcasts"][0]["recipientCount"], 1);
    assert_eq!(body["broadcasts"][0]["readCount"], 0);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/admin/notifications/{broadcast_id}"),
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["broadcast"]["status"], "sent");
}

#[tokio::test]
async fn admin_broadcast_validates_input_and_auth() {
    let app = app_with_admin_notifications();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (user_cookie, _) = signup(app.clone(), "V User", "v@x.dev").await;

    // Customer sessions are rejected.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Hi",
            "content": "there",
            "audience": {"kind": "all"},
        })),
        "session_token",
        &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Email alone is rejected (in-app required to deliver).
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Hi",
            "content": "there",
            "channels": ["email"],
            "audience": {"kind": "all"},
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // In-app + email together sends and records both channels.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Hi",
            "content": "there",
            "channels": ["in_app", "email"],
            "audience": {"kind": "all"},
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["broadcast"]["scheduled"], false);
    let both_id = body["broadcast"]["id"].as_str().unwrap().to_string();

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/admin/notifications/{both_id}"),
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(
        body_json(res).await["broadcast"]["channels"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>(),
        vec!["email", "in_app"]
    );

    // Bad audience kind and unknown user.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Hi",
            "content": "there",
            "audience": {"kind": "org"},
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Hi",
            "content": "there",
            "audience": {"kind": "users", "userIds": ["01J00000000000000000000000"]},
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Unknown broadcast 404s.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/notifications/01J00000000000000000000000",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_schedule_and_cancel_flow() {
    let app = app_with_admin_notifications();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    signup(app.clone(), "S User", "s@x.dev").await;

    // Schedule an hour out.
    let send_at = (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Later",
            "content": "future",
            "audience": {"kind": "all"},
            "scheduledFor": send_at,
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["broadcast"]["scheduled"], true);
    let broadcast_id = body["broadcast"]["id"].as_str().unwrap().to_string();

    // Past dates are rejected.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/notifications",
        Some(json!({
            "title": "Past",
            "content": "past",
            "audience": {"kind": "all"},
            "scheduledFor": "2000-01-01T00:00:00Z",
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Scheduled shows in history with status filter.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/notifications?status=scheduled",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(body_json(res).await["total"], 1);

    // Cancel works; second cancel conflicts.
    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/admin/notifications/scheduled/{broadcast_id}"),
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/admin/notifications/scheduled/{broadcast_id}"),
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}
