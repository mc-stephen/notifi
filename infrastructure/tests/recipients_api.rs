//! End-to-end HTTP tests for the recipients feature (fake-backed services).
//!
//! Runs without a database. Follows the auth_api flow: sign up + log in, then
//! complete onboarding to create a real project, then exercise the recipient
//! endpoints scoped to that project.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use server::api::build_router;
use server::api::state::AppState;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::projects::ProjectService;
use server::domain::recipients::RecipientService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAuditStore, FakeAuthStore, FakeRecipientsStore};
use serde_json::{Value, json};
use tower::ServiceExt;

/// App wired with fake auth + recipients services.
fn app_with_recipients() -> (Router, Arc<FakeRecipientsStore>) {
    let store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(store.clone(), audit.clone()));
    let recipients_store = Arc::new(FakeRecipientsStore::new());
    let recipients = Arc::new(RecipientService::new(
        recipients_store.clone(),
        audit.clone(),
    ));
    (
        build_router(
            AppState {
                db: None,
                redis: None,
                auth: Some(auth),
                oauth: None,
                projects: Some(projects),
                audit: Some(audit),
                recipients: Some(recipients),
                templates: None,
                channel_providers: None,
                tickets: None,
                notifications: None,
                provider_tester: std::sync::Arc::new(ConfigProviderTester::new()) as std::sync::Arc<dyn ProviderTester + Send + Sync>,
            },
            &AppConfig::default(),
        ),
        recipients_store,
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
    (token, user_id)
}

