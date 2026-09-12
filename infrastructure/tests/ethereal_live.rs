//! Live SMTP verification against a real relay (Ethereal in dev).
//! Reads credentials from the environment — never commit them.
//!
//! Run explicitly (never in CI):
//! ```sh
//! NOTIFI_SMTP_HOST=smtp.ethereal.email NOTIFI_SMTP_PORT=587 \
//! NOTIFI_SMTP_USERNAME=... NOTIFI_SMTP_PASSWORD=... \
//! NOTIFI_MAIL_DOMAIN=ethereal.email \
//! cargo test --test ethereal_live -- --ignored --nocapture
//! ```

use server::infra::config::MailConfig;
use server::infra::{FileSystemTemplates, LiveSmtpMailer};
use server::ports::mailer::{SmtpMailer, SystemSender, SystemTemplates};

#[tokio::test]
#[ignore = "needs live SMTP credentials in the environment (dev only)"]
async fn live_ethereal_send() {
    let config = MailConfig {
        smtp_host: std::env::var("NOTIFI_SMTP_HOST").ok(),
        smtp_port: std::env::var("NOTIFI_SMTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok()),
        smtp_username: std::env::var("NOTIFI_SMTP_USERNAME").ok(),
        smtp_password: std::env::var("NOTIFI_SMTP_PASSWORD").ok(),
        mail_domain: std::env::var("NOTIFI_MAIL_DOMAIN").unwrap_or_else(|_| "notifi.dev".into()),
        smtp_tls: true,
        templates_dir: None,
        ..Default::default()
    };
    assert!(config.enabled(), "NOTIFI_SMTP_HOST must be set");

    let templates = FileSystemTemplates::new("assets/templates");
    for (name, vars) in [
        (
            "verify-email",
            vec![
                ("name", "Ethereal Check"),
                ("link", "https://example.com/verify?token=abc"),
            ],
        ),
        (
            "password-reset",
            vec![
                ("name", "Ethereal Check"),
                ("link", "https://example.com/reset?token=abc"),
            ],
        ),
        (
            "login-notice",
            vec![
                ("name", "Ethereal Check"),
                ("when", "September 11, 2026 at 12:00 PM UTC"),
                ("link", "https://example.com/profile"),
            ],
        ),
        (
            "member-invite",
            vec![
                ("name", "Ethereal Check"),
                ("project", "Acme"),
                ("role", "developer"),
                ("link", "https://example.com/invite/abc"),
            ],
        ),
        (
            "password-changed",
            vec![
                ("name", "Ethereal Check"),
                ("link", "https://example.com/auth/login"),
            ],
        ),
    ] {
        let rendered = templates.render(name, &vars).unwrap();
        assert!(
            !rendered.html.contains("{{"),
            "{name} has unsubstituted variables"
        );
        let mailer = LiveSmtpMailer::from_config(&config).expect("mailer builds");
        let reference = mailer
            .send(
                SystemSender::NoReply,
                "inbox-check@example.com",
                &rendered.subject,
                &rendered.text,
                Some(&rendered.html),
            )
            .await
            .expect("Ethereal send succeeds");
        println!("Ethereal accepted {name} (smtp ref: {reference})");
    }
    println!("View them at https://ethereal.email/messages (Ethereal account inbox)");
}
