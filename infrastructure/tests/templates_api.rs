//! End-to-end HTTP tests for the templates feature (fake-backed services).
//!
//! Runs without a database. Follows the recipients_api flow: sign up + log in,
//! complete onboarding to create a real project, then exercise the template
//! endpoints scoped to that project.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use server::api::build_router;
use server::api::state::AppState;
use server::domain::audit::AuditService;
use server::domain::auth::AuthService;
use server::domain::projects::ProjectService;
use server::domain::templates::TemplateService;
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAuditStore, FakeAuthStore, FakeTemplatesStore};
use serde_json::{Value, json};
use tower::ServiceExt;

/// App wired with fake auth + templates services.
fn app_with_templates() -> (Router, Arc<FakeTemplatesStore>) {
    let store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let auth = Arc::new(AuthService::new(store.clone(), true, audit.clone()));
    let projects = Arc::new(ProjectService::new(store.clone(), audit.clone()));
    let templates_store = Arc::new(FakeTemplatesStore::new());
    let templates = Arc::new(TemplateService::new(templates_store.clone(), audit.clone()));
    (
        build_router(
            AppState {
                db: None,
                redis: None,
                auth: Some(auth),
                oauth: None,
                projects: Some(projects),
                audit: Some(audit),
                recipients: None,
                templates: Some(templates),
                channel_providers: None,
                tickets: None,
                notifications: None,
                provider_tester: std::sync::Arc::new(ConfigProviderTester::new()) as std::sync::Arc<dyn ProviderTester + Send + Sync>,
            },
            &AppConfig::default(),
        ),
        templates_store,
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
                .uri("/v1/auth/signup")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name": "Jane", "email": email, "password": "Sup3rSecret!"})
                        .to_string(),
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
                .uri("/v1/auth/login")
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

async fn get_with_cookie(
    app: Router,
    uri: &str,
    cookie: &str,
) -> axum::response::Response {
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

async fn delete_with_cookie(
    app: Router,
    uri: &str,
    cookie: &str,
) -> axum::response::Response {
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

/// Signs up, creates a project via onboarding, grants the user access in the
/// template store, and returns (cookie, uid, project_id).
async fn onboard_project_on(
    app: Router,
    templates_store: &Arc<FakeTemplatesStore>,
    email: &str,
    project_name: &str,
) -> (String, String, String) {
    let (token, uid) = signup_and_login_on(app.clone(), email).await;

    let res = post_with_cookie(
        app.clone(),
        "/v1/auth/onboarding/complete",
        json!({"project": {"name": project_name, "description": "test"}}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    let res = get_with_cookie(app.clone(), "/v1/projects", &token).await;
    let project_id = body_json(res).await["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    templates_store.seed_visible(&uid, &project_id);
    (token, uid, project_id)
}

#[tokio::test]
async fn templates_require_auth() {
    let (app, _store) = app_with_templates();
    let res = get_with_cookie(app, "/v1/projects/p1/templates", "bogus").await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn create_and_list_and_get_and_delete_roundtrip() {
    let (app, store) = app_with_templates();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "tpl1@example.com", "My App").await;

    // create
    let res = post_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/templates"),
        json!({
            "name": "Welcome",
            "channel": "email",
            "content": {
                "subject": "Welcome {{name}}!",
                "html": "<p>Hi {{name}}</p>",
                "text": "Hi {{name}}"
            },
            "attachments": [
                {"name": "logo.png", "mimeType": "image/png", "sizeBytes": 1024, "url": "https://cdn.example/logo.png"}
            ]
        }),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let created = body_json(res).await["template"].clone();
    assert_eq!(created["name"], "Welcome");
    assert_eq!(created["channel"], "email");
    assert_eq!(created["content"]["subject"], "Welcome {{name}}!");
    assert_eq!(created["version"], 1);
    assert_eq!(created["attachments"].as_array().unwrap().len(), 1);
    assert_eq!(created["attachments"][0]["name"], "logo.png");
    let tid = created["id"].as_str().unwrap().to_string();

    // get
    let res = get_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/templates/{tid}"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let got = body_json(res).await["template"].clone();
    assert_eq!(got["name"], "Welcome");
    assert_eq!(got["attachments"][0]["url"], "https://cdn.example/logo.png");

    // list
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid}/templates"), &token).await;
    assert_eq!(res.status(), StatusCode::OK);
    let list = body_json(res).await;
    assert_eq!(list["templates"].as_array().unwrap().len(), 1);
    assert_eq!(list["hasMore"], false);

    // delete
    let res = delete_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/templates/{tid}"),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body_json(res).await["status"], "ok");

    // gone from list
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid}/templates"), &token).await;
    assert_eq!(body_json(res).await["templates"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn update_changes_content_and_replaces_attachments() {
    let (app, store) = app_with_templates();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "tpl2@example.com", "My App").await;

    let uri = format!("/v1/projects/{pid}/templates");
    let res = post_with_cookie(
        app.clone(),
        &uri,
        json!({
            "name": "Welcome",
            "channel": "email",
            "content": {"subject": "Hi", "html": "<p>old</p>"},
            "attachments": [{"name": "a.pdf", "mimeType": "application/pdf", "sizeBytes": 5, "url": "https://cdn/a.pdf"}]
        }),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let tid = body_json(res).await["template"]["id"].as_str().unwrap().to_string();

    // Update: new content, replaced attachments.
    let res = patch_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/templates/{tid}"),
        json!({
            "name": "Welcome v2",
            "description": "updated",
            "channel": "email",
            "content": {"subject": "Welcome {{name}}", "html": "<p>new</p>", "text": "new"},
            "attachments": [
                {"name": "b.pdf", "mimeType": "application/pdf", "sizeBytes": 900, "url": "https://cdn/b.pdf"},
                {"name": "c.png", "mimeType": "image/png", "sizeBytes": 2048, "url": "https://cdn/c.png"}
            ]
        }),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let updated = body_json(res).await["template"].clone();
    assert_eq!(updated["name"], "Welcome v2");
    assert_eq!(updated["version"], 2);
    assert_eq!(updated["content"]["text"], "new");
    let atts = updated["attachments"].as_array().unwrap();
    assert_eq!(atts.len(), 2);
    assert_eq!(atts[0]["name"], "b.pdf");
    assert_eq!(atts[1]["name"], "c.png");
}

#[tokio::test]
async fn templates_are_isolated_per_project() {
    let (app, store) = app_with_templates();

    let (token_a, _uid_a, pid_a) =
        onboard_project_on(app.clone(), &store, "tpl3a@example.com", "App A").await;
    let (token_b, _uid_b, pid_b) =
        onboard_project_on(app.clone(), &store, "tpl3b@example.com", "App B").await;

    let res = post_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid_a}/templates"),
        json!({"name": "Mine", "channel": "email", "content": {"subject": "s"}}),
        &token_a,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    let tid = body_json(res).await["template"]["id"].as_str().unwrap().to_string();

    // User B's list must not show A's template.
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid_b}/templates"), &token_b).await;
    assert_eq!(body_json(res).await["templates"].as_array().unwrap().len(), 0);

    // User B fetching under project B must 404.
    let res = get_with_cookie(app.clone(), &format!("/v1/projects/{pid_b}/templates/{tid}"), &token_b).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_requires_name_and_rejects_bad_content() {
    let (app, store) = app_with_templates();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "tpl4@example.com", "My App").await;

    let uri = format!("/v1/projects/{pid}/templates");

    // Missing required name → 422 (deserialization).
    let res = post_with_cookie(app.clone(), &uri, json!({"channel": "sms"}), &token).await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Empty/blank name → 400 (service-level validation).
    let res = post_with_cookie(app.clone(), &uri, json!({"name": "   "}), &token).await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // Non-object content → 400.
    let res = post_with_cookie(
        app.clone(),
        &uri,
        json!({"name": "Bad", "content": [1, 2, 3]}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn update_and_delete_unknown_template_404() {
    let (app, store) = app_with_templates();
    let (token, _uid, pid) =
        onboard_project_on(app.clone(), &store, "tpl5@example.com", "My App").await;

    let res = patch_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/templates/does_not_exist"),
        json!({"name": "X", "channel": "email"}),
        &token,
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let res = delete_with_cookie(
        app.clone(),
        &format!("/v1/projects/{pid}/templates/does_not_exist"),
        &token,
    )
    .await;
    // Deleting something unknown is idempotent → success.
    assert_eq!(res.status(), StatusCode::OK);
}
