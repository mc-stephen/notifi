//! End-to-end HTTP tests for campaigns.
//!
//! Covers template-first compose (per-channel snapshot, template-only
//! validation), the flow-less delivery door (campaign rows deliver
//! directly through the project's connected providers — flows are never
//! consulted), expansion into per-recipient sends (contact values, not
//! targeting keys), audience scope (all vs custom), scheduled firing
//! through the worker tick, batched draining with pause/resume/cancel
//! semantics, patch/window rules, derived totals, and cross-project
//! isolation. Adapter behavior is scripted via a stub factory.

mod common;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::common::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeBillingStore, FakeCampaignStore,
    FakeChannelProviderStore, FakeFlowStore, FakeHttpRequester, FakeRecipientsStore, FakeSendStore,
    FakeTemplatesStore,
};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use server::api::build_router;
use server::api::state::AppState;
use server::domain::admin::AdminService;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::billing::BillingService;
use server::domain::campaigns::CampaignService;
use server::domain::campaigns::services::BATCH_LIMIT;
use server::domain::flows::FlowService;
use server::domain::projects::ProjectService;
use server::domain::sends::SendService;
use server::domain::templates::TemplateService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::AdapterFactory;
use server::ports::ProviderTester;
use server::ports::recipients_store::RecipientsStore as _;
use server::ports::{
    BoxSendFuture, DeliveryError, DeliveryMessage, DeliveryProvider, DeliveryReceipt,
};
use tower::ServiceExt;

struct StubAdapter {
    succeed: bool,
}

impl DeliveryProvider for StubAdapter {
    fn channel(&self) -> &'static str {
        "stub"
    }

    fn provider_name(&self) -> &'static str {
        "stub"
    }

    fn send(&self, _message: &DeliveryMessage) -> BoxSendFuture<'_> {
        let succeed = self.succeed;
        Box::pin(async move {
            if succeed {
                Ok(DeliveryReceipt {
                    provider: "stub".to_string(),
                    provider_message_id: Some("mid-1".to_string()),
                })
            } else {
                Err(DeliveryError::Provider("boom".to_string()))
            }
        })
    }
}

struct StubFactory {
    fail: Mutex<HashSet<String>>,
}

impl StubFactory {
    fn succeeding() -> Self {
        Self {
            fail: Mutex::new(HashSet::new()),
        }
    }
}

impl AdapterFactory for StubFactory {
    fn adapter(
        &self,
        _channel: &str,
        provider_id: &str,
        _config: &serde_json::Value,
    ) -> Option<Arc<dyn DeliveryProvider>> {
        if provider_id == "ghost" {
            return None;
        }
        let succeed = !self.fail.lock().unwrap().contains(provider_id);
        Some(Arc::new(StubAdapter { succeed }))
    }
}

struct Fakes {
    flows: Arc<FakeFlowStore>,
    campaigns: Arc<FakeCampaignStore>,
    sends: Arc<FakeSendStore>,
    recipients: Arc<FakeRecipientsStore>,
    channels: Arc<FakeChannelProviderStore>,
    templates: Arc<FakeTemplatesStore>,
    /// The service handle behind the app — for direct worker `tick` calls.
    service: Arc<CampaignService>,
}

