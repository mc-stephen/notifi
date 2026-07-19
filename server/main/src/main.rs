use email_channel::{EmailConfig, EmailMessage, EmailProvider};

#[tokio::main]
async fn main() {
    let config = EmailConfig::load("acme").expect("failed to load email config");
    let provider = EmailProvider::new(config);

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
        Ok(()) => println!("Email sent successfully"),
        Err(e) => eprintln!("Send failed: {e}"),
    }
}
