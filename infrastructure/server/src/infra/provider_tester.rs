use async_trait::async_trait;

use crate::channels::chat_channel::{
    discord::{DiscordConfig, DiscordProvider},
    slack::{SlackConfig, SlackProvider},
    telegram::{TelegramConfig, TelegramProvider},
    whatsapp_business::{WhatsAppBusinessConfig, WhatsAppBusinessProvider},
};
use crate::channels::device_channel::android::fcm::{FcmConfig, FcmProvider};
use crate::channels::device_channel::apple::apns::{ApnsProvider, IosConfig};
use crate::channels::device_channel::web::web_push::{WebConfig, WebProvider};
use crate::channels::device_channel::window::wns::WnsAuthenticator;
use crate::channels::other_channel::OtherChannelConfig;
use crate::channels::other_channel::onesignal::{OneSignalConfig, OneSignalProvider};
use crate::channels::other_channel::pushy::{PushyConfig, PushyProvider};
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
    async fn test(
        &self,
        channel_id: &str,
        provider_id: &str,
        config: &serde_json::Value,
    ) -> TestResult {
        match channel_id {
            "email" => test_email_provider(provider_id, config).await,
            "sms" => test_sms_provider(provider_id, config).await,
            "chat" => test_chat_provider(provider_id, config).await,
            "other" => test_other_provider(provider_id, config).await,
            // Device destinations: Firebase proves its service account
            // for real (token exchange); the rest stay house-fallback
            // until their controllers wire up.
            "window" | "android" | "apple" | "web" => {
                test_device_provider(provider_id, config).await
            }
            _ => TestResult {
                success: false,
                message: format!("Unknown channel: {channel_id}"),
            },
        }
    }
}