#[allow(clippy::too_many_lines)]
fn app_with_campaigns() -> (Router, Fakes) {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(auth_store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(auth_store.clone(), audit.clone()));
    let admin = Arc::new(AdminService::new(
        Box::new(FakeAdminStore::new()),
        true,
        audit.clone(),
    ));
    let billing = Arc::new(BillingService::new(
        Arc::new(FakeBillingStore::new()),
        audit.clone(),
    ));
    let flow_store = Arc::new(FakeFlowStore::new());
    let flows = Arc::new(FlowService::new(flow_store.clone()));
    let channel_providers = Arc::new(FakeChannelProviderStore::new());
    let send_store = Arc::new(FakeSendStore::new());
    let http = Arc::new(FakeHttpRequester::new());
    let sends = Arc::new(SendService::new(
        send_store.clone(),
        flows.clone(),
        channel_providers.clone(),
        Arc::new(StubFactory::succeeding()),
        http.clone(),
    ));
    let campaign_store = Arc::new(FakeCampaignStore::new());
    let recipients = Arc::new(FakeRecipientsStore::new());
    let template_store = Arc::new(FakeTemplatesStore::new());
    let templates = Arc::new(TemplateService::new(template_store.clone(), audit.clone()));
    let campaigns = Arc::new(CampaignService::new(
        campaign_store.clone(),
        send_store.clone(),
        sends.clone(),
        recipients.clone(),
        template_store.clone(),
    ));
    let app = build_router(
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
            api_keys: None,
            templates: Some(templates),
            channel_providers: Some(channel_providers.clone()),
            tickets: None,
            billing: Some(billing),
            flows: Some(flows),
            sends: Some(sends),
            schedules: None,
            webhooks: None,
            campaigns: Some(campaigns.clone()),
            notifications: None,
            inbound: None,
            provider_tester: Arc::new(ConfigProviderTester::new())
                as Arc<dyn ProviderTester + Send + Sync>,
        },
        &AppConfig::default(),
    );
    (
        app,
        Fakes {
            flows: flow_store,
            campaigns: campaign_store,
            sends: send_store,
            recipients,
            channels: channel_providers,
            templates: template_store,
            service: campaigns,
        },
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    use http_body_util::BodyExt;
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
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
        token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["project"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn setup_project(
    app: Router,
    fakes: &Fakes,
    email: &str,
    name: &str,
) -> (String, String, String) {
    let (token, user_id) = signup(app.clone(), email).await;
    let project = create_project(app.clone(), &token, name).await;
    fakes.flows.seed_visible(&user_id, &project);
    fakes.campaigns.seed_visible(&user_id, &project);
    fakes.sends.seed_visible(&user_id, &project);
    fakes.recipients.seed_visible(&user_id, &project);
    fakes.templates.seed_visible(&user_id, &project);
    fakes.channels.seed_visible(&user_id, &project);
    fakes.channels.seed_config(&project, "email", "smtp");
    (token, user_id, project)
}

/// Seeds a recipient whose `contacts.email` is the channel contact; the
/// send row must later carry the email VALUE, not `user_id`.
async fn seed_email_recipient(
    fakes: &Fakes,
    project: &str,
    actor: &str,
    user_id: &str,
    email: &str,
) {
    use server::domain::auth::entities::UserId;
    let actor = actor.parse::<UserId>().unwrap();
    fakes
        .recipients
        .create(actor, project, user_id, "Ada", json!({"email": email}))
        .await
        .unwrap();
}

/// Creates a template via the HTTP surface (campaigns compose from
/// templates by design).
async fn create_template(app: Router, token: &str, project: &str, content: Value) -> String {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/projects/{project}/templates"),
        Some(json!({
            "name": "Release notes",
            "channel": "email",
            "content": content,
        })),
        token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["template"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create_campaign(app: Router, token: &str, body: Value) -> (StatusCode, Value) {
    let res = request_with_cookie(app.clone(), "POST", "/app/campaigns", Some(body), token).await;
    let status = res.status();
    (status, body_json(res).await)
}

async fn campaign_action(
    app: Router,
    token: &str,
    id: &str,
    action: &str,
    project: &str,
) -> (StatusCode, Value) {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/campaigns/{id}/{action}"),
        Some(json!({"projectId": project})),
        token,
    )
    .await;
    let status = res.status();
    (status, body_json(res).await)
}

/// Template-first compose body (all-mode audience by default). Channel
/// is explicit — the doctrine demands naming the template version.
fn draft_body(project: &str, template_id: &str) -> Value {
    json!({
        "projectId": project,
        "name": "April release notes",
        "templateId": template_id,
        "channel": "email",
        "description": "Shipped bits",
    })
}

#[tokio::test]
async fn compose_and_crud_round_trip() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    seed_email_recipient(&fakes, &project, &actor, "user-1", "ada@example.com").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "What shipped in April", "text": "Lots of things!"}}),
    )
    .await;

    let (status, body) =
        create_campaign(app.clone(), &token, draft_body(&project, &template_id)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let campaign = &body["campaign"];
    assert_eq!(campaign["status"], "draft");
    assert_eq!(campaign["channel"], "email");
    assert_eq!(campaign["name"], "April release notes");
    // Compose-time snapshot of the render.
    assert_eq!(campaign["title"], "What shipped in April");
    assert_eq!(campaign["body"], "Lots of things!");
    // Audience estimate = channel-capable recipients (all-mode).
    assert_eq!(campaign["estimatedAudience"], 1);
    assert_eq!(campaign["audienceMode"], "all");
    let id = campaign["id"].as_str().unwrap().to_string();

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["campaign"]["totals"]["total"], 0);

    // PATCH: rename + description; the copy is untouched (template-only).
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({
            "projectId": project,
            "name": "May release notes",
            "description": "Renamed",
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let patched = body_json(res).await["campaign"].clone();
    assert_eq!(patched["name"], "May release notes");
    assert_eq!(patched["description"], "Renamed");
    assert_eq!(patched["title"], "What shipped in April");

    // DELETE.
    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/campaigns/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["deleted"], true);

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_validates_compose() {
    let (app, fakes) = app_with_campaigns();
    let (token, _actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    // No template → 400 (template compose by design).
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({"projectId": project, "name": "No template"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // Template without a channel version → 400 (channel is explicit).
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "No channel",
            "templateId": template_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["detail"]
            .as_str()
            .unwrap()
            .contains("pick which version"),
        "{body}"
    );

    // Unknown template → 400.
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Ghost template",
            "templateId": "tpl_00000000000000000000000000",
            "channel": "email",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // A channel the template does not cover → 400.
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Wrong channel",
            "templateId": template_id,
            "channel": "sms",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["detail"]
            .as_str()
            .unwrap()
            .contains("no content for sms"),
        "{body}"
    );

    // Schedule time must be in the future.
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Past",
            "templateId": template_id,
            "channel": "email",
            "sendAt": "2020-01-01T00:00:00Z",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    // Custom audiences need at least one selection.
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Empty custom",
            "templateId": template_id,
            "channel": "email",
            "audienceMode": "custom",
            "selectedUserIds": [],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn run_now_fails_rows_without_connected_provider() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    // A recipient reachable on sms — but no sms provider is connected
    // (setup only seeds the email config).
    use server::domain::auth::entities::UserId;
    let actor_id = actor.parse::<UserId>().unwrap();
    fakes
        .recipients
        .create(
            actor_id,
            &project,
            "user-1",
            "Ada",
            json!({"phone": "+15550001111"}),
        )
        .await
        .unwrap();
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"sms": {"sms": "Server down"}}),
    )
    .await;

    let (_, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "No provider",
            "templateId": template_id,
            "channel": "sms",
        }),
    )
    .await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    // Campaigns are flow-less and never pre-gate: the run succeeds at
    // expanding, then every row fails closed with the Providers-page
    // guidance. The campaign still completes (rows settled).
    let (status, body) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let campaign = &body["campaign"];
    assert_eq!(campaign["status"], "completed");
    assert_eq!(campaign["totals"]["total"], 1);
    assert_eq!(campaign["totals"]["delivered"], 0);
    assert_eq!(campaign["totals"]["failed"], 1);

    let sends = fakes.sends.all();
    let campaign_sends: Vec<_> = sends
        .iter()
        .filter(|s| s.campaign_id.as_deref() == Some(id.as_str()))
        .collect();
    assert_eq!(campaign_sends.len(), 1);
    assert_eq!(campaign_sends[0].status, "failed");
    let error = campaign_sends[0].last_error.as_deref().unwrap_or_default();
    assert!(
        error.contains("no provider connected for channel 'sms'"),
        "{error}"
    );
}

