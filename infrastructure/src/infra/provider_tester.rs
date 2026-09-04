use async_trait::async_trait;

use crate::ports::provider_tester::{ProviderTester, TestResult};

pub struct ConfigProviderTester;

impl Default for ConfigProviderTester {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigProviderTester {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ProviderTester for ConfigProviderTester {
    async fn test(&self, channel_id: &str, provider_id: &str, config: &serde_json::Value) -> TestResult {
        match channel_id {
            "email" => test_email_provider(provider_id, config).await,
            "sms" => test_sms_provider(provider_id, config).await,
            "push" => test_push_provider(provider_id, config).await,
            "chat" => test_chat_provider(provider_id, config).await,
            _ => TestResult {
                success: false,
                message: format!("Unknown channel: {channel_id}"),
            },
        }
    }
}

async fn test_email_provider(provider_id: &str, config: &serde_json::Value) -> TestResult {
    match provider_id {
        "smtp" => test_smtp(config).await,
        "sendgrid" => test_sendgrid(config).await,
        "resend" => test_resend(config).await,
        "aws_ses" => test_aws_ses(config).await,
        "postmark" => test_postmark(config).await,
        "mailgun" => test_mailgun(config).await,
        "brevo" => test_brevo(config).await,
        _ => TestResult {
            success: false,
            message: format!("Unknown email provider: {provider_id}"),
        },
    }
}

async fn test_smtp(config: &serde_json::Value) -> TestResult {
    let host = config.get("host").and_then(|v| v.as_str()).unwrap_or("");
    let port = config.get("port").and_then(|v| v.as_u64()).unwrap_or(587) as u16;
    let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if host.is_empty() {
        return TestResult { success: false, message: "SMTP host is required".to_string() };
    }
    if username.is_empty() {
        return TestResult { success: false, message: "SMTP username is required".to_string() };
    }
    if password.is_empty() {
        return TestResult { success: false, message: "SMTP password is required".to_string() };
    }

    // Try to connect to the SMTP server
    let addr = format!("{host}:{port}");
    match tokio::net::TcpStream::connect(&addr).await {
        Ok(_stream) => TestResult {
            success: true,
            message: format!("Connected to SMTP server at {addr}"),
        },
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to connect to SMTP server at {addr}: {e}"),
        },
    }
}

async fn test_sendgrid(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "SendGrid API key is required".to_string() };
    }

    // Validate API key format (starts with SG.)
    if !api_key.starts_with("SG.") {
        return TestResult {
            success: false,
            message: "Invalid SendGrid API key format (should start with SG.)".to_string(),
        };
    }

    // Try to verify the API key by calling the SendGrid API
    let client = reqwest::Client::new();
    match client
        .get("https://api.sendgrid.com/v3/user/profile")
        .bearer_auth(api_key)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "SendGrid API key is valid".to_string(),
        },
        Ok(resp) => {
            let status = resp.status();
            TestResult {
                success: false,
                message: format!("SendGrid API returned status {status}"),
            }
        }
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify SendGrid API key: {e}"),
        },
    }
}

async fn test_resend(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "Resend API key is required".to_string() };
    }

    // Try to verify the API key
    let client = reqwest::Client::new();
    match client
        .get("https://api.resend.com/domains")
        .bearer_auth(api_key)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "Resend API key is valid".to_string(),
        },
        Ok(resp) => {
            let status = resp.status();
            TestResult {
                success: false,
                message: format!("Resend API returned status {status}"),
            }
        }
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify Resend API key: {e}"),
        },
    }
}

async fn test_aws_ses(config: &serde_json::Value) -> TestResult {
    let access_key = config.get("access_key").and_then(|v| v.as_str()).unwrap_or("");
    let secret_key = config.get("secret_key").and_then(|v| v.as_str()).unwrap_or("");
    let region = config.get("region").and_then(|v| v.as_str()).unwrap_or("");

    if access_key.is_empty() {
        return TestResult { success: false, message: "AWS access key is required".to_string() };
    }
    if secret_key.is_empty() {
        return TestResult { success: false, message: "AWS secret key is required".to_string() };
    }
    if region.is_empty() {
        return TestResult { success: false, message: "AWS region is required".to_string() };
    }

    // Basic format validation
    if !access_key.starts_with("AKIA") {
        return TestResult {
            success: false,
            message: "Invalid AWS access key format (should start with AKIA)".to_string(),
        };
    }

    TestResult {
        success: true,
        message: format!("AWS SES credentials format is valid for region {region}"),
    }
}

