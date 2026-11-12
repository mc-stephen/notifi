//! End-to-end HTTP tests for project team invites (GitHub-style):
//! invite → email → accept/decline, with the 2FA gate enforced at
//! accept time, plus the require-2fa flag and the team roster.

mod common;

use std::sync::Arc;

use crate::common::{FakeAuditStore, FakeAuthStore, FakeMailer, FakeTemplates};
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
                gateway: None,
                auth: Some(auth),
                oauth: None,
                admin: None,
                admin_users: None,
                admin_projects: None,
                admin_notifications: None,
                projects: Some(projects),
                project_exports: None,
                project_imports: None,
                project_members: Some(members),
                audit: Some(audit),
                recipients: None,
                api_keys: None,
                templates: None,
                channel_providers: None,
                tickets: None,
                notifications: None,
                billing: None,
                schedules: None,
                runs: None,
                campaigns: None,
                inbound: None,
                provider_tester: std::sync::Arc::new(ConfigProviderTester::new())
                    as std::sync::Arc<dyn ProviderTester + Send + Sync>,
                config_store: std::sync::Arc::new(server::infra::FileConfigStore::new("configs")),
                flows: None,
                sends: None,
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

/// Signs up and returns the fresh user's id alongside the session token
/// (some tests need the account id, not just the cookie).
async fn signup_with_id(app: Router, name: &str, email: &str) -> (String, String) {
    let res = request(
        app,
        "POST",
        "/app/auth/signup",
        Some(json!({"name": name, "email": email, "password": "Sup3rSecret!"})),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    (
        body["user"]["id"].as_str().unwrap().to_string(),
        body["session"]["token"].as_str().unwrap().to_string(),
    )
}

/// Invites an existing email and accepts with that member's own token.
/// Returns the new member's user id.
async fn onboard_member(
    app: Router,
    owner_token: &str,
    member_token: &str,
    member_email: &str,
    project: &str,
    role: &str,
) -> String {
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": member_email, "role": role})),
        Some(owner_token),
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
        &format!("/app/invites/{token}/accept"),
        None,
        Some(member_token),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    body_json(res).await["member"]["userId"]
        .as_str()
        .unwrap()
        .to_string()
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
async fn owner_role_is_never_inviteable() {
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

    // No one — not even the owner — invites the owner role in; the
    // crown moves only through the ownership transfer.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "third@x.dev", "role": "owner"})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/members"),
        Some(json!({"email": "third@x.dev", "role": "owner"})),
        Some(&admin),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ownership_transfer_moves_the_crown() {
    let (app, _, _) = app();
    let owner = signup(app.clone(), "Owner", "transfer-owner@x.dev").await;
    let member = signup(app.clone(), "Member", "transfer-member@x.dev").await;
    let project = create_project(app.clone(), &owner, "Handover").await;

    let member_id = onboard_member(
        app.clone(),
        &owner,
        &member,
        "transfer-member@x.dev",
        &project,
        "developer",
    )
    .await;

    // The transfer lands on the accepted member.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/ownership"),
        Some(json!({"receiver": member_id})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["owner"]["role"], "owner");
    assert_eq!(body["owner"]["userId"], member_id.as_str());

    // The roster shows the receiver crowned and the previous owner
    // demoted to admin.
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
    let owner_role = |uid: &Value| uid["userId"].as_str().unwrap().to_string();
    let crowned = members
        .iter()
        .find(|m| owner_role(m) == member_id)
        .expect("new owner on roster");
    assert_eq!(crowned["role"], "owner");
    let demoted = members
        .iter()
        .find(|m| owner_role(m) != member_id)
        .expect("previous owner still on roster");
    assert_eq!(demoted["role"], "admin");
    let former_owner_id = owner_role(demoted);

    // The new owner (member token) sees the same roster.
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/members"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["members"].as_array().unwrap().len(), 2);

    // Everyone else — including the demoted former owner — is refused
    // the transfer power.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/ownership"),
        Some(json!({"receiver": former_owner_id})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn ownership_transfer_requires_2fa_when_the_flag_is_on() {
    let (app, _, _) = app();
    let owner = signup(app.clone(), "Owner", "2fa-transfer-owner@x.dev").await;
    let member = signup(app.clone(), "Member", "2fa-transfer-member@x.dev").await;
    let project = create_project(app.clone(), &owner, "Gated Handover").await;

    let member_id = onboard_member(
        app.clone(),
        &owner,
        &member,
        "2fa-transfer-member@x.dev",
        &project,
        "developer",
    )
    .await;

    // Owner flips the require-2FA flag on after the member already
    // joined — the join gate only lives at accept time, so the roster
    // keeps them; the handover is what gets gated now.
    let res = request(
        app.clone(),
        "PATCH",
        &format!("/app/projects/{project}/require-2fa"),
        Some(json!({"enabled": true})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // The handover is refused while the receiving account flies without
    // 2FA — receiving the crown is the strongest way to join.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/ownership"),
        Some(json!({"receiver": member_id})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let body = body_json(res).await;
    assert!(
        body["detail"].as_str().unwrap().contains("2FA"),
        "unexpected detail: {body}"
    );

    // The crown never moved: the roster still crowns the caller.
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/members"),
        None,
        Some(&owner),
    )
    .await;
    let members = body_json(res).await["members"].clone();
    assert!(
        members
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["role"] == "owner"),
        "an owner still exists on the roster"
    );

    // Once the receiver enables 2FA on their own account, the handover
    // goes through.
    enable_totp(app.clone(), &member).await;
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/ownership"),
        Some(json!({"receiver": member_id})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["owner"]["userId"], member_id.as_str());
}

#[tokio::test]
async fn transfer_requires_an_accepted_member() {
    let (app, _, _) = app();
    let (owner_id, owner) = signup_with_id(app.clone(), "Owner", "hands@x.dev").await;
    let (outsider_id, outsider) = signup_with_id(app.clone(), "Outsider", "outsider1@x.dev").await;
    let project = create_project(app.clone(), &owner, "Gate").await;

    // A signed-up account that never accepted an invite holds no crown.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/ownership"),
        Some(json!({"receiver": outsider_id})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // Transferring to yourself is a validation error.
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/ownership"),
        Some(json!({"receiver": owner_id})),
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // ...and a garbage receiver id never parses (id parse precedes the
    // owner check in the handler).
    let res = request(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/ownership"),
        Some(json!({"receiver": "not-a-user-id"})),
        Some(&outsider),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_owner_leaves_only_after_a_transfer() {
    let (app, _, _) = app();
    let owner = signup(app.clone(), "Owner", "leaver@x.dev").await;
    let member = signup(app.clone(), "Member", "leaver-m1@x.dev").await;
    let project = create_project(app.clone(), &owner, "Exit").await;

    let member_id = onboard_member(
        app.clone(),
        &owner,
        &member,
        "leaver-m1@x.dev",
        &project,
        "developer",
    )
    .await;

    // The owner cannot leave before handing over the crown.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/me"),
        None,
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // A member leaves freely.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/me"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // ...and vanishes from the owner's roster.
    let res = request(
        app.clone(),
        "GET",
        &format!("/app/projects/{project}/members"),
        None,
        Some(&owner),
    )
    .await;
    let member_ids = body_json(res).await["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["userId"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(!member_ids.contains(&member_id));

    // Leaving twice changes nothing but the story: 404 now.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/me"),
        None,
        Some(&member),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn kick_rules_owner_and_admin() {
    let (app, _, _) = app();
    let (owner_id, owner) = signup_with_id(app.clone(), "Owner", "kick-owner@x.dev").await;
    let admin1 = signup(app.clone(), "Admin1", "kick-admin1@x.dev").await;
    let admin2 = signup(app.clone(), "Admin2", "kick-admin2@x.dev").await;
    let plain = signup(app.clone(), "Plain", "kick-plain@x.dev").await;
    let viewer = signup(app.clone(), "Viewer", "kick-viewer@x.dev").await;
    let project = create_project(app.clone(), &owner, "Kicks").await;

    let admin_id = onboard_member(
        app.clone(),
        &owner,
        &admin1,
        "kick-admin1@x.dev",
        &project,
        "admin",
    )
    .await;
    let second_admin = onboard_member(
        app.clone(),
        &owner,
        &admin2,
        "kick-admin2@x.dev",
        &project,
        "admin",
    )
    .await;
    let plain_id = onboard_member(
        app.clone(),
        &owner,
        &plain,
        "kick-plain@x.dev",
        &project,
        "developer",
    )
    .await;
    let _viewer_id = onboard_member(
        app.clone(),
        &owner,
        &viewer,
        "kick-viewer@x.dev",
        &project,
        "viewer",
    )
    .await;

    // A viewer has no removal power.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/{plain_id}"),
        None,
        Some(&viewer),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // An admin removes a plain member...
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/{plain_id}"),
        None,
        Some(&admin1),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // ...but never another admin.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/{second_admin}"),
        None,
        Some(&admin1),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // ...and the owner is untouchable short of a transfer.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/{owner_id}"),
        None,
        Some(&admin1),
    )
    .await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    // The owner removes anyone, including an admin.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/{admin_id}"),
        None,
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // The owner removing themselves through the kick door is rejected:
    // leaving has its own endpoint.
    let res = request(
        app.clone(),
        "DELETE",
        &format!("/app/projects/{project}/members/{owner_id}"),
        None,
        Some(&owner),
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
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
