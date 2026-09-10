//! End-to-end HTTP tests for billing (fake-backed services).
//!
//! Covers the user surface (catalog, free-by-default subscription,
//! subscribe/change/cancel/renew, retired-plan refusal, isolation) and
//! the admin plan catalog (CRUD + delete guard).

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
use server::domain::billing::BillingService;
use server::domain::projects::ProjectService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeBillingStore};
use tower::ServiceExt;

fn app_with_billing() -> (Router, Arc<FakeBillingStore>) {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(
        Box::new(FakeAdminStore::new()),
        true,
        audit.clone(),
    ));
    let billing_store = Arc::new(FakeBillingStore::new());
    let billing = Arc::new(BillingService::new(billing_store.clone(), audit.clone()));
    (
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
                tickets: None,
                notifications: None,
                billing: Some(billing),
                provider_tester: Arc::new(ConfigProviderTester::new())
                    as Arc<dyn ProviderTester + Send + Sync>,
            },
            &AppConfig::default(),
        ),
        billing_store,
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
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

async fn signup(app: Router, email: &str) -> (String, String) {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/auth/signup",
        Some(json!({"name": "Pat", "email": email, "password": "Sup3rSecret!"})),
        "session_token",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    let token = body["session"]["token"].as_str().unwrap().to_string();
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    (token, user_id)
}

