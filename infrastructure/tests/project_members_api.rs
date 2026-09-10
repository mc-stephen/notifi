//! End-to-end HTTP tests for project team management: the 2FA gate
//! flag and member invites with TOTP enforcement.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use server::api::build_router;
use server::api::state::AppState;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::projects::{ProjectMembersService, ProjectService};
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAuditStore, FakeAuthStore};
use tower::ServiceExt;

fn app() -> Router {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let members = Arc::new(ProjectMembersService::new(
        auth_store.clone(),
        auth_store,
        audit.clone(),
    ));
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
            project_members: Some(members),
            audit: Some(audit),
            recipients: None,
            templates: None,
            channel_providers: None,
            tickets: None,
            notifications: None,
            billing: None,
            provider_tester: std::sync::Arc::new(ConfigProviderTester::new())
                as std::sync::Arc<dyn ProviderTester + Send + Sync>,
        },
        &AppConfig::default(),
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn request(
    app: Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    let body = body
        .map(|b| Body::from(b.to_string()))
        .unwrap_or_else(Body::empty);
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

async fn signup(app: Router, name: &str, email: &str) -> String {
    let res = request(
        app,
        "POST",
        "/app/auth/signup",
        Some(json!({"name": name, "email": email, "password": "Sup3rSecret!"})),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["session"]["token"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create_project(app: Router, token: &str, name: &str) -> String {
    let res = request(
        app,
        "POST",
        "/app/projects",
        Some(json!({"name": name})),
        Some(token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["project"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn membership_requires_2fa_when_flagged() {
    let app = app();
    let owner = signup(app.clone(), "Owner", "owner@x.dev").await;
    let member = signup(app.clone(), "Member", "member@x.dev").await;
    let project = create_project(app.clone(), &owner, "Gated").await;

    // Flag defaults off and is visible on the project.
    let res = request(app.clone(), "GET", "/app/projects", None, Some(&owner)).await;
    let projects = body_json(res).await["projects"].clone();
    assert_eq!(projects[0]["require2fa"], false);

    // Non-managers cannot flip the flag.
    let res = request(
        app.clone(),
        "PATCH",
        &format!("/app/projects/{project}/require-2fa"),
        Some(json!({"enabled": true})),
        Some(&member),
    )
    .await;
    assert!(res.status() == StatusCode::NOT_FOUND || res.status() == StatusCode::FORBIDDEN);

    // Owner flips it on.
    let res = request(
        app.clone(),
        "PATCH",
        &format!("/app/projects/{project}/require-2fa"),
        Some(json!({"enabled": true})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["require2fa"], true);

    // Inviting an account without 2FA is refused with an explicit message.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "member@x.dev", "role": "developer"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let body = body_json(res).await;
    assert!(
        body["detail"].as_str().unwrap().contains("two-factor"),
        "unexpected detail: {body}"
    );

    // ...but works once the flag is off again.
    let res = request(
        app.clone(),
        "PATCH",
        &format!("/app/projects/{project}/require-2fa"),
        Some(json!({"enabled": false})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "member@x.dev", "role": "developer"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["member"]["role"], "developer");

    // Duplicate membership conflicts.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "member@x.dev", "role": "viewer"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);

    // Unknown accounts and bad roles are rejected, not invited.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "ghost@x.dev", "role": "viewer"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "member@x.dev", "role": "superuser"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn only_owners_grant_owner_role() {
    let app = app();
    let owner = signup(app.clone(), "Owner", "o2@x.dev").await;
    let admin = signup(app.clone(), "Admin", "a2@x.dev").await;
    let project = create_project(app.clone(), &owner, "Roles").await;

    // Owner adds an admin.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "a2@x.dev", "role": "admin"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    // The admin cannot mint another owner.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "o2@x.dev", "role": "owner"})),
        Some(&admin),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn team_roster_shows_2fa_standing() {
    let app = app();
    let owner = signup(app.clone(), "Owner", "owner3@x.dev").await;
    let _plain = signup(app.clone(), "Plain", "plain3@x.dev").await;
    let project = create_project(app.clone(), &owner, "Roster").await;

    // Owner sees themselves with 2FA off.
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/members"),
        None,
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let members = body_json(res).await["members"].clone();
    let members = members.as_array().unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0]["role"], "owner");
    assert_eq!(members[0]["has2fa"], false);

    // Add a member, then enable 2FA for them directly through a second
    // account round-trip is unavailable here — instead assert the member
    // row appears with their live flag (false).
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "plain3@x.dev", "role": "viewer"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(body_json(res).await["member"]["has2fa"], false);

    let res = request(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/members"),
        None,
        Some(&owner),
    )
    .await;
    let members = body_json(res).await["members"].clone();
    assert_eq!(members.as_array().unwrap().len(), 2);

    // Outsiders see nothing (indistinguishable from missing).
    let stranger = signup(app.clone(), "Stranger", "stranger3@x.dev").await;
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/members"),
        None,
        Some(&stranger),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
