//! End-to-end HTTP tests for system mail (fake mailer + templates).

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
use server::infra::config::AppConfig;
use server::infra::provider_tester::ConfigProviderTester;
use server::ports::ProviderTester;
use server::testing::{FakeAdminStore, FakeAuditStore, FakeAuthStore, FakeMailer, FakeTemplates};
use tower::ServiceExt;

fn app_with_mail() -> (Router, Arc<FakeMailer>) {
    let auth_store = Arc::new(FakeAuthStore::new());
    let audit = Arc::new(AuditService::new(Arc::new(FakeAuditStore::new())));
    let mailer = Arc::new(FakeMailer::new());
    let templates = Arc::new(FakeTemplates::new());
    templates.seed(
        "verify-email",
        "Verify your account",
        "<a href=\"{{link}}\">Verify</a>",
        "Verify: {{link}}",
    );
    templates.seed(
        "password-reset",
        "Reset your password",
        "<a href=\"{{link}}\">Reset</a>",
        "Reset: {{link}}",
    );
    templates.seed(
        "login-notice",
        "New sign-in to your account",
        "<p>Signed in {{when}}</p>",
        "Signed in {{when}}",
    );
    templates.seed(
        "password-changed",
        "Your password was changed",
        "<p>Changed. Sign in: {{link}}</p>",
        "Changed. Sign in: {{link}}",
    );
    let auth = Arc::new(
        AuthService::new(auth_store.clone(), true, audit.clone()).with_system_mail(
            mailer.clone(),
            templates,
            "http://localhost:3000".to_string(),
        ),
    );
    let admin = Arc::new(AdminService::new(
        Box::new(FakeAdminStore::new()),
        true,
        audit.clone(),
    ));
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
                projects: None,
                project_members: None,
                audit: Some(audit),
                recipients: None,
                templates: None,
                channel_providers: None,
                tickets: None,
                notifications: None,
                billing: None,
                provider_tester: Arc::new(ConfigProviderTester::new())
                    as Arc<dyn ProviderTester + Send + Sync>,
            },
            &AppConfig::default(),
        ),
        mailer,
    )
}

async fn post(
    app: Router,
    uri: &str,
    body: Value,
    cookie: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", format!("session_token={cookie}"));
    }
    app.oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

#[tokio::test]
async fn signup_sends_verification_email() {
    let (app, mailer) = app_with_mail();
    let res = post(
        app.clone(),
        "/app/auth/signup",
        json!({"name": "Mae", "email": "mae@x.dev", "password": "Sup3rSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let sent = mailer.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "mae@x.dev");
    assert_eq!(sent[0].subject, "Verify your account");
}

#[tokio::test]
async fn forgot_and_resend_send_reset_emails_without_enumeration() {
    let (app, mailer) = app_with_mail();
    let res = post(
        app.clone(),
        "/app/auth/signup",
        json!({"name": "Ray", "email": "ray@x.dev", "password": "Sup3rSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(mailer.sent().len(), 1);

    // Unknown address: still 200, nothing queued.
    let res = post(
        app.clone(),
        "/app/auth/password/forgot",
        json!({"email": "ghost@x.dev"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(mailer.sent().len(), 1);

    // Known address: reset email queued.
    let res = post(
        app.clone(),
        "/app/auth/password/forgot",
        json!({"email": "ray@x.dev"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sent = mailer.sent();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].subject, "Reset your password");
}

#[tokio::test]
async fn password_login_sends_login_notice() {
    let (app, mailer) = app_with_mail();
    let res = post(
        app.clone(),
        "/app/auth/signup",
        json!({"name": "Lou", "email": "lou@x.dev", "password": "Sup3rSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(mailer.sent().len(), 1);

    // A fresh password sign-in queues the login notice alongside it.
    let res = post(
        app.clone(),
        "/app/auth/login",
        json!({"email": "lou@x.dev", "password": "Sup3rSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let sent = mailer.sent();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1].to, "lou@x.dev");
    assert_eq!(sent[1].subject, "New sign-in to your account");
    // Automated mail always sends as no-reply, never a human address.
    assert_eq!(sent[1].from, server::ports::SystemSender::NoReply);
}

#[tokio::test]
async fn completing_a_reset_sends_password_changed_notice() {
    let (app, mailer) = app_with_mail();
    let res = post(
        app.clone(),
        "/app/auth/signup",
        json!({"name": "Nia", "email": "nia@x.dev", "password": "Sup3rSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = post(
        app.clone(),
        "/app/auth/password/forgot",
        json!({"email": "nia@x.dev"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let reset_token = body_json(res).await["resetToken"]
        .as_str()
        .unwrap()
        .to_string();

    let res = post(
        app.clone(),
        "/app/auth/password/reset",
        json!({"token": reset_token, "password": "An0therSecret!"}),
        None,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);

    // verify-email + password-reset + password-changed, in that order.
    let sent = mailer.sent();
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[2].to, "nia@x.dev");
    assert_eq!(sent[2].subject, "Your password was changed");
    assert_eq!(sent[2].from, server::ports::SystemSender::NoReply);
}
