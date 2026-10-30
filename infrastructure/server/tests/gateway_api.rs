//! End-to-end HTTP tests for the device gateway surface (`GET
//! /gateway/{topic}`) — fake-backed queue.

mod common;

use std::sync::Arc;
use std::time::Duration;

use crate::common::FakeGatewayStore;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use server::api::build_router;
use server::api::state::AppState;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::GatewayStore;
use server::ports::ProviderTester;
use tower::ServiceExt;

/// Router over only the gateway service (no other surfaces wired).
fn app_with_gateway(store: Option<Arc<FakeGatewayStore>>) -> Router {
    build_router(
        AppState {
            db: None,
            redis: None,
            auth: None,
            oauth: None,
            admin: None,
            admin_users: None,
            admin_projects: None,
            admin_notifications: None,
            projects: None,
            project_exports: None,
            project_imports: None,
            project_members: None,
            audit: None,
            recipients: None,
            api_keys: None,
            templates: None,
            channel_providers: None,
            tickets: None,
            billing: None,
            flows: None,
            sends: None,
            schedules: None,
            webhooks: None,
            campaigns: None,
            notifications: None,
            inbound: None,
            gateway: store.map(|s| s as Arc<dyn GatewayStore + Send + Sync>),
            provider_tester: Arc::new(ConfigProviderTester::new())
                as Arc<dyn ProviderTester + Send + Sync>,
            config_store: Arc::new(server::infra::FileConfigStore::new("configs")),
        },
        &AppConfig::default(),
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn poll_on(app: Router, topic: &str, query: Option<&str>) -> axum::response::Response {
    let uri = match query {
        Some(query) => format!("/gateway/{topic}?{query}"),
        None => format!("/gateway/{topic}"),
    };
    app.oneshot(
        Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

/// Plain check on an empty route answers an empty list.
#[tokio::test]
async fn poll_empty_route_answers_empty_messages() {
    let store = Arc::new(FakeGatewayStore::new());
    let res = poll_on(
        app_with_gateway(Some(store.clone())),
        "room-1",
        Some("key=k1"),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["messages"].as_array().unwrap().len(), 0);
}

/// A push then a poll delivers the message and reads it away — the second
/// poll finds nothing (poll-and-delete).
#[tokio::test]
async fn poll_delivers_message_once_then_drops_it() {
    let store = Arc::new(FakeGatewayStore::new());
    store
        .push("room-1", "k1", "Build done", "linux box ping")
        .await
        .unwrap();
    let app = app_with_gateway(Some(store.clone()));

    let res = poll_on(app.clone(), "room-1", Some("key=k1")).await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["title"], "Build done");
    assert_eq!(messages[0]["body"], "linux box ping");
    assert!(messages[0]["received_at"].is_string());

    // Gone at first read: a repeat poll sees an empty queue.
    let res = poll_on(app, "room-1", Some("key=k1")).await;
    assert_eq!(
        body_json(res).await["messages"].as_array().unwrap().len(),
        0
    );
}

/// A message queued mid-hold surfaces before the wait budget runs out.
#[tokio::test]
async fn long_poll_wakes_on_mid_hold_push() {
    let store = Arc::new(FakeGatewayStore::new());
    let pusher = store.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        pusher
            .push("room-9", "k9", "now", "late arrival")
            .await
            .unwrap();
    });

    let app = app_with_gateway(Some(store));
    let started = std::time::Instant::now();
    let res = poll_on(app, "room-9", Some("key=k9&wait=3")).await;
    assert_eq!(res.status(), StatusCode::OK);
    let messages = body_json(res).await["messages"]
        .as_array()
        .unwrap()
        .to_vec();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["title"], "now");
    // The hold ended on the push, not at the full budget stop (buffer 1s).
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// A wrong key stars the route out with an empty queue — no existence leak.
#[tokio::test]
async fn poll_with_wrong_key_answers_empty() {
    let store = Arc::new(FakeGatewayStore::new());
    store
        .push("room-1", "k1", "secret", "for the right key")
        .await
        .unwrap();
    let res = poll_on(app_with_gateway(Some(store)), "room-1", Some("key=wrong")).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        body_json(res).await["messages"].as_array().unwrap().len(),
        0
    );
}

/// A blank key is refused before the queue is touched.
#[tokio::test]
async fn poll_without_key_is_refused() {
    let res = poll_on(
        app_with_gateway(Some(Arc::new(FakeGatewayStore::new()))),
        "room-1",
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

/// Without the gateway store wired (no database) the route answers 503.
#[tokio::test]
async fn poll_without_store_answers_503() {
    let res = poll_on(app_with_gateway(None), "room-1", Some("key=k1")).await;
    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}