async fn test_postmark(config: &serde_json::Value) -> TestResult {
    let server_token = config.get("server_token").and_then(|v| v.as_str()).unwrap_or("");

    if server_token.is_empty() {
        return TestResult { success: false, message: "Postmark server token is required".to_string() };
    }

    // Try to verify the server token
    let client = reqwest::Client::new();
    match client
        .get("https://api.postmarkapp.com/stats/outbound")
        .header("Accept", "application/json")
        .header("X-Postmark-Server-Token", server_token)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "Postmark server token is valid".to_string(),
        },
        Ok(resp) => {
            let status = resp.status();
            TestResult {
                success: false,
                message: format!("Postmark API returned status {status}"),
            }
        }
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify Postmark server token: {e}"),
        },
    }
}

async fn test_mailgun(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let domain = config.get("domain").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "Mailgun API key is required".to_string() };
    }
    if domain.is_empty() {
        return TestResult { success: false, message: "Mailgun domain is required".to_string() };
    }

    // Try to verify the API key
    let client = reqwest::Client::new();
    match client
        .get(format!("https://api.mailgun.net/v3/{domain}/events"))
        .basic_auth("api", Some(api_key))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: format!("Mailgun API key is valid for domain {domain}"),
        },
        Ok(resp) => {
            let status = resp.status();
            TestResult {
                success: false,
                message: format!("Mailgun API returned status {status}"),
            }
        }
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify Mailgun API key: {e}"),
        },
    }
}

async fn test_brevo(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "Brevo API key is required".to_string() };
    }

    // Try to verify the API key
    let client = reqwest::Client::new();
    match client
        .get("https://api.brevo.com/v3/account")
        .header("accept", "application/json")
        .header("api-key", api_key)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "Brevo API key is valid".to_string(),
        },
        Ok(resp) => {
            let status = resp.status();
            TestResult {
                success: false,
                message: format!("Brevo API returned status {status}"),
            }
        }
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify Brevo API key: {e}"),
        },
    }
}

async fn test_sms_provider(provider_id: &str, config: &serde_json::Value) -> TestResult {
    match provider_id {
        "twilio" => test_twilio_sms(config).await,
        "termii" => test_termii(config).await,
        "africas_talking" => test_africas_talking(config).await,
        "infobip" => test_infobip(config).await,
        "vonage" => test_vonage(config).await,
        _ => TestResult {
            success: true,
            message: format!("SMS provider {provider_id} credentials saved (validation not available)"),
        },
    }
}

async fn test_twilio_sms(config: &serde_json::Value) -> TestResult {
    let account_sid = config.get("account_sid").and_then(|v| v.as_str()).unwrap_or("");
    let auth_token = config.get("auth_token").and_then(|v| v.as_str()).unwrap_or("");

    if account_sid.is_empty() {
        return TestResult { success: false, message: "Twilio Account SID is required".to_string() };
    }
    if auth_token.is_empty() {
        return TestResult { success: false, message: "Twilio Auth Token is required".to_string() };
    }

    // Validate Account SID format (starts with AC)
    if !account_sid.starts_with("AC") {
        return TestResult {
            success: false,
            message: "Invalid Twilio Account SID format (should start with AC)".to_string(),
        };
    }

    // Try to verify credentials
    let client = reqwest::Client::new();
    match client
        .get(format!("https://api.twilio.com/2010-04-01/Accounts/{account_sid}.json"))
        .basic_auth(account_sid, Some(auth_token))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "Twilio credentials are valid".to_string(),
        },
        Ok(resp) => {
            let status = resp.status();
            TestResult {
                success: false,
                message: format!("Twilio API returned status {status}"),
            }
        }
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify Twilio credentials: {e}"),
        },
    }
}

