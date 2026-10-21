//! End-to-end HTTP tests for project API keys + per-key IP allowlists.
//!
//! Runs without a database. Follows the recipients/templates flow: sign up +
//! log in, complete onboarding to create a real project, seed visibility in
//! the fake keys store, then exercise issuance (secret shown once),
//! lifecycle, and IP allowlist endpoints.

mod common;

use std::sync::Arc;

use crate::common::{FakeApiKeysStore, FakeAuditStore, FakeAuthStore};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use server::api::build_router;
use server::api::state::AppState;
use server::domain::api_keys::ApiKeyService;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::projects::ProjectService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use tower::ServiceExt;

/// App wired with fake auth + api-keys services.
fn app_with_api_keys() -> (Router, Arc<FakeApiKeysStore>) {
    let store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(store.clone(), audit.clone()));
    let keys_store = Arc::new(FakeApiKeysStore::new());
    let api_keys = Arc::new(ApiKeyService::new(keys_store.clone(), audit.clone()));
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
                project_members: None,
                audit: Some(audit),
                recipients: None,
                api_keys: Some(api_keys),
                templates: None,
                channel_providers: None,
                tickets: None,
                notifications: None,
                billing: None,
                schedules: None,
                webhooks: None,
                campaigns: None,
                inbound: None,
                provider_tester: std::sync::Arc::new(ConfigProviderTester::new())
                    as std::sync::Arc<dyn ProviderTester + Send + Sync>,
                flows: None,
                sends: None,
            },
            &AppConfig::default(),
        ),
        keys_store,
    )
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn signup_and_login_on(app: Router, email: &str) -> (String, String) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Jane", "email": email, "password": "Sup3rSecret!"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/app/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"email": email, "password": "Sup3rSecret!", "rememberMe": false})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let user_id = body["user"]["id"].as_str().unwrap().to_string();
    let token = body["session"]["token"].as_str().unwrap().to_string();
    (token, user_id)
}

async fn post_with_cookie(
    app: Router,
    uri: &str,
    body: Value,
    cookie: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn get_with_cookie(app: Router, uri: &str, cookie: &str) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("GET")
            .uri(uri)
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn patch_with_cookie(
    app: Router,
    uri: &str,
    body: Value,
    cookie: &str,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("PATCH")
            .uri(uri)
            .header("content-type", "application/json")
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn delete_with_cookie(app: Router, uri: &str, cookie: &str) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("DELETE")
            .uri(uri)
            .header("cookie", format!("session_token={cookie}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
}

/// Signs up, creates a project via onboarding, grants visibility in the keys
/// store, and returns (cookie, uid, project_id).
async fn onboard_project_on(
    app: Router,
    keys_store: &Arc<FakeApiKeysStore>,
    email: &str,
    project_name: &str,
) -> (String, String, String) {
    let (token, uid) = signup_and_login_on(app.clone(), email).await;

    let res = post_with_cookie(
        app.clone(),
        "/app/auth/onboarding/complete",
        json!({"project": {"name": project_name, "description": "test"}}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = get_with_cookie(app.clone(), "/app/projects", &token).await;
    let project_id = body_json(res).await["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    keys_store.seed_visible(&uid, &project_id);
    (token, uid, project_id)
}

#[tokio::test]
async fn api_keys_require_auth() {
    let (app, _store) = app_with_api_keys();
    let res = get_with_cookie(app, "/app/projects/p1/api-keys", "bogus").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_returns_secret_once_and_lists_without_it() {
    let (app, store) = app_with_api_keys();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "keys1@example.com", "My App").await;

    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        json!({"name": "Prod key", "permissions": ["read", "write"]}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = body_json(res).await;
    let secret = body["secret"].as_str().unwrap().to_string();
    assert!(secret.starts_with("napi_"));
    assert_eq!(body["key"]["name"], "Prod key");
    assert_eq!(body["key"]["prefix"], "napi_");
    // Stored record never echoes the secret back on reads.
    let key_id = body["key"]["id"].as_str().unwrap().to_string();

    let res = get_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    assert_eq!(body["keys"].as_array().unwrap().len(), 1);
    assert_eq!(body["keys"][0]["id"], key_id);
    assert!(body["keys"][0].get("secret").is_none());

    let res = get_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert!(body_json(res).await.get("secret").is_none());
}

#[tokio::test]
async fn create_validates_inputs() {
    let (app, store) = app_with_api_keys();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "keys2@example.com", "My App").await;

    // Empty name → 400.
    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        json!({"name": "  "}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Unknown permission → 400.
    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        json!({"name": "K", "permissions": ["admin"]}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Duplicate name for the project → 409.
    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        json!({"name": "Dup"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        json!({"name": "Dup"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn rename_disable_and_revoke_roundtrip() {
    let (app, store) = app_with_api_keys();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "keys3@example.com", "My App").await;

    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        json!({"name": "Old"}),
        &token,
    )
    .await;
    let key_id = body_json(res).await["key"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Rename.
    let res = patch_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}"),
        json!({"name": "New"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["key"]["name"], "New");

    // Empty patch → 400.
    let res = patch_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}"),
        json!({}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Disable.
    let res = patch_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}"),
        json!({"enabled": false}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["key"]["enabled"], false);

    // Revoke is idempotent — twice still 200 with revokedAt set.
    for _ in 0..2 {
        let res = post_with_cookie(
            app.clone(),
            &format!("/app/projects/{pid}/api-keys/{key_id}/revoke"),
            json!({}),
            &token,
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert!(body_json(res).await["key"]["revokedAt"].is_string());
    }
}

#[tokio::test]
async fn ip_allowlist_crud_and_validation() {
    let (app, store) = app_with_api_keys();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "keys4@example.com", "My App").await;

    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        json!({"name": "K"}),
        &token,
    )
    .await;
    let key_id = body_json(res).await["key"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Empty allowlist reads as empty (unrestricted).
    let res = get_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}/ips"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["ips"].as_array().unwrap().len(), 0);

    // Bare IP normalizes to a host route.
    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}/ips"),
        json!({"cidr": "203.0.113.7", "label": "Office"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let entry = body_json(res).await["ip"].clone();
    assert_eq!(entry["cidr"], "203.0.113.7/32");
    let entry_id = entry["id"].as_str().unwrap().to_string();

    // Duplicate CIDR → 409.
    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}/ips"),
        json!({"cidr": "203.0.113.7/32"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CONFLICT);

    // Garbage → 400.
    let res = post_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}/ips"),
        json!({"cidr": "not-an-ip"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Remove, then removing again → 404.
    let res = delete_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}/ips/{entry_id}"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let res = delete_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys/{key_id}/ips/{entry_id}"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn keys_are_not_visible_across_users() {
    let (app, store) = app_with_api_keys();
    let (_t1, _u1, pid) =
        onboard_project_on(app.clone(), &store, "keys5a@example.com", "App A").await;
    let (token2, uid2, _pid2) =
        onboard_project_on(app.clone(), &store, "keys5b@example.com", "App B").await;
    // token2's project is different; pid from user A must read as missing.
    // (Do NOT seed user B onto user A's project.)
    let _ = uid2;
    let res = get_with_cookie(
        app.clone(),
        &format!("/app/projects/{pid}/api-keys"),
        &token2,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["keys"].as_array().unwrap().len(), 0);
}
