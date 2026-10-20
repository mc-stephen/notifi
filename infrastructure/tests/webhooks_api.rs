//! End-to-end HTTP tests for owner-managed webhooks.
//!
//! Covers webhook CRUD (create builds the owned flow with a locked
//! webhook trigger + fresh token), rename sync, event rewrites keeping
//! the token, archive/restore, the fire path (`POST /v1/hooks/{token}`
//! works for a webhook-born flow), owned flows staying off the `/flows`
//! list and dying with the webhook, and cross-project isolation.

mod common;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::common::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeBillingStore, FakeChannelProviderStore,
    FakeFlowStore, FakeHttpRequester, FakeScheduleStore, FakeSendStore, FakeTemplatesStore,
    FakeTriggerTokenStore, FakeWebhookStore,
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
use server::domain::flows::FlowService;
use server::domain::inbound::InboundService;
use server::domain::projects::ProjectService;
use server::domain::schedules::ScheduleService;
use server::domain::sends::SendService;
use server::domain::templates::TemplateService;
use server::domain::webhooks::WebhookService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::ports::{
    AdapterFactory, BoxSendFuture, DeliveryError, DeliveryMessage, DeliveryProvider,
    DeliveryReceipt,
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
    schedules: Arc<FakeScheduleStore>,
    webhooks: Arc<FakeWebhookStore>,
    sends: Arc<FakeSendStore>,
    channels: Arc<FakeChannelProviderStore>,
    templates: Arc<FakeTemplatesStore>,
}