/// Device-destination probes. Firebase (`fcm`) proves its service
/// account for real (token exchange); Apple (`apns`) loads the `.p8`
/// key and signs offline (APNs has no dry-run); Pushy does an
/// authenticated device-info read; OneSignal shares the other
/// channel's probe (gets reused); WNS exchanges with Microsoft login;
/// web push signs its VAPID pair offline; unknown ids fall to the
/// saved-without-check fallback.
async fn test_device_provider(provider_id: &str, config: &serde_json::Value) -> TestResult {
    match provider_id {
        "fcm" => {
            let Ok(cfg) = serde_json::from_value::<FcmConfig>(config.clone()) else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for fcm (service_account_path required)"
                        .to_string(),
                };
            };
            match FcmProvider::new(cfg).verify().await {
                Ok(()) => TestResult {
                    success: true,
                    message: "Firebase accepted the service account (token granted)".to_string(),
                },
                Err(e) => TestResult {
                    success: false,
                    message: format!("Firebase rejected the credential: {e}"),
                },
            }
        }
        "pushy" => {
            let Ok(cfg) = serde_json::from_value::<PushyConfig>(config.clone()) else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for pushy (api_key required)".to_string(),
                };
            };
            match PushyProvider::new(cfg).verify().await {
                Ok(()) => TestResult {
                    success: true,
                    message: "Pushy accepted the api key (device-info read, nothing sent)"
                        .to_string(),
                },
                Err(e) => TestResult {
                    success: false,
                    message: format!("Pushy rejected the credential: {e}"),
                },
            }
        }
        "apns" => {
            let Ok(cfg) = serde_json::from_value::<IosConfig>(config.clone()) else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for apns (key_id, team_id, bundle_id, \
                              p8_key_path required)"
                        .to_string(),
                };
            };
            match ApnsProvider::new(cfg).and_then(|provider| provider.verify()) {
                Ok(()) => TestResult {
                    success: true,
                    message: "Apple signing key loads and signs (gateway untouched)".to_string(),
                },
                Err(e) => TestResult {
                    success: false,
                    message: format!("Apple rejected the credential: {e}"),
                },
            }
        }
        "onesignal" => {
            let Ok(cfg) = serde_json::from_value::<OneSignalConfig>(config.clone()) else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for onesignal (app_id + a key required)"
                        .to_string(),
                };
            };
            match OneSignalProvider::new(cfg).verify().await {
                Ok(()) => TestResult {
                    success: true,
                    message: "OneSignal accepted the app id + REST key (notification list read, nothing sent)"
                        .to_string(),
                },
                Err(e) => TestResult {
                    success: false,
                    message: format!("OneSignal rejected the credential: {e}"),
                },
            }
        }
        "wns" => {
            let client_id = config
                .get("client_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let client_secret = config
                .get("client_secret")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if client_id.is_empty() || client_secret.is_empty() {
                return TestResult {
                    success: false,
                    message: "WNS requires client_id and client_secret".to_string(),
                };
            }
            let authenticator = WnsAuthenticator::new(
                client_id.to_string(),
                client_secret.to_string(),
                config
                    .get("package_sid")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
            );
            match authenticator.get_token().await {
                Ok(_) => TestResult {
                    success: true,
                    message: "Microsoft granted the WNS token (credential is live)".to_string(),
                },
                Err(e) => TestResult {
                    success: false,
                    message: format!("Microsoft rejected the WNS credential: {e}"),
                },
            }
        }
        "web_push" => {
            let Ok(cfg) = serde_json::from_value::<WebConfig>(config.clone()) else {
                return TestResult {
                    success: false,
                    message:
                        "Config does not parse for web_push (VAPID keypair + contact required)"
                            .to_string(),
                };
            };
            match WebProvider::new(cfg).and_then(|provider| provider.verify()) {
                Ok(()) => TestResult {
                    success: true,
                    message: "VAPID keypair loads and signs (gateway untouched)".to_string(),
                },
                Err(e) => TestResult {
                    success: false,
                    message: format!("Web push rejected the credential: {e}"),
                },
            }
        }
        _ => TestResult {
            success: true,
            message: format!(
                "Device provider {provider_id} credentials saved (validation not available)"
            ),
        },
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
    let username = config
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let password = config
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if host.is_empty() {
        return TestResult {
            success: false,
            message: "SMTP host is required".to_string(),
        };
    }
    if username.is_empty() {
        return TestResult {
            success: false,
            message: "SMTP username is required".to_string(),
        };
    }
    if password.is_empty() {
        return TestResult {
            success: false,
            message: "SMTP password is required".to_string(),
        };
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
        return TestResult {
            success: false,
            message: "SendGrid API key is required".to_string(),
        };
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
        return TestResult {
            success: false,
            message: "Resend API key is required".to_string(),
        };
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
    let access_key = config
        .get("access_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let secret_key = config
        .get("secret_key")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let region = config.get("region").and_then(|v| v.as_str()).unwrap_or("");

    if access_key.is_empty() {
        return TestResult {
            success: false,
            message: "AWS access key is required".to_string(),
        };
    }
    if secret_key.is_empty() {
        return TestResult {
            success: false,
            message: "AWS secret key is required".to_string(),
        };
    }
    if region.is_empty() {
        return TestResult {
            success: false,
            message: "AWS region is required".to_string(),
        };
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
    let server_token = config
        .get("server_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if server_token.is_empty() {
        return TestResult {
            success: false,
            message: "Postmark server token is required".to_string(),
        };
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
        return TestResult {
            success: false,
            message: "Mailgun API key is required".to_string(),
        };
    }
    if domain.is_empty() {
        return TestResult {
            success: false,
            message: "Mailgun domain is required".to_string(),
        };
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
        return TestResult {
            success: false,
            message: "Brevo API key is required".to_string(),
        };
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
        "bird" => test_bird(config).await,
        "ebulksms" => test_ebulksms(config).await,
        "smslive247" => test_smslive247(config).await,
        _ => TestResult {
            success: true,
            message: format!(
                "SMS provider {provider_id} credentials saved (validation not available)"
            ),
        },
    }
}

async fn test_twilio_sms(config: &serde_json::Value) -> TestResult {
    let account_sid = config
        .get("account_sid")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let auth_token = config
        .get("auth_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if account_sid.is_empty() {
        return TestResult {
            success: false,
            message: "Twilio Account SID is required".to_string(),
        };
    }
    if auth_token.is_empty() {
        return TestResult {
            success: false,
            message: "Twilio Auth Token is required".to_string(),
        };
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
        .get(format!(
            "https://api.twilio.com/2010-04-01/Accounts/{account_sid}.json"
        ))
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
        return TestResult {
            success: false,
            message: "Termii API key is required".to_string(),
        };
    }

    TestResult {
        success: true,
        message: "Termii API key format is valid".to_string(),
    }
}

async fn test_africas_talking(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let username = config
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if api_key.is_empty() {
        return TestResult {
            success: false,
            message: "Africa's Talking API key is required".to_string(),
        };
    }
    if username.is_empty() {
        return TestResult {
            success: false,
            message: "Africa's Talking username is required".to_string(),
        };
    }

    TestResult {
        success: true,
        message: "Africa's Talking credentials format is valid".to_string(),
    }
}

async fn test_infobip(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let base_url = config
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if api_key.is_empty() {
        return TestResult {
            success: false,
            message: "Infobip API key is required".to_string(),
        };
    }
    if base_url.is_empty() {
        return TestResult {
            success: false,
            message: "Infobip base URL is required".to_string(),
        };
    }

    TestResult {
        success: true,
        message: "Infobip credentials format is valid".to_string(),
    }
}

async fn test_vonage(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let api_secret = config
        .get("api_secret")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if api_key.is_empty() {
        return TestResult {
            success: false,
            message: "Vonage API key is required".to_string(),
        };
    }
    if api_secret.is_empty() {
        return TestResult {
            success: false,
            message: "Vonage API secret is required".to_string(),
        };
    }

    TestResult {
        success: true,
        message: "Vonage credentials format is valid".to_string(),
    }
}

/// Bird: a real authenticated read — the balance endpoint proves the
/// access key without sending anything.
async fn test_bird(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let base_url = config
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if api_key.is_empty() {
        return TestResult {
            success: false,
            message: "Bird API key is required".to_string(),
        };
    }
    if base_url.is_empty() {
        return TestResult {
            success: false,
            message: "Bird base URL is required".to_string(),
        };
    }

    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(e) => {
            return TestResult {
                success: false,
                message: format!("Failed to build HTTP client: {e}"),
            };
        }
    };
    match client
        .get(format!("{}/balance", base_url.trim_end_matches('/')))
        .header("Authorization", format!("AccessKey {api_key}"))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "Bird access key is valid (balance endpoint answered)".to_string(),
        },
        Ok(resp) => TestResult {
            success: false,
            message: format!("Bird balance check returned status {}", resp.status()),
        },
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify Bird access key: {e}"),
        },
    }
}