#[tokio::test]
async fn run_now_delivers_with_connected_provider_and_no_flows() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    seed_email_recipient(&fakes, &project, &actor, "user-1", "ada@example.com").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    // No flow exists anywhere in this test — delivery resolves purely
    // from the seeded provider config.
    let (_, body) = create_campaign(app.clone(), &token, draft_body(&project, &template_id)).await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    let (status, body) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let campaign = &body["campaign"];
    assert_eq!(campaign["status"], "completed");
    assert_eq!(campaign["totals"]["delivered"], 1);

    let sends = fakes.sends.all();
    let delivered: Vec<_> = sends
        .iter()
        .filter(|s| s.campaign_id.as_deref() == Some(id.as_str()) && s.status == "completed")
        .collect();
    assert_eq!(delivered.len(), 1);
    assert_eq!(
        delivered[0].recipients[0].as_str().unwrap(),
        "ada@example.com"
    );
}

#[tokio::test]
async fn run_now_tries_the_picked_provider_first() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    // Store order puts smtp first; the campaign names ses as primary.
    fakes.channels.seed_config(&project, "email", "ses");
    seed_email_recipient(&fakes, &project, &actor, "user-1", "ada@example.com").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (_, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Pinned",
            "templateId": template_id,
            "channel": "email",
            "providerId": "ses",
        }),
    )
    .await;
    let campaign = &body["campaign"];
    assert_eq!(campaign["providerId"], "ses", "{body}");
    let id = campaign["id"].as_str().unwrap().to_string();

    let (status, body) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["campaign"]["totals"]["delivered"], 1, "{body}");

    // The winning provider is the picked "ses" — store order would have
    // delivered through smtp. "ses" is also the fallback lead of the
    // chain, proving the promotion happened.
    let row = fakes
        .sends
        .all()
        .into_iter()
        .find(|s| s.campaign_id.as_deref() == Some(id.as_str()))
        .unwrap();
    assert_eq!(row.status, "completed");
    assert_eq!(row.provider.as_deref(), Some("ses"));
}