async fn test_termii(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "Termii API key is required".to_string() };
    }

    TestResult {
        success: true,
        message: "Termii API key format is valid".to_string(),
    }
}

async fn test_africas_talking(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "Africa's Talking API key is required".to_string() };
    }
    if username.is_empty() {
        return TestResult { success: false, message: "Africa's Talking username is required".to_string() };
    }

    TestResult {
        success: true,
        message: "Africa's Talking credentials format is valid".to_string(),
    }
}

async fn test_infobip(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let base_url = config.get("base_url").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "Infobip API key is required".to_string() };
    }
    if base_url.is_empty() {
        return TestResult { success: false, message: "Infobip base URL is required".to_string() };
    }

    TestResult {
        success: true,
        message: "Infobip credentials format is valid".to_string(),
    }
}

async fn test_vonage(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let api_secret = config.get("api_secret").and_then(|v| v.as_str()).unwrap_or("");

    if api_key.is_empty() {
        return TestResult { success: false, message: "Vonage API key is required".to_string() };
    }
    if api_secret.is_empty() {
        return TestResult { success: false, message: "Vonage API secret is required".to_string() };
    }

    TestResult {
        success: true,
        message: "Vonage credentials format is valid".to_string(),
    }
}

async fn test_push_provider(provider_id: &str, config: &serde_json::Value) -> TestResult {
    match provider_id {
        "fcm" => test_fcm(config).await,
        "apns" => test_apns(config).await,
        "onesignal" => test_onesignal(config).await,
        _ => TestResult {
            success: true,
            message: format!("Push provider {provider_id} credentials saved (validation not available)"),
        },
    }
}

async fn test_fcm(config: &serde_json::Value) -> TestResult {
    let service_account_key = config.get("service_account_key").and_then(|v| v.as_str()).unwrap_or("");

    if service_account_key.is_empty() {
        return TestResult { success: false, message: "FCM service account key is required".to_string() };
    }

    // Try to parse as JSON
    match serde_json::from_str::<serde_json::Value>(service_account_key) {
        Ok(json) => {
            if json.get("type").and_then(|v| v.as_str()) == Some("service_account") {
                TestResult {
                    success: true,
                    message: "FCM service account key format is valid".to_string(),
                }
            } else {
                TestResult {
                    success: false,
                    message: "Invalid FCM service account key (missing 'type: service_account')".to_string(),
                }
            }
        }
        Err(_) => TestResult {
            success: false,
            message: "Invalid FCM service account key (not valid JSON)".to_string(),
        },
    }
}

async fn test_apns(config: &serde_json::Value) -> TestResult {
    let key_id = config.get("key_id").and_then(|v| v.as_str()).unwrap_or("");
    let team_id = config.get("team_id").and_then(|v| v.as_str()).unwrap_or("");
    let bundle_id = config.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
    let private_key = config.get("private_key").and_then(|v| v.as_str()).unwrap_or("");

    if key_id.is_empty() {
        return TestResult { success: false, message: "APNS Key ID is required".to_string() };
    }
    if team_id.is_empty() {
        return TestResult { success: false, message: "APNS Team ID is required".to_string() };
    }
    if bundle_id.is_empty() {
        return TestResult { success: false, message: "APNS Bundle ID is required".to_string() };
    }
    if private_key.is_empty() {
        return TestResult { success: false, message: "APNS Private Key is required".to_string() };
    }

    // Check if private key looks like a P8 key
    if !private_key.contains("BEGIN PRIVATE KEY") {
        return TestResult {
            success: false,
            message: "Invalid APNS private key format (should be a P8 key)".to_string(),
        };
    }

    TestResult {
        success: true,
        message: "APNS credentials format is valid".to_string(),
    }
}