fn app_with_webhooks() -> (Router, Fakes) {
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
    let schedule_store = Arc::new(FakeScheduleStore::new(flow_store.clone()));
    let schedules = Arc::new(ScheduleService::new(
        schedule_store.clone(),
        flow_store.clone(),
        send_store.clone(),
        Arc::new(FakeTemplatesStore::new()),
        channel_providers.clone(),
        Arc::new(StubFactory::succeeding()),
        http.clone(),
    ));
    let webhook_store = Arc::new(FakeWebhookStore::new(flow_store.clone()));
    let token_store = Arc::new(FakeTriggerTokenStore::new());
    let inbound = Arc::new(InboundService::new(
        token_store.clone(),
        send_store.clone(),
        Arc::new(FakeTemplatesStore::new()),
        channel_providers.clone(),
        Arc::new(StubFactory::succeeding()),
        http.clone(),
    ));
    let webhooks = Arc::new(WebhookService::new(
        webhook_store.clone(),
        flow_store.clone(),
        inbound.clone(),
    ));
    let template_store = Arc::new(FakeTemplatesStore::new());
    let templates = Arc::new(TemplateService::new(template_store.clone(), audit.clone()));
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
            schedules: Some(schedules),
            webhooks: Some(webhooks),
            campaigns: None,
            notifications: None,
            inbound: Some(inbound),
            provider_tester: Arc::new(ConfigProviderTester::new())
                as Arc<dyn ProviderTester + Send + Sync>,
        },
        &AppConfig::default(),
    );
    (
        app,
        Fakes {
            flows: flow_store,
            schedules: schedule_store,
            webhooks: webhook_store,
            sends: send_store,
            channels: channel_providers,
            templates: template_store,
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

async fn setup_project(app: Router, fakes: &Fakes, email: &str, name: &str) -> (String, String) {
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

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/projects",
        Some(json!({"name": name})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let project = body_json(res).await["project"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    fakes.flows.seed_visible(&user_id, &project);
    fakes.schedules.seed_visible(&user_id, &project);
    fakes.webhooks.seed_visible(&user_id, &project);
    fakes.sends.seed_visible(&user_id, &project);
    fakes.templates.seed_visible(&user_id, &project);
    fakes.channels.seed_visible(&user_id, &project);
    fakes.channels.seed_config(&project, "email", "smtp");
    (token, project)
}

fn create_webhook_body(project: &str) -> Value {
    json!({
        "projectId": project,
        "name": "Order intake",
        "description": "Fires the order flow",
        "events": ["notification.created", "order.placed"],
    })
}

async fn create_webhook(app: Router, token: &str, body: Value) -> Value {
    let res = request_with_cookie(app.clone(), "POST", "/app/webhooks", Some(body), token).await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["webhook"].clone()
}

/// Reads the owned flow by id (the builder's load path).
async fn get_flow(app: Router, token: &str, project: &str, flow_id: &str) -> (StatusCode, Value) {
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows/{flow_id}?project_id={project}"),
        None,
        token,
    )
    .await;
    let status = res.status();
    let body = body_json(res).await;
    (status, body["flow"].clone())
}

fn valid_trigger_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

#[tokio::test]
async fn webhook_create_builds_owned_flow_with_locked_trigger() {
    let (app, _fakes) = app_with_webhooks();
    let (token, project) = setup_project(app.clone(), &_fakes, "a@x.com", "P").await;

    let webhook = create_webhook(app.clone(), &token, create_webhook_body(&project)).await;
    assert_eq!(webhook["name"], "Order intake");
    assert_eq!(webhook["description"], "Fires the order flow");
    assert_eq!(webhook["active"], true);
    assert!(webhook["archivedAt"].is_null());
    assert_eq!(
        webhook["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["notification.created", "order.placed"]
    );
    let flow = webhook["flowId"].as_str().unwrap().to_string();
    assert_eq!(webhook["flowName"], "Order intake");

    // The owned flow: named after the webhook, one webhook trigger with a
    // fresh token + events, tagged `webhook` as its source.
    let (status, flow_doc) = get_flow(app.clone(), &token, &project, &flow).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(flow_doc["name"], "Order intake");
    assert_eq!(flow_doc["source"], "webhook");
    let nodes = flow_doc["definition"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["type"], "trigger");
    assert_eq!(nodes[0]["data"]["kind"], "webhook");
    assert!(valid_trigger_token(
        nodes[0]["data"]["token"].as_str().unwrap()
    ));
    assert_eq!(nodes[0]["data"]["events"].as_array().unwrap().len(), 2);

    // Owner-managed flows stay off the flows list.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(body_json(res).await["flows"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn webhook_rename_syncs_and_events_rewrite_keeps_token() {
    let (app, _fakes) = app_with_webhooks();
    let (token, project) = setup_project(app.clone(), &_fakes, "b@x.com", "P").await;
    let webhook = create_webhook(app.clone(), &token, create_webhook_body(&project)).await;
    let id = webhook["id"].as_str().unwrap().to_string();
    let flow = webhook["flowId"].as_str().unwrap().to_string();
    let original_token = {
        let (_, flow_doc) = get_flow(app.clone(), &token, &project, &flow).await;
        flow_doc["definition"]["nodes"][0]["data"]["token"]
            .as_str()
            .unwrap()
            .to_string()
    };

    // Rename + shrink events.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/webhooks/{id}"),
        Some(json!({
            "projectId": project,
            "name": "Checkout intake",
            "events": ["order.placed"],
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let patched = body_json(res).await["webhook"].clone();
    assert_eq!(patched["name"], "Checkout intake");
    assert_eq!(patched["flowName"], "Checkout intake");
    assert_eq!(patched["events"].as_array().unwrap().len(), 1);

    // The trigger node was rewritten in place — same node, same token.
    let (_, flow_doc) = get_flow(app.clone(), &token, &project, &flow).await;
    assert_eq!(flow_doc["name"], "Checkout intake");
    assert_eq!(
        flow_doc["definition"]["nodes"][0]["data"]["token"],
        original_token
    );
    assert_eq!(
        flow_doc["definition"]["nodes"][0]["data"]["events"][0],
        "order.placed"
    );
}

#[tokio::test]
async fn webhook_fire_through_owned_flow_records_send() {
    let (app, _fakes) = app_with_webhooks();
    let (token, project) = setup_project(app.clone(), &_fakes, "c@x.com", "P").await;
    let webhook = create_webhook(app.clone(), &token, create_webhook_body(&project)).await;
    let flow = webhook["flowId"].as_str().unwrap().to_string();

    // Wire a provider route onto the owned flow (the trigger survives).
    let (_, flow_doc) = get_flow(app.clone(), &token, &project, &flow).await;
    let mut definition = flow_doc["definition"].clone();
    definition["nodes"].as_array_mut().unwrap().push(json!(
        {"id": "p1", "type": "provider", "position": {"x": 0, "y": 0},
         "data": {"channelId": "email", "providerId": "smtp", "priority": 1}}
    ));
    let res = request_with_cookie(
        app.clone(),
        "PUT",
        &format!("/app/flows/{flow}"),
        Some(json!({
            "projectId": project,
            "name": "Order intake",
            "definition": definition,
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // The webhook-born flow fires on its trigger token — no extra sync.
    let token_in_node = definition["nodes"][0]["data"]["token"]
        .as_str()
        .unwrap()
        .to_string();
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/v1/hooks/{token_in_node}"),
        Some(json!({
            "channel": "email", "recipients": ["user-1"],
            "title": "Order shipped", "body": "On its way",
        })),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let n = body_json(res).await["notification"].clone();
    assert_eq!(n["status"], "completed");
    assert_eq!(n["title"], "Order shipped");
}

#[tokio::test]
async fn archive_parks_webhook_and_restores() {
    let (app, _fakes) = app_with_webhooks();
    let (token, project) = setup_project(app.clone(), &_fakes, "d@x.com", "P").await;
    let webhook = create_webhook(app.clone(), &token, create_webhook_body(&project)).await;
    let id = webhook["id"].as_str().unwrap().to_string();
    let flow = webhook["flowId"].as_str().unwrap().to_string();

    // Archive — the flow stays bound and reachable.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/webhooks/{id}"),
        Some(json!({"projectId": project, "archived": true})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let archived = body_json(res).await["webhook"].clone();
    assert!(archived["archivedAt"].is_string());
    let (_, flow_doc) = get_flow(app.clone(), &token, &project, &flow).await;
    assert_eq!(flow_doc["source"], "webhook");

    // Restore clears it.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/webhooks/{id}"),
        Some(json!({"projectId": project, "archived": false})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["webhook"]["archivedAt"].is_null());
}

#[tokio::test]
async fn webhook_delete_takes_its_flow() {
    let (app, _fakes) = app_with_webhooks();
    let (token, project) = setup_project(app.clone(), &_fakes, "e@x.com", "P").await;
    let webhook = create_webhook(app.clone(), &token, create_webhook_body(&project)).await;
    let id = webhook["id"].as_str().unwrap().to_string();
    let flow = webhook["flowId"].as_str().unwrap().to_string();

    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/webhooks/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["deleted"], true);

    // The flow is gone too — nowhere to resurrect it from.
    let (status, _) = get_flow(app.clone(), &token, &project, &flow).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/webhooks?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(
        body_json(res).await["webhooks"].as_array().unwrap().len(),
        0
    );
}

#[tokio::test]
async fn webhooks_are_project_scoped() {
    let (app, _fakes) = app_with_webhooks();
    let (token_a, project_a) = setup_project(app.clone(), &_fakes, "f@x.com", "PA").await;
    let (token_b, project_b) = setup_project(app.clone(), &_fakes, "g@x.com", "PB").await;
    let webhook = create_webhook(app.clone(), &token_a, create_webhook_body(&project_a)).await;
    let id = webhook["id"].as_str().unwrap().to_string();
    let flow = webhook["flowId"].as_str().unwrap().to_string();

    // B lists nothing and reads 404.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/webhooks?project_id={project_b}"),
        None,
        &token_b,
    )
    .await;
    assert_eq!(
        body_json(res).await["webhooks"].as_array().unwrap().len(),
        0
    );
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/webhooks/{id}?project_id={project_b}"),
        None,
        &token_b,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // B cannot read A's flow either.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows/{flow}?project_id={project_b}"),
        None,
        &token_b,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn webhook_validation_edges() {
    let (app, _fakes) = app_with_webhooks();
    let (token, project) = setup_project(app.clone(), &_fakes, "h@x.com", "P").await;

    // Empty name.
    let mut bad = create_webhook_body(&project);
    bad["name"] = json!("   ");
    let res = request_with_cookie(app.clone(), "POST", "/app/webhooks", Some(bad), &token).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Events with empties/dupes normalize rather than fail.
    let created = create_webhook(
        app.clone(),
        &token,
        json!({
            "projectId": project,
            "name": "Noisy events",
            "events": ["a.b", "", " a.b ", "c.d"],
        }),
    )
    .await;
    assert_eq!(
        created["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["a.b", "c.d"]
    );
}
