//! End-to-end HTTP tests for flow schedules.
//!
//! Covers schedule CRUD (each create builds its own flow — the owner
//! doctrine), validation edges, owned flows staying off the `/flows`
//! list and dying with the schedule, flow-by-id reads tagging `source`,
//! archive/restore, manual fire through the owned flow, cross-project
//! isolation, and reactivation re-anchoring. Adapter behavior is
//! scripted via a stub factory.

mod common;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::common::{
    FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeBillingStore, FakeChannelProviderStore,
    FakeFlowStore, FakeHttpRequester, FakeScheduleStore, FakeSendStore, FakeTemplatesStore,
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
use server::domain::projects::ProjectService;
use server::domain::schedules::ScheduleService;
use server::domain::sends::SendService;
use server::domain::templates::TemplateService;
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
    sends: Arc<FakeSendStore>,
    channels: Arc<FakeChannelProviderStore>,
    templates: Arc<FakeTemplatesStore>,
    http: Arc<FakeHttpRequester>,
}

fn app_with_schedules() -> (Router, Fakes) {
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
    let template_store = Arc::new(FakeTemplatesStore::new());
    let templates = Arc::new(TemplateService::new(template_store.clone(), audit.clone()));
    let schedules = Arc::new(ScheduleService::new(
        schedule_store.clone(),
        flow_store.clone(),
        send_store.clone(),
        template_store.clone(),
        channel_providers.clone(),
        Arc::new(StubFactory::succeeding()),
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
            templates: Some(templates),
            channel_providers: Some(channel_providers.clone()),
            tickets: None,
            billing: Some(billing),
            flows: Some(flows),
            sends: Some(sends),
            schedules: Some(schedules),
            webhooks: None,
            campaigns: None,
            notifications: None,
            inbound: None,
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
            schedules: schedule_store,
            sends: send_store,
            channels: channel_providers,
            templates: template_store,
            http,
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

fn email_flow_definition(provider: &str) -> Value {
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
    fakes.schedules.seed_visible(&user_id, &project);
    fakes.sends.seed_visible(&user_id, &project);
    fakes.templates.seed_visible(&user_id, &project);
    fakes.channels.seed_visible(&user_id, &project);
    fakes.channels.seed_config(&project, "email", "smtp");
    (token, project)
}

/// Replaces the schedule's flow definition (the builder's save path).
/// The flow is created by the schedule; this wires actual steps onto it.
async fn put_flow_definition(
    app: Router,
    token: &str,
    project: &str,
    flow_id: &str,
    name: &str,
    definition: Value,
) {
    let res = request_with_cookie(
        app.clone(),
        "PUT",
        &format!("/app/flows/{flow_id}"),
        Some(json!({
            "projectId": project,
            "name": name,
            "definition": definition,
        })),
        token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

fn recurring_body(project: &str) -> Value {
    json!({
        "projectId": project,
        "name": "Morning digest",
        "kind": "recurring",
        "intervalSecs": 3600,
        "timezone": "UTC",
        "channel": "email",
        "title": "Good morning",
        "body": "Here is your digest.",
        "recipients": ["user-1"],
    })
}

async fn create_schedule(app: Router, token: &str, body: Value) -> Value {
    let res = request_with_cookie(app.clone(), "POST", "/app/schedules", Some(body), token).await;
    assert_eq!(res.status(), StatusCode::CREATED);
    body_json(res).await["schedule"].clone()
}

#[tokio::test]
async fn schedules_crud_round_trip() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "a@x.com", "P").await;

    let created = create_schedule(app.clone(), &token, recurring_body(&project)).await;
    assert_eq!(created["name"], "Morning digest");
    assert_eq!(created["kind"], "recurring");
    // The flow is born with the schedule: same name, trigger seeded.
    let flow = created["flowId"].as_str().unwrap().to_string();
    assert_eq!(created["flowName"], "Morning digest");
    assert_eq!(created["timing"], "Every 1 hour");
    assert_eq!(created["active"], true);
    assert!(created["archivedAt"].is_null());
    assert!(created["nextRunAt"].is_string());
    let id = created["id"].as_str().unwrap().to_string();

    // The seeded flow carries one locked schedule trigger.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows/{flow}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let flow_doc = body_json(res).await["flow"].clone();
    assert_eq!(flow_doc["name"], "Morning digest");
    assert_eq!(flow_doc["source"], "schedule");
    let nodes = flow_doc["definition"]["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["type"], "trigger");
    assert_eq!(nodes[0]["data"]["kind"], "schedule");

    // Get one.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/schedules/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // List contains it.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/schedules?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let list = body_json(res).await;
    assert_eq!(list["schedules"].as_array().unwrap().len(), 1);

    // Pause + rename — the rename syncs down to the flow.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/schedules/{id}"),
        Some(json!({"projectId": project, "name": "Evening digest", "active": false})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let patched = body_json(res).await["schedule"].clone();
    assert_eq!(patched["name"], "Evening digest");
    assert_eq!(patched["flowName"], "Evening digest");
    assert_eq!(patched["active"], false);

    // Delete.
    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/schedules/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["deleted"], true);

    // Gone afterwards.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/schedules/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn schedule_validation_edges() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "b@x.com", "P").await;

    // Unknown kind.
    let mut bad = recurring_body(&project);
    bad["kind"] = json!("cron");
    let res = request_with_cookie(app.clone(), "POST", "/app/schedules", Some(bad), &token).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Once without run_at.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/schedules",
        Some(json!({
            "projectId": project, "name": "x", "kind": "once",
            "channel": "email", "title": "t", "recipients": ["u"],
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Interval below the one-minute floor.
    let mut bad = recurring_body(&project);
    bad["intervalSecs"] = json!(30);
    let res = request_with_cookie(app.clone(), "POST", "/app/schedules", Some(bad), &token).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Empty recipients.
    let mut bad = recurring_body(&project);
    bad["recipients"] = json!([]);
    let res = request_with_cookie(app.clone(), "POST", "/app/schedules", Some(bad), &token).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Missing project_id on list.
    let res = request_with_cookie(app.clone(), "GET", "/app/schedules?other=1", None, &token).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn owned_flow_hidden_from_flows_list_and_deleted_with_schedule() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "c@x.com", "P").await;

    let created = create_schedule(app.clone(), &token, recurring_body(&project)).await;
    let id = created["id"].as_str().unwrap().to_string();

    // Hidden from the flows list (owner-managed).
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(body_json(res).await["flows"].as_array().unwrap().len(), 0);

    // Deleting the schedule deletes its flow too — it never reappears.
    let res = request_with_cookie(
        app.clone(),
        "DELETE",
        &format!("/app/schedules/{id}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(body_json(res).await["flows"].as_array().unwrap().len(), 0);
    let flow = created["flowId"].as_str().unwrap().to_string();
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows/{flow}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn run_now_executes_owned_flow() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "d@x.com", "P").await;
    let created = create_schedule(app.clone(), &token, recurring_body(&project)).await;
    let id = created["id"].as_str().unwrap().to_string();
    let flow = created["flowId"].as_str().unwrap().to_string();

    // Wire a real provider route onto the owned flow (builder save path).
    put_flow_definition(
        app.clone(),
        &token,
        &project,
        &flow,
        "Morning digest",
        email_flow_definition("smtp"),
    )
    .await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/schedules/{id}/run-now"),
        Some(json!({"projectId": project})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let ran = body_json(res).await["schedule"].clone();
    assert_eq!(ran["lastStatus"], "completed");
    assert_eq!(ran["runCount"], 1);
    assert!(ran["lastRunAt"].is_string());

    // The run is recorded as a send for the deliveries view.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/sends?project_id={project}"),
        None,
        &token,
    )
    .await;
    let sends = body_json(res).await["sends"].clone();
    assert_eq!(sends.as_array().unwrap().len(), 1);
    assert_eq!(sends[0]["status"], "completed");
    assert_eq!(sends[0]["title"], "Good morning");
}

#[tokio::test]
async fn run_now_without_provider_route_fails() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "e@x.com", "P").await;
    // A freshly born flow only has the trigger — no provider route.
    let created = create_schedule(app.clone(), &token, recurring_body(&project)).await;
    let id = created["id"].as_str().unwrap().to_string();

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/schedules/{id}/run-now"),
        Some(json!({"projectId": project})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let ran = body_json(res).await["schedule"].clone();
    assert_eq!(ran["lastStatus"], "failed");
    assert!(ran["lastError"].as_str().unwrap().contains("email"));
}

#[tokio::test]
async fn schedules_are_project_scoped() {
    let (app, fakes) = app_with_schedules();
    let (token_a, project_a) = setup_project(app.clone(), &fakes, "f@x.com", "PA").await;
    let (token_b, project_b) = setup_project(app.clone(), &fakes, "g@x.com", "PB").await;
    let created = create_schedule(app.clone(), &token_a, recurring_body(&project_a)).await;
    let id = created["id"].as_str().unwrap().to_string();
    let flow = created["flowId"].as_str().unwrap().to_string();

    // B lists nothing and reads 404.
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/schedules?project_id={project_b}"),
        None,
        &token_b,
    )
    .await;
    assert_eq!(
        body_json(res).await["schedules"].as_array().unwrap().len(),
        0
    );
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/schedules/{id}?project_id={project_b}"),
        None,
        &token_b,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    // B cannot read A's flow either (project-scoped by id).
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

async fn create_template(app: Router, fakes: &Fakes, token: &str, project: &str) -> String {
    let res = request_with_cookie(
        app.clone(),
        "POST",
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
    let template_id = body_json(res).await["template"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // Mirror the Postgres template JOIN in schedule reads.
    fakes.schedules.seed_template(
        &template_id,
        "Morning edition",
        json!({"email": {"subject": "Good morning", "text": "Digest body"}}),
    );
    template_id
}

#[tokio::test]
async fn weekly_schedule_round_trip() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "i@x.com", "P").await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/schedules",
        Some(json!({
            "projectId": project, "name": "Mon/Wed blast",
            "kind": "weekly", "days": [1, 3], "runTime": "09:00", "timezone": "UTC",
            "channel": "email", "title": "t", "recipients": ["u"],
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let created = body_json(res).await["schedule"].clone();
    assert_eq!(created["timing"], "Weekly Mon, Wed 09:00");
    assert!(created["nextRunAt"].is_string());

    // Weekly validation edges.
    for body in [
        json!({"projectId": project, "name": "x", "kind": "weekly",
               "days": [], "runTime": "09:00", "channel": "email", "title": "t", "recipients": ["u"]}),
        json!({"projectId": project, "name": "x", "kind": "weekly",
               "days": [1], "runTime": "25:00", "channel": "email", "title": "t", "recipients": ["u"]}),
        json!({"projectId": project, "name": "x", "kind": "weekly",
               "days": [1], "runTime": "09:00", "timezone": "Nope/Nowhere",
               "channel": "email", "title": "t", "recipients": ["u"]}),
    ] {
        let res =
            request_with_cookie(app.clone(), "POST", "/app/schedules", Some(body), &token).await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn template_schedule_runs_rendered_content() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "j@x.com", "P").await;
    let template = create_template(app.clone(), &fakes, &token, &project).await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/schedules",
        Some(json!({
            "projectId": project, "name": "templated",
            "kind": "recurring", "intervalSecs": 3600, "templateId": template,
            "recipients": ["user-9"],
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let created = body_json(res).await["schedule"].clone();
    assert_eq!(created["templateId"], template);
    assert_eq!(created["templateName"], "Morning edition");
    let id = created["id"].as_str().unwrap().to_string();
    let flow = created["flowId"].as_str().unwrap().to_string();

    // Wire an email provider route onto the owned flow.
    put_flow_definition(
        app.clone(),
        &token,
        &project,
        &flow,
        "templated",
        email_flow_definition("smtp"),
    )
    .await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/schedules/{id}/run-now"),
        Some(json!({"projectId": project})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let ran = body_json(res).await["schedule"].clone();
    assert_eq!(ran["lastStatus"], "completed");

    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/sends?project_id={project}"),
        None,
        &token,
    )
    .await;
    let sends = body_json(res).await["sends"].clone();
    assert_eq!(sends.as_array().unwrap().len(), 1);
    assert_eq!(sends[0]["title"], "Good morning");
    assert_eq!(sends[0]["channel"], "email");
}

#[tokio::test]
async fn template_schedule_without_recipients_fails_closed() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "k@x.com", "P").await;
    let template = create_template(app.clone(), &fakes, &token, &project).await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/schedules",
        Some(json!({
            "projectId": project, "name": "no-audience",
            "kind": "recurring", "intervalSecs": 3600, "templateId": template,
            "recipients": [],
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let id = body_json(res).await["schedule"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/schedules/{id}/run-now"),
        Some(json!({"projectId": project})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let ran = body_json(res).await["schedule"].clone();
    assert_eq!(ran["lastStatus"], "failed");
    assert!(ran["lastError"].as_str().unwrap().contains("recipients"));
}

#[tokio::test]
async fn unknown_template_rejected() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "l@x.com", "P").await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/schedules",
        Some(json!({
            "projectId": project, "name": "bad-tpl",
            "kind": "recurring", "intervalSecs": 3600, "templateId": "tpl_nope",
            "recipients": [],
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reactivation_reanchors_stale_next_run() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "h@x.com", "P").await;

    // One-shot in the past: next run is already stale.
    let res = request_with_cookie(
        app.clone(),
        "POST",
        "/app/schedules",
        Some(json!({
            "projectId": project, "name": "past", "kind": "once",
            "runAt": "2020-01-01T00:00:00Z",
            "channel": "email", "title": "t", "recipients": ["u"],
        })),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let id = body_json(res).await["schedule"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Switching a stale schedule to recurring re-anchors to now + interval.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/schedules/{id}"),
        Some(json!({"projectId": project, "kind": "recurring", "intervalSecs": 3600})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let next = body_json(res).await["schedule"]["nextRunAt"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(next.as_str() > "2026-01-01T00:00:00Z");
}

#[tokio::test]
async fn archive_parks_schedule_and_restores_runs() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "n@x.com", "P").await;
    let created = create_schedule(app.clone(), &token, recurring_body(&project)).await;
    let id = created["id"].as_str().unwrap().to_string();
    let flow = created["flowId"].as_str().unwrap().to_string();
    put_flow_definition(
        app.clone(),
        &token,
        &project,
        &flow,
        "Morning digest",
        email_flow_definition("smtp"),
    )
    .await;

    // Archive.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/schedules/{id}"),
        Some(json!({"projectId": project, "archived": true})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let archived = body_json(res).await["schedule"].clone();
    assert!(archived["archivedAt"].is_string());

    // Run-now is refused while archived (the flow stays bound).
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/schedules/{id}/run-now"),
        Some(json!({"projectId": project})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);

    // The flow is still reachable by id (source still tags schedule).
    let res = request_with_cookie(
        app.clone(),
        "GET",
        &format!("/app/flows/{flow}?project_id={project}"),
        None,
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["flow"]["source"], "schedule");

    // Restore — next run re-anchors, run-now works again.
    let res = request_with_cookie(
        app.clone(),
        "PATCH",
        &format!("/app/schedules/{id}"),
        Some(json!({"projectId": project, "archived": false})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await["schedule"]["archivedAt"].is_null());
    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/schedules/{id}/run-now"),
        Some(json!({"projectId": project})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["schedule"]["lastStatus"], "completed");
}

#[tokio::test]
async fn schedule_runs_fire_http_steps_without_affecting_outcome() {
    let (app, fakes) = app_with_schedules();
    let (token, project) = setup_project(app.clone(), &fakes, "m@x.com", "P").await;
    let created = create_schedule(app.clone(), &token, recurring_body(&project)).await;
    let id = created["id"].as_str().unwrap().to_string();
    let flow = created["flowId"].as_str().unwrap().to_string();

    // Wire a trigger + provider + HTTP step onto the owned flow.
    put_flow_definition(
        app.clone(),
        &token,
        &project,
        &flow,
        "Morning digest",
        json!({
            "version": 1,
            "nodes": [
                {"id": "t", "type": "trigger", "position": {"x": 0, "y": 0}, "data": {"kind": "schedule"}},
                {"id": "p1", "type": "provider", "position": {"x": 0, "y": 0}, "data": {"channelId": "email", "providerId": "smtp", "priority": 1}},
                {"id": "h1", "type": "http-request", "position": {"x": 0, "y": 0},
                 "data": {"url": "https://hooks.example.com/ping", "method": "POST", "headers": [], "body": "{}"}},
                {"id": "s", "type": "stop", "position": {"x": 0, "y": 0}, "data": {"reason": ""}}
            ],
            "edges": []
        }),
    )
    .await;

    let res = request_with_cookie(
        app.clone(),
        "POST",
        &format!("/app/schedules/{id}/run-now"),
        Some(json!({"projectId": project})),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["schedule"]["lastStatus"], "completed");

    let calls = fakes.http.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].url, "https://hooks.example.com/ping");
}