async fn test_onesignal(config: &serde_json::Value) -> TestResult {
    let app_id = config.get("app_id").and_then(|v| v.as_str()).unwrap_or("");
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

    if app_id.is_empty() {
        return TestResult { success: false, message: "OneSignal App ID is required".to_string() };
    }
    if api_key.is_empty() {
        return TestResult { success: false, message: "OneSignal API key is required".to_string() };
    }

    TestResult {
        success: true,
        message: "OneSignal credentials format is valid".to_string(),
    }
}

async fn test_chat_provider(provider_id: &str, config: &serde_json::Value) -> TestResult {
    match provider_id {
        "slack" => test_slack(config).await,
        "telegram" => test_telegram(config).await,
        "discord" => test_discord(config).await,
        _ => TestResult {
            success: true,
            message: format!("Chat provider {provider_id} credentials saved (validation not available)"),
        },
    }
}

async fn test_slack(config: &serde_json::Value) -> TestResult {
    let webhook_url = config.get("webhook_url").and_then(|v| v.as_str()).unwrap_or("");
    let bot_token = config.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");

    if webhook_url.is_empty() && bot_token.is_empty() {
        return TestResult {
            success: false,
            message: "Slack webhook URL or bot token is required".to_string(),
        };
    }

    if !webhook_url.is_empty() {
        // Try to verify webhook URL format
        if !webhook_url.contains("hooks.slack.com") {
            return TestResult {
                success: false,
                message: "Invalid Slack webhook URL (should contain hooks.slack.com)".to_string(),
            };
        }
    }

    if !bot_token.is_empty() {
        // Validate bot token format
        if !bot_token.starts_with("xoxb-") {
            return TestResult {
                success: false,
                message: "Invalid Slack bot token format (should start with xoxb-)".to_string(),
            };
        }
    }

    TestResult {
        success: true,
        message: "Slack credentials format is valid".to_string(),
    }
}

async fn test_telegram(config: &serde_json::Value) -> TestResult {
    let bot_token = config.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
    let chat_id = config.get("chat_id").and_then(|v| v.as_str()).unwrap_or("");

    if bot_token.is_empty() {
        return TestResult { success: false, message: "Telegram bot token is required".to_string() };
    }
    if chat_id.is_empty() {
        return TestResult { success: false, message: "Telegram chat ID is required".to_string() };
    }

    // Validate bot token format (numbers:alphanumeric)
    if !bot_token.contains(':') {
        return TestResult {
            success: false,
            message: "Invalid Telegram bot token format (should contain ':')".to_string(),
        };
    }

    // Try to get bot info
    let client = reqwest::Client::new();
    match client
        .get(format!("https://api.telegram.org/bot{bot_token}/getMe"))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "Telegram bot token is valid".to_string(),
        },
        Ok(resp) => {
            let status = resp.status();
            TestResult {
                success: false,
                message: format!("Telegram API returned status {status}"),
            }
        }
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify Telegram bot token: {e}"),
        },
    }
}

async fn test_discord(config: &serde_json::Value) -> TestResult {
    let webhook_url = config.get("webhook_url").and_then(|v| v.as_str()).unwrap_or("");
    let bot_token = config.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");

    if webhook_url.is_empty() && bot_token.is_empty() {
        return TestResult {
            success: false,
            message: "Discord webhook URL or bot token is required".to_string(),
        };
    }

    if !webhook_url.is_empty() {
        // Try to verify webhook URL format
        if !webhook_url.contains("discord.com/api/webhooks/") && !webhook_url.contains("discordapp.com/api/webhooks/") {
            return TestResult {
                success: false,
                message: "Invalid Discord webhook URL".to_string(),
            };
        }
    }

    if !bot_token.is_empty() {
        // Try to get bot info
        let client = reqwest::Client::new();
        match client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {bot_token}"))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => TestResult {
                success: true,
                message: "Discord bot token is valid".to_string(),
            },
            Ok(resp) => {
                let status = resp.status();
                TestResult {
                    success: false,
                    message: format!("Discord API returned status {status}"),
                }
            }
            Err(e) => TestResult {
                success: false,
                message: format!("Failed to verify Discord bot token: {e}"),
            },
        }
    } else {
        TestResult {
            success: true,
            message: "Discord webhook URL format is valid".to_string(),
        }
    }
}
