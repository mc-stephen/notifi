//! Notifi HTTP API — composition root.
//!
//! M0: bootstraps telemetry and configuration. M1 replaces the SMTP smoke
//! test below with the axum application core (routing, middleware, health).

use email_channel::{EmailConfig, EmailMessage, EmailProvider};
use notifi_infra_config::AppConfig;
use notifi_infra_telemetry as telemetry;

#[tokio::main]
async fn main() {
    let _guard = match telemetry::init() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("failed to initialize telemetry: {e}");
            return;
        }
    };

    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("invalid configuration: {e}");
            return;
        }
    };

    tracing::info!(
        host = %config.server.host,
        port = config.server.port,
        "api bootstrap complete"
    );

    // M0 smoke test: send an email through the SMTP channel when a tenant
    // config exists at `{NOTIFI_CONFIG_ROOT}/brands/{brand}/config/email/`
    // (channel standardization on ConfigResolver arrives in M3).
    let Ok(email_config) = EmailConfig::load("acme") else {
        tracing::warn!("email config not found; skipping SMTP smoke test (see README)");
        return;
    };
    let provider = EmailProvider::new(email_config);

    let msg = EmailMessage {
        to: vec!["user@example.com".to_string()],
        cc: vec!["manager@acme.com".to_string()],
        bcc: vec!["audit@acme.com".to_string()],
        reply_to: Some("support@acme.com".to_string()),
        subject: "Hello from Notifi".to_string(),
        body_text: "This is a test notification.".to_string(),
        body_html: Some("<h1>Test</h1><p>This is a test notification.</p>".to_string()),
    };

    match provider.send_mail(&msg).await {
        Ok(()) => tracing::info!("email sent successfully"),
        Err(e) => tracing::error!("email send failed: {e}"),
    }
}
