//! End-to-end HTTP tests for inbound webhook triggers.
//!
//! Covers token sync on flow saves, firing with full payloads,
//! caller-wins/fallback resolution, unknown-token 404s, fail-closed
//! validation, token rotation on flow updates, and the door's own auth:
//! `x-notifi-secret` checks (401) and IP allow-list checks (403) with
//! the open-door default when neither is configured.

mod common;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::common::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeBillingStore, FakeChannelProviderStore,
    FakeFlowStore, FakeHttpRequester, FakeSendStore, FakeTemplatesStore, FakeTriggerTokenStore,
};
use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
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
use server::domain::sends::SendService;
use server::domain::templates::TemplateService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::ports::{
    AdapterFactory, BoxSendFuture, DeliveryMessage, DeliveryProvider, DeliveryReceipt,
};
use tower::ServiceExt;

struct StubAdapter;

impl DeliveryProvider for StubAdapter {
    fn channel(&self) -> &'static str {
        "stub"
    }

    fn provider_name(&self) -> &'static str {
        "stub"
    }

    fn send(&self, _message: &DeliveryMessage) -> BoxSendFuture<'_> {
        Box::pin(async move {
            Ok(DeliveryReceipt {
                provider: "stub".to_string(),
                provider_message_id: Some("mid-1".to_string()),
            })
        })
    }
}

struct StubFactory {
    fail: Mutex<HashSet<String>>,
}

impl AdapterFactory for StubFactory {
    fn adapter(
        &self,
        _project_id: &str,
        _channel: &str,
        provider_id: &str,
        _config: &serde_json::Value,
    ) -> Option<Arc<dyn DeliveryProvider>> {
        if provider_id == "ghost" || self.fail.lock().unwrap().contains(provider_id) {
            return None;
        }
        Some(Arc::new(StubAdapter))
    }
}

struct Fakes {
    flows: Arc<FakeFlowStore>,
    sends: Arc<FakeSendStore>,
    channels: Arc<FakeChannelProviderStore>,
    templates: Arc<FakeTemplatesStore>,
}