async fn create_project(app: Router, token: &str, name: &str) -> String {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/projects",
        Some(json!({"name": name})),
        "session_token",
        token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["project"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn session_cookie_from(res: &axum::response::Response, name: &str) -> String {
    res.headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .strip_prefix(&format!("{name}="))
        .unwrap_or("")
        .to_string()
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
    session_cookie_from(&res, "admin_session")
}

#[tokio::test]
async fn new_project_defaults_to_free_subscription() {
    let (app, billing) = app_with_billing();
    let (token, user_id) = signup(app.clone(), "free@x.dev").await;
    let project = create_project(app.clone(), &token, "Website").await;
    billing.seed_visible(&user_id, &project);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/billing/subscription?project_id={project}"),
        None,
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sub = &body_json(res).await["subscription"];
    assert_eq!(sub["plan"]["id"], "free");
    assert_eq!(sub["status"], "active");
    assert_eq!(sub["billingCycle"], "monthly");
    assert_eq!(sub["usage"]["notificationsUsed"], 0);
    assert_eq!(sub["usage"]["notificationsLimit"], 1000);
}

#[tokio::test]
async fn plans_catalog_lists_active_plans_with_yearly_math() {
    let (app, _) = app_with_billing();
    let (token, _) = signup(app.clone(), "catalog@x.dev").await;

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/app/billing/plans",
        None,
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let plans = body_json(res).await["plans"].clone();
    let plans = plans.as_array().unwrap();
    assert_eq!(plans.len(), 4);
    assert_eq!(plans[0]["id"], "free");
    // Pro: 9900 * 12 * 0.8 = 95040.
    let pro = plans.iter().find(|p| p["id"] == "pro").unwrap();
    assert_eq!(pro["yearlyPriceCents"], 95040);
    // Enterprise is custom-priced: no computed totals.
    let ent = plans.iter().find(|p| p["id"] == "enterprise").unwrap();
    assert!(ent["priceCents"].is_null());
    assert!(ent["yearlyPriceCents"].is_null());
}

#[tokio::test]
async fn subscribe_change_cancel_and_revive() {
    let (app, billing) = app_with_billing();
    let (token, user_id) = signup(app.clone(), "sub@x.dev").await;
    let project = create_project(app.clone(), &token, "App").await;
    billing.seed_visible(&user_id, &project);

    // Subscribe to pro, yearly.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "pro", "billingCycle": "yearly"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sub = &body_json(res).await["subscription"];
    assert_eq!(sub["plan"]["id"], "pro");
    assert_eq!(sub["billingCycle"], "yearly");
    assert_eq!(sub["periodPriceCents"], 95040);

    // Change to starter monthly.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "starter"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sub = &body_json(res).await["subscription"];
    assert_eq!(sub["plan"]["id"], "starter");
    assert_eq!(sub["periodPriceCents"], 1900);

    // Cancel marks end-of-period: plan + status stay until period end.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions/cancel",
        Some(json!({"projectId": project})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sub = body_json(res).await["subscription"].clone();
    assert_eq!(sub["plan"]["id"], "starter");
    assert_eq!(sub["status"], "active");
    assert_eq!(sub["cancelAtPeriodEnd"], true);

    // Re-subscribing clears the pending cancel.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "starter"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sub = body_json(res).await["subscription"].clone();
    assert_eq!(sub["status"], "active");
    assert_eq!(sub["cancelAtPeriodEnd"], false);
}

#[tokio::test]
async fn free_plan_cannot_be_cancelled() {
    let (app, billing) = app_with_billing();
    let (token, user_id) = signup(app.clone(), "freecancel@x.dev").await;
    let project = create_project(app.clone(), &token, "Freebie").await;
    billing.seed_visible(&user_id, &project);

    // Fresh projects sit on free: cancelling is refused.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions/cancel",
        Some(json!({"projectId": project})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn expired_pending_cancel_reverts_to_free() {
    use server::domain::billing::entities::{BillingCycle, SubscriptionStatus};
    use server::ports::billing_store::BillingStore;

    let (app, billing) = app_with_billing();
    let (token, user_id) = signup(app.clone(), "revert@x.dev").await;
    let project = create_project(app.clone(), &token, "Reverter").await;
    billing.seed_visible(&user_id, &project);

    // Subscribe to pro, then cancel (pending) and backdate the period
    // past its end through the store.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "pro"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let past = chrono::Utc::now() - chrono::Duration::days(60);
    billing
        .set_subscription(
            &project,
            "pro",
            SubscriptionStatus::Active,
            BillingCycle::Monthly,
            past - chrono::Duration::days(30),
            past,
        )
        .await
        .unwrap();
    billing
        .set_cancel_at_period_end(&project, true)
        .await
        .unwrap();

    // Next read reverts to free with a fresh period.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/billing/subscription?project_id={project}"),
        None,
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sub = body_json(res).await["subscription"].clone();
    assert_eq!(sub["plan"]["id"], "free");
    assert_eq!(sub["status"], "active");
    assert_eq!(sub["cancelAtPeriodEnd"], false);
}

#[tokio::test]
async fn retired_and_custom_plans_are_refused() {
    let (app, billing) = app_with_billing();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (token, user_id) = signup(app.clone(), "ret@x.dev").await;
    let project = create_project(app.clone(), &token, "Shop").await;
    billing.seed_visible(&user_id, &project);

    // Custom-priced plans need a sales conversation.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "enterprise"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Archive starter via the admin API, then subscribing fails with 422.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        "/admin/billing/plans/starter",
        Some(json!({"isActive": false})),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "starter"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // A project already on starter can't renew it either.
    let (token2, user2) = signup(app.clone(), "ret2@x.dev").await;
    let project2 = create_project(app.clone(), &token2, "Blog").await;
    billing.seed_visible(&user2, &project2);
    // Subscribe first while active... starter is archived now, so use pro.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project2, "planId": "pro"})),
        "session_token",
        &token2,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    // Archive pro, then renew is refused.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        "/admin/billing/plans/pro",
        Some(json!({"isActive": false})),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions/renew",
        Some(json!({"projectId": project2})),
        "session_token",
        &token2,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn subscriptions_are_isolated_between_users() {
    let (app, billing) = app_with_billing();
    let (alice_token, alice) = signup(app.clone(), "alice@x.dev").await;
    let (bob_token, _) = signup(app.clone(), "bob@x.dev").await;
    let project = create_project(app.clone(), &alice_token, "AliceApp").await;
    billing.seed_visible(&alice, &project);

    // Bob sees neither the subscription nor the catalog actions for it.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/billing/subscription?project_id={project}"),
        None,
        "session_token",
        &bob_token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "pro"})),
        "session_token",
        &bob_token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_manages_plans_with_delete_guard() {
    let (app, billing) = app_with_billing();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (token, user_id) = signup(app.clone(), "cust@x.dev").await;
    let project = create_project(app.clone(), &token, "Store").await;
    billing.seed_visible(&user_id, &project);

    // List carries live subscriber counts.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/plans",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["plans"].as_array().unwrap().len(), 4);

    // Create a plan.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/admin/billing/plans",
        Some(json!({
            "name": "Growth",
            "priceCents": 4900,
            "capabilities": {"notificationsPerMonth": 50000, "channels": ["email", "sms"], "teamMembers": 5, "retentionDays": 60, "support": "Priority", "branding": true, "apiCalls": null},
            "yearlyDiscount": {"kind": "percent", "value": 15},
        })),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    assert_eq!(body["plan"]["id"], "growth");
    assert_eq!(body["plan"]["activeSubscribers"], 0);

    // Subscribe the project, then delete is refused with 409.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "growth"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        "/admin/billing/plans/growth",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);

    // Archive instead, then renew is refused but the plan remains.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        "/admin/billing/plans/growth",
        Some(json!({"isActive": false})),
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["plan"]["isActive"], false);

    // Move the project back to free, then delete succeeds.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "free"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        "/admin/billing/plans/growth",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn billing_requires_auth() {
    let (app, _) = app_with_billing();
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/app/billing/plans",
        None,
        "session_token",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/plans",
        None,
        "admin_session",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_lists_plan_subscribers_with_customer_identity() {
    let (app, billing) = app_with_billing();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (token, user_id) = signup(app.clone(), "subscriber@x.dev").await;
    let project = create_project(app.clone(), &token, "Subscriber Shop").await;
    billing.seed_visible(&user_id, &project);
    billing.seed_billing_project(&project, "Subscriber Shop");
    billing.seed_billing_user(&user_id, "Pat", "subscriber@x.dev");

    // Nobody on pro yet.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/plans/pro/subscriptions",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["subscriptions"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    // Subscribe, then the drill-down shows who.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "pro"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/plans/pro/subscriptions",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let subs = body_json(res).await["subscriptions"].clone();
    let subs = subs.as_array().unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0]["projectName"], "Subscriber Shop");
    assert_eq!(subs[0]["customerEmail"], "subscriber@x.dev");
    assert_eq!(subs[0]["status"], "active");

    // Unknown plan is a 404.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/plans/nope/subscriptions",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_subscriber_history_reflects_subscriptions() {
    let (app, billing) = app_with_billing();
    let admin_cookie = bootstrap_admin(app.clone()).await;
    let (token, user_id) = signup(app.clone(), "history@x.dev").await;
    let project = create_project(app.clone(), &token, "History Shop").await;
    billing.seed_visible(&user_id, &project);

    // One project on pro: the trailing point must show it.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/billing/subscriptions",
        Some(json!({"projectId": project, "planId": "pro"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/subscribers/history?months=3",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let points = body_json(res).await["points"].clone();
    let points = points.as_array().unwrap();
    assert_eq!(points.len(), 3);
    let last = &points[2];
    assert_eq!(last["total"], 1);
    assert_eq!(last["byPlan"]["pro"], 1);

    // months clamps to the 1..24 window.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/subscribers/history?months=99",
        None,
        "admin_session",
        &admin_cookie,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["points"].as_array().unwrap().len(), 24);

    // Auth gate holds.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        "/admin/billing/subscribers/history",
        None,
        "admin_session",
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