#[tokio::test]
async fn picked_provider_without_a_sender_falls_back() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    // "ghost" has a connected config but no sender implementation; the
    // pick still stores and the chain hops to the next provider.
    fakes.channels.seed_config(&project, "email", "ghost");
    seed_email_recipient(&fakes, &project, &actor, "user-1", "ada@example.com").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (_, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Ghost pick",
            "templateId": template_id,
            "channel": "email",
            "providerId": "ghost",
        }),
    )
    .await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    let (status, body) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["campaign"]["totals"]["delivered"], 1, "{body}");

    // Ghost was tried first (no sender), then smtp delivered.
    let row = fakes
        .sends
        .all()
        .into_iter()
        .find(|s| s.campaign_id.as_deref() == Some(id.as_str()))
        .unwrap();
    assert_eq!(row.status, "completed");
    assert_eq!(row.provider.as_deref(), Some("smtp"));
}

#[tokio::test]
async fn run_now_expands_delivers_and_completes() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    seed_email_recipient(&fakes, &project, &actor, "user-1", "ada@example.com").await;
    seed_email_recipient(&fakes, &project, &actor, "user-2", "grace@example.com").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "What shipped", "text": "A lot!"}}),
    )
    .await;

    let (_, body) = create_campaign(app.clone(), &token, draft_body(&project, &template_id)).await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    let (status, body) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let campaign = &body["campaign"];
    // Two rows, both delivered in the first batch (2 ≤ BATCH_LIMIT), so
    // the campaign completes right away.
    assert_eq!(campaign["status"], "completed");
    assert_eq!(campaign["totals"]["total"], 2);
    assert_eq!(campaign["totals"]["delivered"], 2);
    assert_eq!(campaign["totals"]["pending"], 0);
    assert!(campaign["startedAt"].is_string());
    assert!(campaign["completedAt"].is_string());
    // Small audiences lose the estimate once expanded.
    assert!(campaign["estimatedAudience"].is_null());

    // Expanded rows carry the contact VALUES (emails) and land completed
    // with the campaign id.
    let sends = fakes.sends.all();
    let campaign_sends: Vec<_> = sends
        .iter()
        .filter(|s| s.campaign_id.as_deref() == Some(id.as_str()))
        .collect();
    assert_eq!(campaign_sends.len(), 2);
    let mut recipients: Vec<String> = campaign_sends
        .iter()
        .map(|s| s.recipients[0].as_str().unwrap().to_string())
        .collect();
    recipients.sort();
    assert_eq!(
        recipients,
        vec![
            "ada@example.com".to_string(),
            "grace@example.com".to_string(),
        ]
    );
    assert!(campaign_sends.iter().all(|s| s.status == "completed"));
}

