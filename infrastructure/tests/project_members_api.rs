//! End-to-end HTTP tests for project team invites (GitHub-style):
//! invite → email → accept/decline, with the 2FA gate enforced at
//! accept time, plus the require-2fa flag and the team roster.

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
use server::testing::{FakeAuditStore, FakeAuthStore, FakeMailer, FakeTemplates};
use totp_rs::{Rfc6238, Secret, TOTP};
use tower::ServiceExt;

fn app() -> (Router, Arc<FakeAuthStore>, Arc<FakeMailer>) {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let mailer = Arc::new(FakeMailer::new());
    let templates = Arc::new(FakeTemplates::new());
    templates.seed(
        "member-invite",
        "Invite",
        "<a href=\"{{link}}\">Accept</a>",
        "Accept: {{link}}",
    );
    let members = Arc::new(
        ProjectMembersService::new(auth_store.clone(), auth_store.clone(), audit.clone())
            .with_system_mail(
                mailer.clone(),
                templates,
                "http://localhost:3000".to_string(),
                true,
            ),
    );
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
        ),
        auth_store,
        mailer,
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

/// A currently-valid TOTP code for a base32 secret.
fn current_code(secret_base32: &str) -> String {
    let raw = Secret::Encoded(secret_base32.to_string()).to_raw().unwrap();
    let rfc = Rfc6238::with_defaults(raw.to_bytes().unwrap()).unwrap();
    TOTP::from_rfc6238(rfc).unwrap().generate_current().unwrap()
}

async fn enable_totp(app: Router, token: &str) {
    let res = request(
        app.clone(),
        "POST",
        "/app/auth/totp/setup",
        Some(json!({})),
        Some(token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let secret = body_json(res).await["totpSecret"]
        .as_str()
        .unwrap()
        .to_string();
    let res = request(
        app,
        "POST",
        "/app/auth/totp/verify",
        Some(json!({"code": current_code(&secret)})),
        Some(token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn invite_accept_decline_lifecycle() {
    let (app, _, mailer) = app();
    let owner = signup(app.clone(), "Owner", "owner@x.dev").await;
    let member = signup(app.clone(), "Member", "member@x.dev").await;
    let project = create_project(app.clone(), &owner, "Invites").await;

    // Invite creates a pending invite + email (dev token exposed).
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
    assert_eq!(body["invite"]["email"], "member@x.dev");
    assert_eq!(body["invite"]["role"], "developer");
    let token = body["inviteToken"].as_str().unwrap().to_string();
    assert_eq!(mailer.sent().len(), 1);
    assert_eq!(mailer.sent()[0].to, "member@x.dev");
    // Invites are automated mail: no-reply sender, never support/human.
    assert_eq!(mailer.sent()[0].from, server::ports::SystemSender::NoReply);

    // Not a member yet — invite alone grants nothing.
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/members"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Preview works for the invitee.
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/invites/{token}"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // ...but not for other accounts.
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/invites/{token}"),
        None,
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Accept joins the project.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/invites/{token}/accept"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["member"]["role"], "developer");

    // Double accept is idempotent.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/invites/{token}/accept"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Unknown tokens are quiet.
    let res = request(
        app.clone(),
        "POST",
        "/app/invites/nope/decline",
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn membership_requires_2fa_at_accept_time() {
    let (app, _, _) = app();
    let owner = signup(app.clone(), "Owner", "o@x.dev").await;
    let member = signup(app.clone(), "Member", "m@x.dev").await;
    let project = create_project(app.clone(), &owner, "Gated").await;

    // Flag defaults off and is visible on the project.
    let res = request(app.clone(), "GET", "/app/projects", None, Some(&owner)).await;
    assert_eq!(body_json(res).await["projects"][0]["require2fa"], false);

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

    // Invite still succeeds — the gate is at accept time.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "m@x.dev", "role": "developer"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let token = body_json(res).await["inviteToken"]
        .as_str()
        .unwrap()
        .to_string();

    // Accept without 2FA is refused with an explicit message.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/invites/{token}/accept"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let body = body_json(res).await;
    assert!(
        body["detail"].as_str().unwrap().contains("two-factor"),
        "unexpected detail: {body}"
    );

    // Enable 2FA, then accept works.
    enable_totp(app.clone(), &member).await;
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/invites/{token}/accept"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["member"]["has2fa"], true);
}

#[tokio::test]
async fn expired_invites_are_rejected() {
    use server::domain::auth::value_objects::hash_token;

    let (app, store, _) = app();
    let owner = signup(app.clone(), "Owner", "oe@x.dev").await;
    signup(app.clone(), "Member", "me@x.dev").await;
    let project = create_project(app.clone(), &owner, "Expiring").await;

    store.seed_invite(
        &project,
        "me@x.dev",
        "viewer",
        &hash_token("expired-raw-token"),
        chrono::Utc::now() - chrono::Duration::hours(1),
    );
    let res = request(
        app.clone(),
        "POST",
        "/app/auth/login",
        Some(json!({"email": "me@x.dev", "password": "Sup3rSecret!"})),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let invitee = body_json(res).await["session"]["token"]
        .as_str()
        .unwrap()
        .to_string();

    let res = request(
        app.clone(),
        "POST",
        "/app/invites/expired-raw-token/accept",
        None,
        Some(&invitee),
    )
    .await;
    assert_eq!(res.status(), StatusCode::GONE);
}

#[tokio::test]
async fn only_owners_grant_owner_role() {
    let (app, _, _) = app();
    let owner = signup(app.clone(), "Owner", "o2@x.dev").await;
    let admin = signup(app.clone(), "Admin", "a2@x.dev").await;
    let project = create_project(app.clone(), &owner, "Roles").await;

    // Owner invites an admin.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "a2@x.dev", "role": "admin"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let token = body_json(res).await["inviteToken"]
        .as_str()
        .unwrap()
        .to_string();

    // The admin accepts to join.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/invites/{token}/accept"),
        None,
        Some(&admin),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // The admin cannot invite another owner.
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
    let (app, _, _) = app();
    let owner = signup(app.clone(), "Owner", "owner3@x.dev").await;
    signup(app.clone(), "Plain", "plain3@x.dev").await;
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

    // Invite + accept adds them to the roster.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "plain3@x.dev", "role": "viewer"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let token = body_json(res).await["inviteToken"]
        .as_str()
        .unwrap()
        .to_string();

    let res = request(
        app.clone(),
        "POST",
        "/app/auth/login",
        Some(json!({"email": "plain3@x.dev", "password": "Sup3rSecret!"})),
        None,
    )
    .await;
    let invitee = body_json(res).await["session"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/invites/{token}/accept"),
        None,
        Some(&invitee),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

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