/// EbulkSms: a real authenticated read — the balance endpoint proves
/// the username + apikey pair without sending anything.
async fn test_ebulksms(config: &serde_json::Value) -> TestResult {
    let username = config
        .get("username")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let base_url = config
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://api.ebulksms.com");

    if username.is_empty() {
        return TestResult {
            success: false,
            message: "EbulkSms username is required".to_string(),
        };
    }
    if api_key.is_empty() {
        return TestResult {
            success: false,
            message: "EbulkSms API key is required".to_string(),
        };
    }

    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(e) => {
            return TestResult {
                success: false,
                message: format!("Failed to build HTTP client: {e}"),
            };
        }
    };
    match client
        .get(format!(
            "{}/balance/{}/{}",
            base_url.trim_end_matches('/'),
            username,
            api_key
        ))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "EbulkSms credentials are valid (balance endpoint answered)".to_string(),
        },
        Ok(resp) => TestResult {
            success: false,
            message: format!("EbulkSms balance check returned status {}", resp.status()),
        },
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify EbulkSms credentials: {e}"),
        },
    }
}

/// SmsLive247: a real authenticated read — the self-account endpoint
/// proves the api key without sending anything.
async fn test_smslive247(config: &serde_json::Value) -> TestResult {
    let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    let base_url = config
        .get("base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://api.smslive247.com");

    if api_key.is_empty() {
        return TestResult {
            success: false,
            message: "SmsLive247 API key is required".to_string(),
        };
    }

    let client = match reqwest::Client::builder().build() {
        Ok(c) => c,
        Err(e) => {
            return TestResult {
                success: false,
                message: format!("Failed to build HTTP client: {e}"),
            };
        }
    };
    match client
        .get(format!(
            "{}/api/v4/accounts/self",
            base_url.trim_end_matches('/')
        ))
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => TestResult {
            success: true,
            message: "SmsLive247 API key is valid (account endpoint answered)".to_string(),
        },
        Ok(resp) => TestResult {
            success: false,
            message: format!("SmsLive247 account check returned status {}", resp.status()),
        },
        Err(e) => TestResult {
            success: false,
            message: format!("Failed to verify SmsLive247 API key: {e}"),
        },
    }
}