#[tokio::test]
async fn scheduled_fire_fails_without_audience() {
    let (app, fakes) = app_with_campaigns();
    let (token, _actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "No audience",
            "templateId": template_id,
            "channel": "email",
            "sendAt": "2030-01-01T00:00:00Z",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["campaign"]["status"], "scheduled");
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    // The worker claims it, finds no recipients, and fails the campaign
    // with the guidance in last_error.
    let now = chrono::DateTime::parse_from_rfc3339("2030-01-01T02:00:00Z")
        .unwrap()
        .into();
    fakes.service.tick(now).await.unwrap();

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    let campaign = body_json(res).await["campaign"].clone();
    assert_eq!(campaign["status"], "failed");
    assert!(
        campaign["lastError"]
            .as_str()
            .unwrap()
            .contains("audience is empty"),
        "{}",
        campaign["lastError"]
    );
}

#[tokio::test]
async fn scheduled_fire_expands_completes_and_claims_once() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    seed_email_recipient(&fakes, &project, &actor, "user-1", "ada@example.com").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Scheduled",
            "templateId": template_id,
            "channel": "email",
            "sendAt": "2030-01-01T00:00:00Z",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    let now = chrono::DateTime::parse_from_rfc3339("2030-01-01T02:00:00Z")
        .unwrap()
        .into();
    fakes.service.tick(now).await.unwrap();

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    let campaign = body_json(res).await["campaign"].clone();
    assert_eq!(campaign["status"], "completed");
    assert_eq!(campaign["totals"]["total"], 1);
    assert_eq!(campaign["totals"]["delivered"], 1);

    // A second tick must not re-claim or re-send anything.
    fakes.service.tick(now).await.unwrap();
    let sends: Vec<_> = fakes
        .sends
        .all()
        .into_iter()
        .filter(|s| s.campaign_id.as_deref() == Some(id.as_str()))
        .collect();
    assert_eq!(sends.len(), 1);
}

#[tokio::test]
async fn pause_resume_cancel_round() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    // One recipient more than the first batch, so expansion leaves a
    // pending remainder for the worker.
    let total = BATCH_LIMIT + 1;
    for i in 0..total {
        seed_email_recipient(
            &fakes,
            &project,
            &actor,
            &format!("user-{i}"),
            &format!("r{i}@example.com"),
        )
        .await;
    }
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (_, body) = create_campaign(app.clone(), &token, draft_body(&project, &template_id)).await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    // run-now drains the first batch only; the campaign stays running.
    let (status, body) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["campaign"]["status"], "running");
    assert_eq!(body["campaign"]["totals"]["delivered"], BATCH_LIMIT);
    assert_eq!(body["campaign"]["totals"]["pending"], 1);

    // DELETE refuses while running.
    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/campaigns/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);

    // Pause stops the drain.
    let (status, body) = campaign_action(app.clone(), &token, &id, "pause", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["campaign"]["status"], "paused");

    // Resume hands back to the sweep: the next tick completes it.
    let (status, body) = campaign_action(app.clone(), &token, &id, "resume", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["campaign"]["status"], "running");
    let now = chrono::Utc::now();
    fakes.service.tick(now).await.unwrap();
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(body_json(res).await["campaign"]["status"], "completed");

    // A second drain never happened beyond the one pending row.
    let sends: Vec<_> = fakes
        .sends
        .all()
        .into_iter()
        .filter(|s| s.campaign_id.as_deref() == Some(id.as_str()))
        .collect();
    assert_eq!(sends.len(), total);
}