async fn post_with_cookie(
    app: Router,
    uri: &str,
    body: Value,
    cookie: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn get_with_cookie(
    app: Router,
    uri: &str,
    cookie: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("GET")
            .uri(uri)
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn delete_with_cookie(
    app: Router,
    uri: &str,
    cookie: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("DELETE")
            .uri(uri)
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn patch_with_cookie(
    app: Router,
    uri: &str,
    body: Value,
    cookie: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("content-type", "application/json")
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

/// Signs up, creates a project via onboarding, grants the user access in the
/// recipient store, and returns (cookie, uid, project_id).
async fn onboard_project_on(
    app: Router,
    recipient_store: &Arc<FakeRecipientsStore>,
    email: &str,
    project_name: &str,
) -> (String, String, String) {
    let (token, uid) = signup_and_login_on(app.clone(), email).await;

    let res = post_with_cookie(
        app.clone(),
        "/v1/auth/onboarding/complete",
        json!({"project": {"name": project_name, "description": "test"}}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = get_with_cookie(app.clone(), "/v1/projects", &token).await;
    let project_id = body_json(res).await["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    recipient_store.seed_visible(&uid, &project_id);
    (token, uid, project_id)
}

#[tokio::test]
async fn recipients_require_auth() {
    let (app, _store) = app_with_recipients();
    let res = get_with_cookie(app, "/v1/projects/p1/recipients", "bogus").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_and_list_and_get_and_delete_roundtrip() {
    let (app, _store) = app_with_recipients();
    let (token, _uid, pid) = onboard_project_on(app.clone(), &_store, "rcp1@example.com", "My App").await;

    // create
    let res = post_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/recipients"),
        json!({"userId": "u_123", "name": "Ada Lovelace", "contacts": {"email": "ada@example.com", "phone": "+1415"}}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let created = body_json(res).await["recipient"].clone();
    assert_eq!(created["userId"], "u_123");
    assert_eq!(created["name"], "Ada Lovelace");
    assert_eq!(created["contacts"]["email"], "ada@example.com");
    let rid = created["id"].as_str().unwrap().to_string();

    // get
    let res = get_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/recipients/{rid}"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["recipient"]["name"], "Ada Lovelace");

    // list
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid}/recipients"), &token).await;
    assert_eq!(res.status(), StatusCode::OK);
    let list = body_json(res).await;
    assert_eq!(list["recipients"].as_array().unwrap().len(), 1);
    assert_eq!(list["hasMore"], false);

    // delete
    let res = delete_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/recipients/{rid}"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"], "ok");

    // gone from list
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid}/recipients"), &token).await;
    assert_eq!(body_json(res).await["recipients"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn duplicate_user_id_within_a_project_conflicts() {
    let (app, _store) = app_with_recipients();
    let (token, _uid, pid) = onboard_project_on(app.clone(), &_store, "rcp2@example.com", "My App").await;

    let uri = format!("/v1/projects/{pid}/recipients");
    let res = post_with_cookie(
        app.clone(),
        &uri,
        json!({"userId": "u_dup", "name": "First"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = post_with_cookie(
        app,
        &uri,
        json!({"userId": "u_dup", "name": "Second"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
    assert!(body_json(res).await["detail"]
        .as_str()
        .unwrap()
        .contains("user_id"));
}

#[tokio::test]
async fn recipients_are_isolated_per_project() {
    let (app, store) = app_with_recipients();

    // Two separate users, each with their own project.
    let (token_a, _uid_a, pid_a) =
        onboard_project_on(app.clone(), &store, "rcp3a@example.com", "App A").await;
    let (token_b, _uid_b, pid_b) =
        onboard_project_on(app.clone(), &store, "rcp3b@example.com", "App B").await;

    // User A creates one recipient in project A.
    let res = post_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid_a}/recipients"),
        json!({"userId": "u_own", "name": "Owner"}),
        &token_a,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let rid = body_json(res).await["recipient"]["id"].as_str().unwrap().to_string();

    // User B's project list must not show project A's recipient.
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid_b}/recipients"), &token_b).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["recipients"].as_array().unwrap().len(), 0);

    // User B fetching the recipient under project B must 404 (exists only under A).
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid_b}/recipients/{rid}"), &token_b).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_requires_user_id_and_name() {
    let (app, store) = app_with_recipients();
    let (token, _uid, pid) = onboard_project_on(app.clone(), &store, "rcp4@example.com", "My App").await;

    let uri = format!("/v1/projects/{pid}/recipients");

    // Missing required fields are rejected at deserialization (422).
    let res = post_with_cookie(app.clone(), &uri, json!({"name": "No ID"}), &token).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let res = post_with_cookie(app.clone(), &uri, json!({"userId": "u_x"}), &token).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // `contacts` must be a JSON object (service-level validation → 400).
    let res = post_with_cookie(app.clone(), &uri, json!({"userId": "u_x", "name": "Bad contacts", "contacts": [1, 2]}), &token).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_changes_name_and_replaces_contacts() {
    let (app, store) = app_with_recipients();
    let (token, _uid, pid) = onboard_project_on(app.clone(), &store, "rcp5@example.com", "My App").await;

    let uri = format!("/v1/projects/{pid}/recipients");
    let res = post_with_cookie(
        app.clone(),
        &uri,
        json!({"userId": "u_update", "name": "Old", "contacts": {"email": "old@example.com", "phone": "+1415"}}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let rid = body_json(res).await["recipient"]["id"].as_str().unwrap().to_string();

    // Update both name and contacts (whatsapp/androidToken added, phone dropped).
    let res = patch_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/recipients/{rid}"),
        json!({
            "name": "New Name",
            "contacts": {
                "email": "new@example.com",
                "whatsapp": "+15551234567",
                "androidToken": "APA91b-example"
            }
        }),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let updated = body_json(res).await["recipient"].clone();
    assert_eq!(updated["name"], "New Name");
    assert_eq!(updated["userId"], "u_update"); // user_id immutable
    assert_eq!(updated["contacts"]["email"], "new@example.com");
    assert_eq!(updated["contacts"]["whatsapp"], "+15551234567");
    assert_eq!(updated["contacts"]["androidToken"], "APA91b-example");
    assert!(updated["contacts"]["phone"].is_null(), "phone should be replaced away");

    // GET reflects the update.
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid}/recipients/{rid}"), &token).await;
    let got = body_json(res).await["recipient"].clone();
    assert_eq!(got["name"], "New Name");
    assert_eq!(got["contacts"]["whatsapp"], "+15551234567");
}

#[tokio::test]
async fn update_name_only_keeps_contacts() {
    let (app, store) = app_with_recipients();
    let (token, _uid, pid) = onboard_project_on(app.clone(), &store, "rcp6@example.com", "My App").await;

    let uri = format!("/v1/projects/{pid}/recipients");
    let res = post_with_cookie(
        app.clone(),
        &uri,
        json!({"userId": "u_name", "name": "Original", "contacts": {"email": "keep@example.com"}}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let rid = body_json(res).await["recipient"]["id"].as_str().unwrap().to_string();

    // Only name provided → contacts untouched.
    let res = patch_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/recipients/{rid}"),
        json!({"name": "Renamed"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let updated = body_json(res).await["recipient"].clone();
    assert_eq!(updated["name"], "Renamed");
    assert_eq!(updated["contacts"]["email"], "keep@example.com");
}

#[tokio::test]
async fn update_unknown_recipient_404_and_bad_contacts_400() {
    let (app, store) = app_with_recipients();
    let (token, _uid, pid) = onboard_project_on(app.clone(), &store, "rcp7@example.com", "My App").await;

    // Unknown recipient id → 404.
    let res = patch_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/recipients/does_not_exist"),
        json!({"name": "X"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Non-object contacts → 400.
    let uri = format!("/v1/projects/{pid}/recipients");
    let res = post_with_cookie(
        app.clone(),
        &uri,
        json!({"userId": "u_bad", "name": "Bad"}),
        &token,
    )
    .await;
    let rid = body_json(res).await["recipient"]["id"].as_str().unwrap().to_string();
    let res = patch_with_cookie(
        app,
        &format!("/v1/projects/{pid}/recipients/{rid}"),
        json!({"contacts": [1, 2, 3]}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