/// Chat-destination probes. Slack bots read auth.test (webhook flavors
/// honestly carry nothing to verify); Telegram reads getMe; Discord
/// GETs its webhook URL or the bot identity — both post nothing;
/// WhatsApp reads the phone info; Teams webhooks only send. Unknown ids
/// fall to the saved-without-check fallback.
async fn test_chat_provider(provider_id: &str, config: &serde_json::Value) -> TestResult {
    match provider_id {
        "slack" => test_slack(config).await,
        "telegram" => test_telegram(config).await,
        "whatsapp_business" => test_whatsapp(config).await,
        "discord" => test_discord(config).await,
        "ms_teams" => test_ms_teams(config).await,
        _ => TestResult {
            success: true,
            message: format!(
                "Chat provider {provider_id} credentials saved (validation not available)"
            ),
        },
    }
}

async fn test_slack(config: &serde_json::Value) -> TestResult {
    // The webhook flavor cannot be read without posting — the URL is
    // the credential, so its shape is the only honest check there.
    let webhook_url = config
        .get("webhook_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let bot_token = config
        .get("bot_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if webhook_url.is_empty() && bot_token.is_empty() {
        return TestResult {
            success: false,
            message: "Slack webhook URL or bot token is required".to_string(),
        };
    }

    if !webhook_url.is_empty() && !webhook_url.contains("hooks.slack.com") {
        return TestResult {
            success: false,
            message: "Invalid Slack webhook URL (should contain hooks.slack.com)".to_string(),
        };
    }

    if !bot_token.is_empty() {
        // The bot flavor proves itself with auth.test — an
        // authenticated read that posts nothing.
        let Ok(cfg) = serde_json::from_value::<SlackConfig>(config.clone()) else {
            return TestResult {
                success: false,
                message: "Config does not parse for slack (webhook or bot flavor, \
                          type-tagged)"
                    .to_string(),
            };
        };
        return match SlackProvider::new(cfg).verify().await {
            Ok(()) => TestResult {
                success: true,
                message: "Slack accepted the bot token (auth.test read, nothing posted)"
                    .to_string(),
            },
            Err(e) => TestResult {
                success: false,
                message: format!("Slack rejected the credential: {e}"),
            },
        };
    }

    TestResult {
        success: true,
        message: "Slack webhook saved — nothing to verify until it fires; the URL \
                  cannot be read without posting"
            .to_string(),
    }
}

async fn test_telegram(config: &serde_json::Value) -> TestResult {
    // getMe is an authenticated read of the bot's identity that sends
    // nothing.
    let Ok(cfg) = serde_json::from_value::<TelegramConfig>(config.clone()) else {
        return TestResult {
            success: false,
            message: "Config does not parse for telegram (bot_token + chat_id required)"
                .to_string(),
        };
    };
    match TelegramProvider::new(cfg).verify().await {
        Ok(()) => TestResult {
            success: true,
            message: "Telegram accepted the bot token (getMe read, nothing sent)".to_string(),
        },
        Err(e) => TestResult {
            success: false,
            message: format!("Telegram rejected the credential: {e}"),
        },
    }
}

async fn test_discord(config: &serde_json::Value) -> TestResult {
    let webhook_url = config
        .get("webhook_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let bot_token = config
        .get("bot_token")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if webhook_url.is_empty() && bot_token.is_empty() {
        return TestResult {
            success: false,
            message: "Discord webhook URL or bot token is required".to_string(),
        };
    }

    // A webhook GET returns the webhook object without posting; the
    // bot flavor reads the authenticated user. Both reveal nothing
    // sendable.
    let Ok(cfg) = serde_json::from_value::<DiscordConfig>(config.clone()) else {
        return TestResult {
            success: false,
            message: "Config does not parse for discord (webhook or bot flavor, \
                      type-tagged)"
                .to_string(),
        };
    };
    match DiscordProvider::new(cfg).verify().await {
        Ok(()) => TestResult {
            success: true,
            message: "Discord accepted the credential (a read that posts nothing)".to_string(),
        },
        Err(e) => TestResult {
            success: false,
            message: format!("Discord rejected the credential: {e}"),
        },
    }
}

async fn test_whatsapp(config: &serde_json::Value) -> TestResult {
    // The phone info read is authenticated and sends nothing.
    let Ok(cfg) = serde_json::from_value::<WhatsAppBusinessConfig>(config.clone()) else {
        return TestResult {
            success: false,
            message: "Config does not parse for whatsapp_business (phone_number_id + \
                      access_token required)"
                .to_string(),
        };
    };
    match WhatsAppBusinessProvider::new(cfg).verify().await {
        Ok(()) => TestResult {
            success: true,
            message: "WhatsApp accepted the system-user token (phone info read, nothing \
                      sent)"
                .to_string(),
        },
        Err(e) => TestResult {
            success: false,
            message: format!("WhatsApp rejected the credential: {e}"),
        },
    }
}

async fn test_ms_teams(config: &serde_json::Value) -> TestResult {
    let webhook_url = config
        .get("webhook_url")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if webhook_url.is_empty() {
        return TestResult {
            success: false,
            message: "MS Teams requires webhook_url".to_string(),
        };
    }
    // Teams webhooks only send — there is no read that proves the URL
    // without messaging a channel for real.
    TestResult {
        success: true,
        message: "MS Teams webhook saved — nothing to verify until it fires; Teams \
                  webhooks only send"
            .to_string(),
    }
}

/// Other-channel probes. OneSignal shares the device family's real read
/// (same vendor, same REST surface, handed the bare fields after the
/// envelope parses); pushy does the same hand-off (device-info read);
/// the relay recipe honestly reports that a webhook cannot be read
/// without posting to it; the gateway checks its route recipe
/// house-side. The envelope parse pins the vendor tag against the arm —
/// a flat save's fields parse to nothing.
async fn test_other_provider(provider_id: &str, config: &serde_json::Value) -> TestResult {
    match provider_id {
        "onesignal" => {
            let Ok(OtherChannelConfig::OneSignal {
                app_id,
                api_key,
                rest_api_key,
            }) = serde_json::from_value::<OtherChannelConfig>(config.clone())
            else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for the other channel's OneSignal (the \
                              saved envelope carries the provider tag plus its fields)"
                        .to_string(),
                };
            };
            if app_id.trim().is_empty()
                || (api_key.trim().is_empty()
                    && rest_api_key.as_deref().unwrap_or("").trim().is_empty())
            {
                return TestResult {
                    success: false,
                    message: "OneSignal needs an app id plus an API key or REST API key"
                        .to_string(),
                };
            }
            // The device arms own the read — hand them the bare fields.
            let bare = serde_json::json!({
                "app_id": app_id,
                "api_key": api_key,
                "rest_api_key": rest_api_key
            });
            test_device_provider("onesignal", &bare).await
        }
        "pushy" => {
            let Ok(OtherChannelConfig::Pushy { api_key, app_id }) =
                serde_json::from_value::<OtherChannelConfig>(config.clone())
            else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for the other channel's pushy (the \
                              saved envelope carries the provider tag plus its fields)"
                        .to_string(),
                };
            };
            if api_key.trim().is_empty() {
                return TestResult {
                    success: false,
                    message: "Pushy needs an api key".to_string(),
                };
            }
            // The device arm owns the read — hand it the bare fields.
            let bare = serde_json::json!({ "api_key": api_key, "app_id": app_id });
            test_device_provider("pushy", &bare).await
        }
        "relay" => {
            let Ok(OtherChannelConfig::Relay { target_url, .. }) =
                serde_json::from_value::<OtherChannelConfig>(config.clone())
            else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for the other channel's relay (the \
                              saved envelope carries the provider tag plus its fields)"
                        .to_string(),
                };
            };
            if target_url.trim().is_empty() {
                return TestResult {
                    success: false,
                    message: "Relay needs a target URL".to_string(),
                };
            }
            // Relay webhooks only send — there is no read that proves
            // the URL without posting a payload for real.
            TestResult {
                success: true,
                message: "Relay saved — nothing to verify until it fires; relay webhooks \
                          only send"
                    .to_string(),
            }
        }
        // The Notifi gateway: no vendor API exists to shake hands with —
        // the honest probe is house-side (the route recipe's shape); the
        // route proves itself once the user's app polls it.
        "gateway" => {
            let Ok(OtherChannelConfig::Gateway {
                topic,
                secret_key,
                mode: _,
            }) = serde_json::from_value::<OtherChannelConfig>(config.clone())
            else {
                return TestResult {
                    success: false,
                    message: "Config does not parse for the other channel's gateway (the \
                              saved envelope carries the provider tag plus its fields)"
                        .to_string(),
                };
            };
            if topic.trim().is_empty() {
                TestResult {
                    success: false,
                    message: "The gateway recipe needs its topic (the route path)".to_string(),
                }
            } else if secret_key.len() < 24 {
                TestResult {
                    success: false,
                    message: "The route secret looks too short (regenerate it — the connect \
                              form makes 30+ characters)"
                        .to_string(),
                }
            } else {
                TestResult {
                    success: true,
                    message: "Route recipe checks out (topic + secret key) — the route \
                              proves itself when your app polls"
                        .to_string(),
                }
            }
        }
        _ => TestResult {
            success: true,
            message: format!(
                "Other provider {provider_id} credentials saved (validation not available)"
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fcm_probe_rejects_unparseable_or_broken_credentials() {
        let t = ConfigProviderTester
            .test("android", "fcm", &serde_json::json!({}))
            .await;
        assert!(!t.success);
        assert!(t.message.contains("does not parse"), "{}", t.message);

        // A JSON file that is not a real service account fails the token
        // exchange offline — the probe reports Firebase's rejection.
        let sa = std::env::temp_dir().join(format!("notifi-fcm-probe-{}.json", std::process::id()));
        std::fs::write(&sa, r#"{"project_id": "proj-1"}"#).unwrap();
        let t = ConfigProviderTester
            .test(
                "android",
                "fcm",
                &serde_json::json!({"service_account_path": sa.to_string_lossy()}),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("Firebase rejected"), "{}", t.message);
    }

    #[tokio::test]
    async fn unwired_device_providers_keep_the_house_fallback() {
        // Every registry vendor now probes for real; the fallback
        // remains for ids that never existed.
        let t = ConfigProviderTester
            .test(
                "android",
                "not_a_vendor",
                &serde_json::json!({"some": "config"}),
            )
            .await;
        assert!(t.success);
        assert!(t.message.contains("validation not available"));
    }

    #[tokio::test]
    async fn pushy_probe_rejects_a_bad_config_before_any_network() {
        let t = ConfigProviderTester
            .test("android", "pushy", &serde_json::json!({}))
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("does not parse"), "{}", t.message);
    }

    #[tokio::test]
    async fn other_pushy_probe_asks_at_least_for_the_key() {
        // Blank keys fail pre-network with the vendor's ask.
        let t = ConfigProviderTester
            .test(
                "other",
                "pushy",
                &serde_json::json!({"provider": "pushy", "api_key": " "}),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("api key"), "{}", t.message);
    }

    #[tokio::test]
    async fn other_onesignal_probe_rides_the_device_read() {
        // Blank keys fail pre-network with the vendor's ask.
        let t = ConfigProviderTester
            .test(
                "other",
                "onesignal",
                &serde_json::json!({"provider": "onesignal", "app_id": "a"}),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(
            t.message.contains("app id plus an API key"),
            "{}",
            t.message
        );
    }

    #[tokio::test]
    async fn other_arms_refuse_flat_saves() {
        // The envelope's provider tag is the connect flow's job — flat
        // fields alone parse to nothing and the probe says so.
        let t = ConfigProviderTester
            .test(
                "other",
                "onesignal",
                &serde_json::json!({"app_id": "a", "api_key": "k"}),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("does not parse"), "{}", t.message);

        let t = ConfigProviderTester
            .test("other", "pushy", &serde_json::json!({"api_key": "k"}))
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("does not parse"), "{}", t.message);

        let t = ConfigProviderTester
            .test(
                "other",
                "relay",
                &serde_json::json!({"target_url": "https://x/hook"}),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("does not parse"), "{}", t.message);
    }

    #[tokio::test]
    async fn relay_probe_reports_its_read_limit_honestly() {
        let t = ConfigProviderTester
            .test(
                "other",
                "relay",
                &serde_json::json!({
                    "provider": "relay",
                    "target_url": "https://x/hook",
                    "method": "POST",
                    "payload_template": "{}"
                }),
            )
            .await;
        assert!(t.success, "{}", t.message);
        assert!(t.message.contains("only send"), "{}", t.message);

        let t = ConfigProviderTester
            .test(
                "other",
                "relay",
                &serde_json::json!({
                    "provider": "relay",
                    "target_url": " ",
                    "method": "POST",
                    "payload_template": "{}"
                }),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("target URL"), "{}", t.message);
    }

    #[tokio::test]
    async fn gateway_probe_checks_the_route_recipe_house_side() {
        // A recipe missing its topic fails before anything else.
        let t = ConfigProviderTester
            .test(
                "other",
                "gateway",
                &serde_json::json!({
                    "provider": "gateway",
                    "secret_key": "0123456789abcdef0123456789"
                }),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("topic"), "{}", t.message);

        // A stubbed-together key is refused (regenerated keys are 30+).
        let t = ConfigProviderTester
            .test(
                "other",
                "gateway",
                &serde_json::json!({
                    "provider": "gateway",
                    "topic": "gw-xk4p2m9tq7v3",
                    "secret_key": "short"
                }),
            )
            .await;
        assert!(!t.success, "{}", t.message);
        assert!(t.message.contains("too short"), "{}", t.message);

        // A full recipe passes — honestly, with a "your app proves it" line.
        let t = ConfigProviderTester
            .test(
                "other",
                "gateway",
                &serde_json::json!({
                    "provider": "gateway",
                    "topic": "gw-xk4p2m9tq7v3",
                    "secret_key": "0123456789abcdef0123456789",
                    "mode": "polling"
                }),
            )
            .await;
        assert!(t.success, "{}", t.message);
        assert!(t.message.contains("proves itself"), "{}", t.message);
    }
}
