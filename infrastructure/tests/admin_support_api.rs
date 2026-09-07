//! End-to-end HTTP tests for the admin support surface (fake-backed services).

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
use server::testing::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeRecipientsStore, FakeTicketsStore,
};
use serde_json::{Value, json};
use tower::ServiceExt;

fn app_with_admin_support() -> (Router, Arc<FakeTicketsStore>) {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(Box::new(FakeAdminStore::new())));
    let tickets_store = Arc::new(FakeTicketsStore::new());
    let tickets = Arc::new(TicketService::new(tickets_store.clone(), audit.clone()));
    let _recipients_store = Arc::new(FakeRecipientsStore::new());
    (
        build_router(
            AppState {
                db: None,
                redis: None,
                auth: Some(auth),
                oauth: None,
                admin: Some(admin),
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
        ),
        tickets_store,
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn session_cookie_from(res: &axum::response::Response) -> String {
    let set_cookie = res
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    set_cookie
        .split(';')
        .next()
        .unwrap_or("")
        .strip_prefix("session_token=")
        .unwrap_or("")
        .to_string()
}

async fn signup_and_login(app: Router, tickets: &FakeTicketsStore, email: &str) -> (String, String) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Jane", "email": email, "password": "Sup3rSecret!"})
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
    let body = body_json(res).await;
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    let token = body["session"]["token"].as_str().unwrap().to_string();
    tickets.seed_user(&user_id, "Jane", email);
    (token, user_id)
}

async fn bootstrap_admin(app: Router, tickets: &FakeTicketsStore) -> String {
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/admin/bootstrap")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Root", "email": "root@notifi.dev", "password": "Adm1n!Pass"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let cookie = session_cookie_from(&res);
    assert!(!cookie.is_empty(), "bootstrap must set a session cookie");
    let body = body_json(res).await;
    let admin_id = body["userId"].as_str().unwrap().to_string();
    tickets.seed_admin(&admin_id, "Root");
    cookie
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
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    let body = body.map(|b| Body::from(b.to_string())).unwrap_or_else(Body::empty);
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

async fn create_ticket_as(app: Router, cookie: &str) -> String {
    let res = request_with_cookie(
        app,
        "POST",
        "/v1/support/tickets",
        Some(json!({
            "projectId": null,
            "subject": "Login is broken",
            "category": "Technical Issue",
            "priority": "High",
            "description": "Cannot sign in since yesterday.",
        })),
        cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    body["ticket"]["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn admin_lists_all_tickets_with_customer_identity() {
    let (app, tickets) = app_with_admin_support();
    let (user_cookie, _) = signup_and_login(app.clone(), &tickets, "jane@x.dev").await;
    let admin_cookie = bootstrap_admin(app.clone(), &tickets).await;
    let ticket_id = create_ticket_as(app.clone(), &user_cookie).await;

    let res = request_with_cookie(app.clone(), "GET", "/v1/admin/support/tickets", None, &admin_cookie).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let list = body["tickets"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"].as_str().unwrap(), ticket_id);
    assert_eq!(list[0]["customerEmail"].as_str().unwrap(), "jane@x.dev");
    assert_eq!(list[0]["customerName"].as_str().unwrap(), "Jane");
    assert_eq!(list[0]["status"].as_str().unwrap(), "open");
    assert_eq!(body["hasMore"], false);
}

#[tokio::test]
async fn admin_views_thread_and_replies_as_support() {
    let (app, tickets) = app_with_admin_support();
    let (user_cookie, _) = signup_and_login(app.clone(), &tickets, "jane@x.dev").await;
    let admin_cookie = bootstrap_admin(app.clone(), &tickets).await;
    let ticket_id = create_ticket_as(app.clone(), &user_cookie).await;

    // Admin sees the ticket.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/v1/admin/support/tickets/{ticket_id}"),
        None,
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Empty thread.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/v1/admin/support/tickets/{ticket_id}/messages"),
        None,
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["messages"].as_array().unwrap().len(), 0);

    // Admin replies.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/v1/admin/support/tickets/{ticket_id}/messages"),
        Some(json!({"body": "We are looking into this."})),
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["message"]["author"].as_str().unwrap(), "support");

    // Customer sees the reply attributed to support.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/v1/support/tickets/{ticket_id}/messages"),
        None,
        &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["author"].as_str().unwrap(), "support");
    assert_eq!(messages[0]["body"].as_str().unwrap(), "We are looking into this.");

    // Admin thread shows the support author's name.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/v1/admin/support/tickets/{ticket_id}/messages"),
        None,
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["authorName"].as_str().unwrap(), "Root");
}

#[tokio::test]
async fn admin_sets_ticket_status() {
    let (app, tickets) = app_with_admin_support();
    let (user_cookie, _) = signup_and_login(app.clone(), &tickets, "jane@x.dev").await;
    let admin_cookie = bootstrap_admin(app.clone(), &tickets).await;
    let ticket_id = create_ticket_as(app.clone(), &user_cookie).await;

    for status in ["in_progress", "resolved", "closed", "open"] {
        let res = request_with_cookie(
            app.clone(),
            "PATCH",
            &format!("/v1/admin/support/tickets/{ticket_id}"),
            Some(json!({"status": status})),
            &admin_cookie,
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert_eq!(body["ticket"]["status"].as_str().unwrap(), status);
    }

    // Invalid status is rejected.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/v1/admin/support/tickets/{ticket_id}"),
        Some(json!({"status": "waiting"})),
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn admin_reply_moves_resolved_ticket_to_in_progress() {
    let (app, tickets) = app_with_admin_support();
    let (user_cookie, _) = signup_and_login(app.clone(), &tickets, "jane@x.dev").await;
    let admin_cookie = bootstrap_admin(app.clone(), &tickets).await;
    let ticket_id = create_ticket_as(app.clone(), &user_cookie).await;

    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/v1/admin/support/tickets/{ticket_id}"),
        Some(json!({"status": "resolved"})),
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/v1/admin/support/tickets/{ticket_id}/messages"),
        Some(json!({"body": "Actually, one more thing to check."})),
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/v1/admin/support/tickets/{ticket_id}"),
        None,
        &admin_cookie,
    )
    .await;
    assert_eq!(body_json(res).await["ticket"]["status"].as_str().unwrap(), "in_progress");
}

#[tokio::test]
async fn admin_cannot_reply_to_closed_ticket() {
    let (app, tickets) = app_with_admin_support();
    let (user_cookie, _) = signup_and_login(app.clone(), &tickets, "jane@x.dev").await;
    let admin_cookie = bootstrap_admin(app.clone(), &tickets).await;
    let ticket_id = create_ticket_as(app.clone(), &user_cookie).await;

    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/v1/admin/support/tickets/{ticket_id}"),
        Some(json!({"status": "closed"})),
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/v1/admin/support/tickets/{ticket_id}/messages"),
        Some(json!({"body": "Hello?"})),
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn admin_endpoints_require_admin_session() {
    let (app, tickets) = app_with_admin_support();
    let (user_cookie, _) = signup_and_login(app.clone(), &tickets, "jane@x.dev").await;
    let admin_cookie = bootstrap_admin(app.clone(), &tickets).await;
    let ticket_id = create_ticket_as(app.clone(), &user_cookie).await;

    // No cookie at all.
    let res = request_with_cookie(app.clone(), "GET", "/v1/admin/support/tickets", None, "").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // A customer session is not an admin session.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/v1/admin/support/tickets/{ticket_id}"),
        None,
        &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/v1/admin/support/tickets/{ticket_id}/messages"),
        Some(json!({"body": "hi"})),
        &user_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // Unknown ticket -> 404 (with a valid admin session).
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/v1/admin/support/tickets/01J0000000000000000000000",
        None,
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_get_unknown_ticket_is_404() {
    let (app, tickets) = app_with_admin_support();
    let admin_cookie = bootstrap_admin(app.clone(), &tickets).await;

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/v1/admin/support/tickets/01J0000000000000000000000",
        None,
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/v1/admin/support/tickets/01J0000000000000000000000/messages",
        None,
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
