//! End-to-end HTTP tests for record-first product sends.
//!
//! Covers POST /v1/notifications (instant execution through the default
//! flow with priority fallback), the 409/422/400/404 edges, the default
//! flow CRUD surface, and GET /app/sends history. Adapter behavior is
//! scripted via a stub factory — no real providers are touched.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

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
use server::domain::projects::ProjectService;
use server::domain::sends::SendService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::ports::{
    AdapterFactory, BoxSendFuture, DeliveryError, DeliveryMessage, DeliveryProvider,
    DeliveryReceipt,
};
use server::testing::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeBillingStore, FakeChannelProviderStore,
    FakeFlowStore, FakeHttpRequester, FakeSendStore,
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

/// Scripted adapters: listed provider ids fail, `ghost` has no adapter,
struct StubFactory {
    fail: Mutex<HashSet<String>>,
}

impl StubFactory {
    fn failing(ids: &[&str]) -> Self {
        Self {
            fail: Mutex::new(ids.iter().map(|s| s.to_string()).collect()),
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

fn app_with_sends(fail: &[&str]) -> (Router, Fakes) {
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
        Arc::new(StubFactory::failing(fail)),
        http.clone(),
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
            templates: None,
            channel_providers: Some(channel_providers.clone()),
            tickets: None,
            billing: Some(billing),
            flows: Some(flows),
            sends: Some(sends),
            notifications: None,
            schedules: None,
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
            sends: send_store,
            channels: channel_providers,
            http,
        },
    )
}

struct Fakes {
    flows: Arc<FakeFlowStore>,
    sends: Arc<FakeSendStore>,
    channels: Arc<FakeChannelProviderStore>,
    http: Arc<FakeHttpRequester>,
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

fn two_provider_definition() -> Value {
    json!({
        "version": 1,
        "nodes": [
            {"id": "t", "type": "trigger", "position": {"x": 0, "y": 0}, "data": {"event": "notification.created"}},
            {"id": "p1", "type": "provider", "position": {"x": 0, "y": 0}, "data": {"channelId": "email", "providerId": "smtp", "priority": 1}},
            {"id": "p2", "type": "provider", "position": {"x": 0, "y": 0}, "data": {"channelId": "email", "providerId": "sendgrid", "priority": 2}},
            {"id": "s", "type": "stop", "position": {"x": 0, "y": 0}, "data": {"reason": ""}}
        ],
        "edges": []
    })
}

fn single_provider_definition(provider: &str) -> Value {
    json!({
        "version": 1,
        "nodes": [
            {"id": "t", "type": "trigger", "position": {"x": 0, "y": 0}, "data": {"event": "notification.created"}},
            {"id": "p1", "type": "provider", "position": {"x": 0, "y": 0}, "data": {"channelId": "email", "providerId": provider, "priority": 1}},
            {"id": "s", "type": "stop", "position": {"x": 0, "y": 0}, "data": {"reason": ""}}
        ],
        "edges": []
    })
}

async fn setup_project(app: Router, fakes: &Fakes, email: &str, name: &str) -> (String, String) {
    let (token, user_id) = signup(app.clone(), email).await;
    let project = create_project(app.clone(), &token, name).await;
    fakes.flows.seed_visible(&user_id, &project);
    fakes.sends.seed_visible(&user_id, &project);
    fakes.channels.seed_visible(&user_id, &project);
    fakes.channels.seed_config(&project, "email", "smtp");
    fakes.channels.seed_config(&project, "email", "sendgrid");
    (token, project)
}

async fn put_flow(app: Router, token: &str, project: &str) -> String {
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/flows",
        Some(json!({"projectId": project, "name": "Email chain", "definition": two_provider_definition()})),
        "session_token",
        token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["flow"]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn send_body(project: &str) -> Value {
    json!({
        "projectId": project,
        "channel": "email",
        "recipients": ["user@example.com"],
        "title": "Hello",
        "body": "World",
    })
}

#[tokio::test]
async fn instant_send_falls_back_by_priority() {
    let (app, fakes) = app_with_sends(&["smtp"]);
    let (token, project) = setup_project(app.clone(), &fakes, "a@x.dev", "Shop").await;
    put_flow(app.clone(), &token, &project).await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(send_body(&project)),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let n = &body_json(res).await["notification"];
    assert_eq!(n["status"], "completed");
    assert_eq!(n["provider"], "sendgrid");
    assert_eq!(n["channel"], "email");
}

#[tokio::test]
async fn exhausted_chain_marks_failed() {
    let (app, fakes) = app_with_sends(&["smtp", "sendgrid"]);
    let (token, project) = setup_project(app.clone(), &fakes, "b@x.dev", "Shop").await;
    put_flow(app.clone(), &token, &project).await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(send_body(&project)),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let n = &body_json(res).await["notification"];
    assert_eq!(n["status"], "failed");
    assert!(n["lastError"].as_str().unwrap().contains("boom"));
}

#[tokio::test]
async fn missing_default_flow_rejects_with_conflict() {
    let (app, fakes) = app_with_sends(&[]);
    let (token, project) = setup_project(app.clone(), &fakes, "c@x.dev", "Shop").await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(send_body(&project)),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn future_send_at_is_refused() {
    let (app, fakes) = app_with_sends(&[]);
    let (token, project) = setup_project(app.clone(), &fakes, "d@x.dev", "Shop").await;
    put_flow(app.clone(), &token, &project).await;

    let mut body = send_body(&project);
    body["sendAt"] = json!("2999-01-01T00:00:00Z");
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(body),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn send_validates_input_and_isolation() {
    let (app, fakes) = app_with_sends(&[]);
    let (token, project) = setup_project(app.clone(), &fakes, "e@x.dev", "Shop").await;
    put_flow(app.clone(), &token, &project).await;
    let (bob_token, _) = signup(app.clone(), "bob@x.dev").await;

    // Empty recipients.
    let mut body = send_body(&project);
    body["recipients"] = json!([]);
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(body),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Another user's project is invisible.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(send_body(&project)),
        "session_token",
        &bob_token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn flows_crud_round_trip() {
    let (app, fakes) = app_with_sends(&[]);
    let (token, project) = setup_project(app.clone(), &fakes, "f@x.dev", "Shop").await;

    // Empty at first.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows?project_id={project}"),
        None,
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["flows"].as_array().unwrap().len(), 0);

    // Create without a definition starts empty (builder asks for a trigger).
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/flows",
        Some(json!({"projectId": project, "name": "Mine"})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let created = body_json(res).await["flow"].clone();
    assert_eq!(created["name"], "Mine");
    let id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["definition"]["nodes"].as_array().unwrap().len(), 0);

    // Update renames.
    let res = request_with_cookie(
        app.clone(),
        "PUT",
        &format!("/app/flows/{id}"),
        Some(
            json!({"projectId": project, "name": "Renamed", "definition": two_provider_definition()}),
        ),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["flow"]["name"], "Renamed");

    // Delete removes it (sends then 409).
    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/flows/{id}?project_id={project}"),
        None,
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["deleted"], true);
}

#[tokio::test]
async fn multiple_matching_flows_all_run() {
    // First flow fails throughout, second delivers: the send still
    // completes via the second flow (run-all-matching aggregation).
    let (app, fakes) = app_with_sends(&["smtp"]);
    let (token, project) = setup_project(app.clone(), &fakes, "h@x.dev", "Shop").await;

    for name in ["First", "Second"] {
        let provider = if name == "First" { "smtp" } else { "sendgrid" };
        let res = request_with_cookie(
            app.clone(),
            "POST",
            "/app/flows",
            Some(json!({"projectId": project, "name": name, "definition": single_provider_definition(provider)})),
            "session_token",
            &token,
        )
        .await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(send_body(&project)),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let n = &body_json(res).await["notification"];
    assert_eq!(n["status"], "completed");
    assert_eq!(n["provider"], "sendgrid");
}

#[tokio::test]
async fn sends_list_is_newest_first() {
    let (app, fakes) = app_with_sends(&[]);
    let (token, project) = setup_project(app.clone(), &fakes, "g@x.dev", "Shop").await;
    put_flow(app.clone(), &token, &project).await;

    for _ in 0..2 {
        let res = request_with_cookie(
            app.clone(),
            "POST",
            "/v1/notifications",
            Some(send_body(&project)),
            "session_token",
            &token,
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/sends?project_id={project}"),
        None,
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sends = body_json(res).await["sends"].clone();
    let sends = sends.as_array().unwrap();
    assert_eq!(sends.len(), 2);
    assert!(sends[0]["createdAt"].as_str().unwrap() >= sends[1]["createdAt"].as_str().unwrap());
}

fn http_step_definition() -> Value {
    json!({
        "version": 1,
        "nodes": [
            {"id": "t", "type": "trigger", "position": {"x": 0, "y": 0}, "data": {"event": "notification.created"}},
            {"id": "p1", "type": "provider", "position": {"x": 0, "y": 0}, "data": {"channelId": "email", "providerId": "smtp", "priority": 1}},
            {"id": "h1", "type": "http-request", "position": {"x": 0, "y": 0},
             "data": {"url": "https://hooks.example.com/ping", "method": "POST",
                      "headers": [{"key": "X-A", "value": "1"}], "body": "{}"}},
            {"id": "s", "type": "stop", "position": {"x": 0, "y": 0}, "data": {"reason": ""}}
        ],
        "edges": []
    })
}

#[tokio::test]
async fn http_request_steps_fire_without_affecting_delivery() {
    let (app, fakes) = app_with_sends(&[]);
    let (token, project) = setup_project(app.clone(), &fakes, "z@x.dev", "Shop").await;
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/flows",
        Some(json!({"projectId": project, "name": "Ping chain", "definition": http_step_definition()})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(send_body(&project)),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["notification"]["status"], "completed");

    let calls = fakes.http.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].url, "https://hooks.example.com/ping");
    assert_eq!(calls[0].method, "POST");
    assert_eq!(calls[0].headers, vec![("X-A".to_string(), "1".to_string())]);
}

#[tokio::test]
async fn failing_http_steps_do_not_fail_the_send() {
    let (app, fakes) = app_with_sends(&[]);
    fakes.http.set_fail(true);
    let (token, project) = setup_project(app.clone(), &fakes, "y@x.dev", "Shop").await;
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/flows",
        Some(json!({"projectId": project, "name": "Ping chain", "definition": http_step_definition()})),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/v1/notifications",
        Some(send_body(&project)),
        "session_token",
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["notification"]["status"], "completed");
    assert_eq!(fakes.http.calls().len(), 1);
}