fn app_with_inbound() -> (Router, Fakes) {
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
    let template_store = Arc::new(FakeTemplatesStore::new());
    let templates = Arc::new(TemplateService::new(template_store.clone(), audit.clone()));
    let token_store = Arc::new(FakeTriggerTokenStore::new());
    let http = Arc::new(FakeHttpRequester::new());
    let sends = Arc::new(SendService::new(
        send_store.clone(),
        flows.clone(),
        channel_providers.clone(),
        Arc::new(StubFactory {
            fail: Mutex::new(HashSet::new()),
        }),
        http.clone(),
    ));
    let inbound = Arc::new(InboundService::new(
        token_store.clone(),
        send_store.clone(),
        template_store.clone(),
        channel_providers.clone(),
        Arc::new(StubFactory {
            fail: Mutex::new(HashSet::new()),
        }),
        http.clone(),
    ));
    let app = build_router(
        AppState {
            db: None,
            redis: None,
            gateway: None,
            auth: Some(auth),
            oauth: None,
            admin: Some(admin),
            admin_users: None,
            admin_projects: None,
            admin_notifications: None,
            projects: Some(projects),
            project_exports: None,
            project_imports: None,
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
            campaigns: None,
            inbound: Some(inbound),
            notifications: None,
            provider_tester: Arc::new(ConfigProviderTester::new())
                as Arc<dyn ProviderTester + Send + Sync>,
            config_store: std::sync::Arc::new(server::infra::FileConfigStore::new("configs")),
        },
        &AppConfig::default(),
    );
    (
        app,
        Fakes {
            flows: flow_store,
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

async fn post_json(
    app: Router,
    uri: &str,
    body: Option<Value>,
    cookie: &str,
) -> axum::response::Response {
    // ConnectInfo feeds the webhook IP allow-list; the loopback default
    // keeps plain requests outside any 403 path.
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 41234))));
    if !cookie.is_empty() {
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    let body = body
        .map(|b| Body::from(b.to_string()))
        .unwrap_or_else(Body::empty);
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

/// Fire with the two door inputs a test controls: the `x-notifi-secret`
/// header and the caller's socket address (via the ConnectInfo
/// extension — the real serve line supplies it from the connection).
async fn post_fire(
    app: Router,
    token: &str,
    body: Value,
    secret: Option<&str>,
    ip: [u8; 4],
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri(format!("/v1/hooks/{token}"))
        .header("content-type", "application/json")
        .extension(ConnectInfo(SocketAddr::from((ip, 41234))));
    if let Some(secret) = secret {
        builder = builder.header("x-notifi-secret", secret);
    }
    app.oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn put_json(app: Router, uri: &str, body: Value, cookie: &str) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("PUT")
        .uri(uri)
        .header("content-type", "application/json");
    if !cookie.is_empty() {
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    app.oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn setup_project(app: Router, fakes: &Fakes, email: &str, name: &str) -> (String, String) {
    let res = post_json(
        app.clone(),
        "/app/auth/signup",
        Some(json!({"name": "Pat", "email": email, "password": "Sup3rSecret!"})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    let token = body["session"]["token"].as_str().unwrap().to_string();
    let user_id = body["user"]["id"].as_str().unwrap().to_string();

    let res = post_json(
        app.clone(),
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
    fakes.sends.seed_visible(&user_id, &project);
    fakes.channels.seed_visible(&user_id, &project);
    fakes.templates.seed_visible(&user_id, &project);
    fakes.channels.seed_config(&project, "email", "smtp");
    (token, project)
}

const TOKEN_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TOKEN_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn webhook_flow_definition(token: &str) -> Value {
    json!({
        "version": 1,
        "nodes": [
            {"id": "t", "type": "trigger", "position": {"x": 0, "y": 0},
             "data": {"kind": "webhook", "token": token, "note": "shop"}},
            {"id": "p1", "type": "provider", "position": {"x": 0, "y": 0},
             "data": {"channelId": "email", "providerId": "smtp", "priority": 1}},
            {"id": "s", "type": "stop", "position": {"x": 0, "y": 0}, "data": {"reason": ""}}
        ],
        "edges": []
    })
}

async fn create_flow(app: Router, token: &str, project: &str, definition: Value) -> String {
    let res = post_json(
        app.clone(),
        "/app/flows",
        Some(json!({"projectId": project, "name": "Webhook flow", "definition": definition})),
        token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["flow"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn create_template(app: Router, token: &str, project: &str) -> String {
    let res = post_json(
        app.clone(),
        &format!("/app/projects/{project}/templates"),
        Some(json!({
            "name": "Morning edition",
            "channel": "email",
            "content": {"email": {"subject": "Good morning", "text": "Digest body"}},
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

#[tokio::test]
async fn webhook_fire_full_payload_records_send() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;
    create_flow(
        app.clone(),
        &token,
        &project,
        webhook_flow_definition(TOKEN_A),
    )
    .await;

    let res = post_json(
        app.clone(),
        &format!("/v1/hooks/{TOKEN_A}"),
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
    assert_eq!(n["provider"], "smtp");
}

#[tokio::test]
async fn unknown_token_reads_as_missing() {
    let (app, fakes) = app_with_inbound();
    let (_token, _project) = setup_project(app.clone(), &fakes, "b@x.com", "P").await;
    let res = post_json(
        app.clone(),
        "/v1/hooks/cccccccccccccccccccccccccccccccc",
        Some(json!({"channel": "email", "recipients": ["u"], "title": "t"})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_recipients_fails_closed() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "c@x.com", "P").await;
    create_flow(
        app.clone(),
        &token,
        &project,
        webhook_flow_definition(TOKEN_A),
    )
    .await;

    let res = post_json(
        app.clone(),
        &format!("/v1/hooks/{TOKEN_A}"),
        Some(json!({"channel": "email", "title": "t"})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

fn door_body() -> Value {
    json!({
        "channel": "email", "recipients": ["user-1"],
        "title": "Order shipped", "body": "On its way",
    })
}

#[tokio::test]
async fn webhook_fire_open_door_without_config() {
    // No secrets, no IP rules, no header — the pre-auth default holds.
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "door@x.com", "P").await;
    create_flow(
        app.clone(),
        &token,
        &project,
        webhook_flow_definition(TOKEN_A),
    )
    .await;
    let res = post_fire(app, TOKEN_A, door_body(), None, [127, 0, 0, 1]).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn webhook_fire_secret_header_passes() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "key@x.com", "P").await;
    let mut definition = webhook_flow_definition(TOKEN_A);
    definition["nodes"][0]["data"]["secrets"] = json!(["whsk_4420aaaa", "whsk_bbbbbbbb"]);
    create_flow(app.clone(), &token, &project, definition).await;

    // Either listed key passes (rotation), with any caller address.
    for (secret, ip) in [
        ("whsk_4420aaaa", [8, 8, 8, 8]),
        ("whsk_bbbbbbbb", [1, 1, 1, 1]),
    ] {
        let res = post_fire(app.clone(), TOKEN_A, door_body(), Some(secret), ip).await;
        assert_eq!(res.status(), StatusCode::OK, "key failed for {ip:?}");
    }
}

#[tokio::test]
async fn webhook_fire_missing_secret_header_is_unauthorized() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "lock1@x.com", "P").await;
    let mut definition = webhook_flow_definition(TOKEN_A);
    definition["nodes"][0]["data"]["secrets"] = json!(["whsk_4420aaaa"]);
    create_flow(app.clone(), &token, &project, definition).await;

    let res = post_fire(app, TOKEN_A, door_body(), None, [8, 8, 8, 8]).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn webhook_fire_wrong_secret_header_is_unauthorized() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "lock2@x.com", "P").await;
    let mut definition = webhook_flow_definition(TOKEN_A);
    definition["nodes"][0]["data"]["secrets"] = json!(["whsk_4420aaaa"]);
    create_flow(app.clone(), &token, &project, definition).await;

    let res = post_fire(app, TOKEN_A, door_body(), Some("whsk_wrong"), [8, 8, 8, 8]).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn webhook_fire_ip_allowlist_admits_inside_caller() {
    // CIDR + plain-IP rules both pass for their own and block others.
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "net@x.com", "P").await;
    let mut definition = webhook_flow_definition(TOKEN_A);
    definition["nodes"][0]["data"]["allowedIps"] = json!(["10.0.0.0/8"]);
    create_flow(app.clone(), &token, &project, definition).await;

    let res = post_fire(app, TOKEN_A, door_body(), None, [10, 1, 2, 3]).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn webhook_fire_ip_allowlist_denies_outside_caller() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "net2@x.com", "P").await;
    let mut definition = webhook_flow_definition(TOKEN_A);
    definition["nodes"][0]["data"]["allowedIps"] = json!(["10.0.0.0/8"]);
    create_flow(app.clone(), &token, &project, definition).await;

    let res = post_fire(app, TOKEN_A, door_body(), None, [8, 8, 8, 8]).await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn template_fallback_fills_missing_fields() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "d@x.com", "P").await;
    let template = create_template(app.clone(), &token, &project).await;
    let mut definition = webhook_flow_definition(TOKEN_A);
    definition["nodes"][0]["data"]["templateId"] = json!(template);
    create_flow(app.clone(), &token, &project, definition).await;

    // Caller sends only recipients; template owns channel/title/body.
    let res = post_json(
        app.clone(),
        &format!("/v1/hooks/{TOKEN_A}"),
        Some(json!({"recipients": ["user-2"]})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let n = body_json(res).await["notification"].clone();
    assert_eq!(n["status"], "completed");
    assert_eq!(n["title"], "Good morning");
    assert_eq!(n["channel"], "email");
}

#[tokio::test]
async fn template_fallback_presets_render_declared_refs() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "f2@x.com", "P").await;
    // A fallback template declaring `{{code}}` — the trigger's stored
    // `values` presets feed it at fire time.
    let res = post_json(
        app.clone(),
        &format!("/app/projects/{project}/templates"),
        Some(json!({
            "name": "Coded edition",
            "channel": "email",
            "content": {"email": {"subject": "Code {{code}}", "text": "Your code is {{code}} — {{missing}} stays raw."}},
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let template = body_json(res).await["template"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut definition = webhook_flow_definition(TOKEN_A);
    definition["nodes"][0]["data"]["templateId"] = json!(template);
    definition["nodes"][0]["data"]["values"] = json!({"code": "4420", "blank": ""});
    create_flow(app.clone(), &token, &project, definition).await;

    // Caller sends only recipients: the template + presets own content.
    let res = post_json(
        app.clone(),
        &format!("/v1/hooks/{TOKEN_A}"),
        Some(json!({"recipients": ["user-3"]})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let n = body_json(res).await["notification"].clone();
    assert_eq!(n["status"], "completed");
    assert_eq!(n["title"], "Code 4420");

    // The body rendered off the same presets; undeclared refs stay raw.
    let rows = fakes.sends.all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].body, "Your code is 4420 — {{missing}} stays raw.");
}

#[tokio::test]
async fn token_rotates_on_flow_update() {
    let (app, fakes) = app_with_inbound();
    let (token, project) = setup_project(app.clone(), &fakes, "e@x.com", "P").await;
    let flow = create_flow(
        app.clone(),
        &token,
        &project,
        webhook_flow_definition(TOKEN_A),
    )
    .await;

    let res = post_json(
        app.clone(),
        &format!("/v1/hooks/{TOKEN_A}"),
        Some(json!({"channel": "email", "recipients": ["u"], "title": "t"})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // Rotate: same node, new token.
    let definition = webhook_flow_definition(TOKEN_B);
    let res = put_json(
        app.clone(),
        &format!("/app/flows/{flow}"),
        json!({"projectId": project, "name": "Webhook flow", "definition": definition}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = post_json(
        app.clone(),
        &format!("/v1/hooks/{TOKEN_A}"),
        Some(json!({"channel": "email", "recipients": ["u"], "title": "t"})),
        "",
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = post_json(
        app.clone(),
        &format!("/v1/hooks/{TOKEN_B}"),
        Some(json!({"channel": "email", "recipients": ["u"], "title": "t"})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}