#[tokio::test]
async fn cancel_drops_pending_and_marks_terminal() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    let total = BATCH_LIMIT + 1;
    for i in 0..total {
        seed_email_recipient(
            &fakes,
            &project,
            &actor,
            &format!("user-{i}"),
            &format!("r{i}@example.com"),
        )
        .await;
    }
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (_, body) = create_campaign(app.clone(), &token, draft_body(&project, &template_id)).await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();
    let (status, _) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = campaign_action(app.clone(), &token, &id, "cancel", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["campaign"]["status"], "cancelled");
    assert_eq!(body["campaign"]["totals"]["total"], BATCH_LIMIT);
    assert_eq!(body["campaign"]["totals"]["pending"], 0);

    // Only delivered rows survive cancel; nothing else is pending.
    let pending = fakes
        .sends
        .all()
        .into_iter()
        .filter(|s| s.campaign_id.as_deref() == Some(id.as_str()) && s.status == "pending")
        .count();
    assert_eq!(pending, 0);

    // Terminal: further actions refuse.
    let (status, _) = campaign_action(app.clone(), &token, &id, "pause", &project).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn patch_windows_and_reschedule() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    let total = BATCH_LIMIT + 1;
    for i in 0..total {
        seed_email_recipient(
            &fakes,
            &project,
            &actor,
            &format!("user-{i}"),
            &format!("r{i}@example.com"),
        )
        .await;
    }
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    // Scheduled campaign: reschedule via PATCH.
    let (_, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Later",
            "templateId": template_id,
            "channel": "email",
            "sendAt": "2030-01-01T00:00:00Z",
        }),
    )
    .await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({
            "projectId": project,
            "sendAt": "2031-06-01T00:00:00Z",
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let campaign = body_json(res).await["campaign"].clone();
    assert_eq!(campaign["status"], "scheduled");
    assert_eq!(campaign["sendAt"], "2031-06-01T00:00:00Z");

    // Unschedule turns it back into a draft.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({
            "projectId": project,
            "sendAt": Value::Null,
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["campaign"]["status"], "draft");

    // Fire it: PATCH windows close while running.
    let (status, _) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK);
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({"projectId": project, "name": "Nope"})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn patch_re_snapshots_the_channel() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    seed_email_recipient(&fakes, &project, &actor, "user-1", "ada@example.com").await;
    let email_only = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "A mail", "text": "A body"}}),
    )
    .await;
    let both = create_template(
        app.clone(),
        &token,
        &project,
        json!({
            "email": {"subject": "B mail", "text": "B body"},
            "sms": {"sms": "B short"},
        }),
    )
    .await;

    let (_, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Split",
            "templateId": email_only,
            "channel": "email",
            "audienceMode": "custom",
            "selectedUserIds": ["user-1"],
        }),
    )
    .await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    // Template patch WITHOUT a channel re-pick: the stored channel
    // carries over and must still be covered.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({
            "projectId": project,
            "templateId": both,
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK, "{}", body_json(res).await);
    let campaign = body_json(res).await["campaign"].clone();
    assert_eq!(campaign["channel"], "email");
    assert_eq!(campaign["title"], "B mail");

    // Channel re-pick: re-snapshots from the bound template.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({
            "projectId": project,
            "channel": "sms",
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK, "{}", body_json(res).await);
    let campaign = body_json(res).await["campaign"].clone();
    assert_eq!(campaign["channel"], "sms");
    assert_eq!(campaign["body"], "B short");

    // A channel the bound template does not cover → 400.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({
            "projectId": project,
            "channel": "push",
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Template switch back to the email-only template: the stored
    // channel (sms) now falls through uncovered → honest 400.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/campaigns/{id}"),
        Some(json!({
            "projectId": project,
            "templateId": email_only,
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let detail = body_json(res).await["detail"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(detail.contains("no content for sms"), "{detail}");
}

#[tokio::test]
async fn cross_project_isolation() {
    let (app, fakes) = app_with_campaigns();
    let (token_a, _actor_a, project_a) = setup_project(app.clone(), &fakes, "a@x.com", "A").await;
    let (token_b, _actor_b, project_b) = setup_project(app.clone(), &fakes, "b@x.com", "B").await;
    let template_a = create_template(
        app.clone(),
        &token_a,
        &project_a,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (_, body) =
        create_campaign(app.clone(), &token_a, draft_body(&project_a, &template_a)).await;
    let id = body["campaign"]["id"].as_str().unwrap().to_string();

    // B reads A's campaign → not found.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns/{id}?project_id={project_a}"),
        None,
        &token_b,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // B runs an action on A's campaign → not found.
    let (status, _) = campaign_action(app.clone(), &token_b, &id, "run-now", &project_b).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A's campaign list holds one; B's holds none.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns?project_id={project_a}"),
        None,
        &token_a,
    )
    .await;
    assert_eq!(
        body_json(res).await["campaigns"].as_array().unwrap().len(),
        1
    );
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/campaigns?project_id={project_b}"),
        None,
        &token_b,
    )
    .await;
    assert_eq!(
        body_json(res).await["campaigns"].as_array().unwrap().len(),
        0
    );
}

#[tokio::test]
async fn template_compose_snapshots_render() {
    let (app, fakes) = app_with_campaigns();
    let (token, _actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "What shipped", "text": "A lot!"}}),
    )
    .await;

    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "From template",
            "templateId": template_id,
            "channel": "email",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let campaign = &body["campaign"];
    // Snapshot: channel + title + body ride from the named channel's
    // render — there is no default preference to fall back to.
    assert_eq!(campaign["channel"], "email");
    assert_eq!(campaign["title"], "What shipped");
    assert_eq!(campaign["body"], "A lot!");
    assert_eq!(campaign["templateId"], template_id);
}

#[tokio::test]
async fn per_channel_snapshot_follows_the_pick() {
    let (app, fakes) = app_with_campaigns();
    let (token, _actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({
            "email": {"subject": "What shipped", "text": "A lot!"},
            "sms": {"sms": "A lot, but short"},
        }),
    )
    .await;

    // Explicit email pick → email's fields.
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Email pick",
            "templateId": template_id,
            "channel": "email",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["campaign"]["channel"], "email");
    assert_eq!(body["campaign"]["title"], "What shipped");

    // Explicit channel → that channel's fields.
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Sms pick",
            "templateId": template_id,
            "channel": "sms",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["campaign"]["channel"], "sms");
    // SMS has no title field — the body carries the message alone.
    assert_eq!(body["campaign"]["title"], "");
    assert_eq!(body["campaign"]["body"], "A lot, but short");

    // Uncovered channel → 400.
    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Push pick",
            "templateId": template_id,
            "channel": "push",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

#[tokio::test]
async fn custom_audience_expands_only_selected() {
    let (app, fakes) = app_with_campaigns();
    let (token, actor, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    for (key, email) in [
        ("user-1", "ada@example.com"),
        ("user-2", "grace@example.com"),
        ("user-3", "linus@example.com"),
    ] {
        seed_email_recipient(&fakes, &project, &actor, key, email).await;
    }
    let template_id = create_template(
        app.clone(),
        &token,
        &project,
        json!({"email": {"subject": "Hi", "text": "There"}}),
    )
    .await;

    let (status, body) = create_campaign(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Selected",
            "templateId": template_id,
            "channel": "email",
            "audienceMode": "custom",
            "selectedUserIds": ["user-1", "user-3"],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let campaign = &body["campaign"];
    assert_eq!(campaign["audienceMode"], "custom");
    assert_eq!(campaign["estimatedAudience"], 2);
    let id = campaign["id"].as_str().unwrap().to_string();

    let (status, body) = campaign_action(app.clone(), &token, &id, "run-now", &project).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["campaign"]["totals"]["total"], 2);
    assert_eq!(body["campaign"]["totals"]["delivered"], 2);

    // Exactly the selected recipients' contacts; user-2 never gets a row.
    let mut recipients: Vec<String> = fakes
        .sends
        .all()
        .into_iter()
        .filter(|s| s.campaign_id.as_deref() == Some(id.as_str()))
        .map(|s| s.recipients[0].as_str().unwrap().to_string())
        .collect();
    recipients.sort();
    assert_eq!(
        recipients,
        vec![
            "ada@example.com".to_string(),
            "linus@example.com".to_string(),
        ]
    );
}
