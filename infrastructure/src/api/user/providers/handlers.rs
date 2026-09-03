use axum::Json;
use serde::Serialize;

use crate::domain::channels::{
    ChannelDefinition, ConfigField, ConfigFieldType, ProviderDefinition, ProviderRegistry,
    ProviderScope, SmtpFallbackConfig,
};

/// Response type for GET /v1/providers
#[derive(Debug, Serialize)]
pub struct ProvidersResponse {
    pub version: String,
    pub last_updated: String,
    pub channels: Vec<ChannelDto>,
}

#[derive(Debug, Serialize)]
pub struct ChannelDto {
    pub channel_id: String,
    pub channel_name: String,
    pub providers: Vec<ProviderDto>,
}

#[derive(Debug, Serialize)]
pub struct ProviderDto {
    pub provider_id: String,
    pub name: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub primary_regions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
    pub config_fields: Vec<ConfigFieldDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_fallback: Option<SmtpFallbackDto>,
}

#[derive(Debug, Serialize)]
pub struct ConfigFieldDto {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct SmtpFallbackDto {
    pub fields: Vec<ConfigFieldDto>,
}

/// GET /v1/providers — returns the full provider registry.
/// This is derived from the Rust types — no hardcoded JSON.
pub async fn get_providers() -> Json<ProvidersResponse> {
    let registry = build_registry();
    Json(registry.into())
}

/// Build the provider registry from Rust types.
/// Each channel and provider is defined here — the compiler enforces completeness.
fn build_registry() -> ProviderRegistry {
    ProviderRegistry {
        version: "1.0.0".to_string(),
        last_updated: chrono::Utc::now().to_rfc3339(),
        channels: vec![
            build_email_channel(),
            build_sms_channel(),
            build_push_channel(),
            build_chat_channel(),
        ],
    }
}

fn build_email_channel() -> ChannelDefinition {
    ChannelDefinition {
        channel_id: "email".to_string(),
        channel_name: "Email".to_string(),
        providers: vec![
            ProviderDefinition {
                provider_id: "smtp".to_string(),
                name: "SMTP (Generic)".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: None,
                docs_url: Some("https://datatracker.ietf.org/doc/html/rfc5321".to_string()),
                config_fields: vec![
                    ConfigField { key: "host".to_string(), label: "SMTP Host".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "port".to_string(), label: "SMTP Port".to_string(), field_type: ConfigFieldType::Number, required: true },
                    ConfigField { key: "username".to_string(), label: "Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "password".to_string(), label: "Password".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                    ConfigField { key: "from_name".to_string(), label: "From Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "sendgrid".to_string(),
                name: "Twilio SendGrid".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/sendgrid.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://docs.sendgrid.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                    ConfigField { key: "from_name".to_string(), label: "From Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField { key: "smtp_host".to_string(), label: "SMTP Host".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_port".to_string(), label: "SMTP Port".to_string(), field_type: ConfigFieldType::Number, required: true },
                        ConfigField { key: "smtp_username".to_string(), label: "SMTP Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_password".to_string(), label: "SMTP Password".to_string(), field_type: ConfigFieldType::Password, required: true },
                        ConfigField { key: "smtp_from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                        ConfigField { key: "smtp_tls".to_string(), label: "TLS".to_string(), field_type: ConfigFieldType::Boolean, required: false },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "resend".to_string(),
                name: "Resend".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/resend.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://resend.com/docs".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                    ConfigField { key: "from_name".to_string(), label: "From Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField { key: "smtp_host".to_string(), label: "SMTP Host".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_port".to_string(), label: "SMTP Port".to_string(), field_type: ConfigFieldType::Number, required: true },
                        ConfigField { key: "smtp_username".to_string(), label: "SMTP Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_password".to_string(), label: "SMTP Password".to_string(), field_type: ConfigFieldType::Password, required: true },
                        ConfigField { key: "smtp_from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                        ConfigField { key: "smtp_tls".to_string(), label: "TLS".to_string(), field_type: ConfigFieldType::Boolean, required: false },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "aws_ses".to_string(),
                name: "Amazon SES".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://api.iconify.design/logos/aws-ses.svg".to_string()),
                docs_url: Some("https://docs.aws.amazon.com/ses/".to_string()),
                config_fields: vec![
                    ConfigField { key: "access_key".to_string(), label: "Access Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "secret_key".to_string(), label: "Secret Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "region".to_string(), label: "Region".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                    ConfigField { key: "from_name".to_string(), label: "From Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField { key: "smtp_host".to_string(), label: "SMTP Host".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_port".to_string(), label: "SMTP Port".to_string(), field_type: ConfigFieldType::Number, required: true },
                        ConfigField { key: "smtp_username".to_string(), label: "SMTP Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_password".to_string(), label: "SMTP Password".to_string(), field_type: ConfigFieldType::Password, required: true },
                        ConfigField { key: "smtp_from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                        ConfigField { key: "smtp_tls".to_string(), label: "TLS".to_string(), field_type: ConfigFieldType::Boolean, required: false },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "postmark".to_string(),
                name: "Postmark".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/postmarkapp.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://postmarkapp.com/developer".to_string()),
                config_fields: vec![
                    ConfigField { key: "server_token".to_string(), label: "Server Token".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                    ConfigField { key: "from_name".to_string(), label: "From Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField { key: "smtp_host".to_string(), label: "SMTP Host".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_port".to_string(), label: "SMTP Port".to_string(), field_type: ConfigFieldType::Number, required: true },
                        ConfigField { key: "smtp_username".to_string(), label: "SMTP Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_password".to_string(), label: "SMTP Password".to_string(), field_type: ConfigFieldType::Password, required: true },
                        ConfigField { key: "smtp_from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                        ConfigField { key: "smtp_tls".to_string(), label: "TLS".to_string(), field_type: ConfigFieldType::Boolean, required: false },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "mailgun".to_string(),
                name: "Mailgun".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/mailgun.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://documentation.mailgun.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "domain".to_string(), label: "Domain".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                    ConfigField { key: "from_name".to_string(), label: "From Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField { key: "smtp_host".to_string(), label: "SMTP Host".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_port".to_string(), label: "SMTP Port".to_string(), field_type: ConfigFieldType::Number, required: true },
                        ConfigField { key: "smtp_username".to_string(), label: "SMTP Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_password".to_string(), label: "SMTP Password".to_string(), field_type: ConfigFieldType::Password, required: true },
                        ConfigField { key: "smtp_from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                        ConfigField { key: "smtp_tls".to_string(), label: "TLS".to_string(), field_type: ConfigFieldType::Boolean, required: false },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "brevo".to_string(),
                name: "Brevo".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/brevo.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://developers.brevo.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                    ConfigField { key: "from_name".to_string(), label: "From Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField { key: "smtp_host".to_string(), label: "SMTP Host".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_port".to_string(), label: "SMTP Port".to_string(), field_type: ConfigFieldType::Number, required: true },
                        ConfigField { key: "smtp_username".to_string(), label: "SMTP Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                        ConfigField { key: "smtp_password".to_string(), label: "SMTP Password".to_string(), field_type: ConfigFieldType::Password, required: true },
                        ConfigField { key: "smtp_from_email".to_string(), label: "From Email".to_string(), field_type: ConfigFieldType::Email, required: true },
                        ConfigField { key: "smtp_tls".to_string(), label: "TLS".to_string(), field_type: ConfigFieldType::Boolean, required: false },
                    ],
                }),
            },
        ],
    }
}

fn build_sms_channel() -> ChannelDefinition {
    ChannelDefinition {
        channel_id: "sms".to_string(),
        channel_name: "SMS".to_string(),
        providers: vec![
            ProviderDefinition {
                provider_id: "twilio".to_string(),
                name: "Twilio".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/twilio.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://www.twilio.com/docs".to_string()),
                config_fields: vec![
                    ConfigField { key: "account_sid".to_string(), label: "Account SID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "auth_token".to_string(), label: "Auth Token".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "from_number".to_string(), label: "From Number".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "termii".to_string(),
                name: "Termii".to_string(),
                scope: ProviderScope::Regional,
                primary_regions: vec!["NG".to_string(), "GH".to_string(), "KE".to_string()],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/termii.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://developer.termii.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "sender_id".to_string(), label: "Sender ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "base_url".to_string(), label: "Base URL".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "africas_talking".to_string(),
                name: "Africa's Talking".to_string(),
                scope: ProviderScope::Regional,
                primary_regions: vec!["NG".to_string(), "GH".to_string(), "KE".to_string(), "UG".to_string()],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/africastalking.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://docs.africastalking.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "username".to_string(), label: "Username".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "sender_id".to_string(), label: "Sender ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "smslive247".to_string(),
                name: "SmsLive247".to_string(),
                scope: ProviderScope::Regional,
                primary_regions: vec!["NG".to_string()],
                platforms: vec![],
                icon_url: None,
                docs_url: Some("https://smslive247.com/docs".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "sender_id".to_string(), label: "Sender ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "ebulksms".to_string(),
                name: "EbulkSms".to_string(),
                scope: ProviderScope::Regional,
                primary_regions: vec!["NG".to_string()],
                platforms: vec![],
                icon_url: None,
                docs_url: Some("https://www.ebulksms.com/pages/api-docs".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "sender_id".to_string(), label: "Sender ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "infobip".to_string(),
                name: "Infobip".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/infobip.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://www.infobip.com/docs".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "base_url".to_string(), label: "Base URL".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "sender_id".to_string(), label: "Sender ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "vonage".to_string(),
                name: "Vonage".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/vonage.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://developer.nexmo.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "api_secret".to_string(), label: "API Secret".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "from_number".to_string(), label: "From Number".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "bird".to_string(),
                name: "Bird".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/bird.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://bird.com/developer".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "base_url".to_string(), label: "Base URL".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "sender_id".to_string(), label: "Sender ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
        ],
    }
}

fn build_push_channel() -> ChannelDefinition {
    ChannelDefinition {
        channel_id: "push".to_string(),
        channel_name: "Push".to_string(),
        providers: vec![
            ProviderDefinition {
                provider_id: "fcm".to_string(),
                name: "Firebase Cloud Messaging".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec!["android".to_string(), "web".to_string()],
                icon_url: Some("https://api.iconify.design/logos/firebase-icon.svg".to_string()),
                docs_url: Some("https://firebase.google.com/docs/cloud-messaging".to_string()),
                config_fields: vec![
                    ConfigField { key: "service_account_key".to_string(), label: "Service Account Key (JSON)".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "project_id".to_string(), label: "Project ID".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "apns".to_string(),
                name: "Apple Push Notification Service".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec!["ios".to_string(), "macos".to_string()],
                icon_url: Some("https://cdn.brandfetch.io/apple.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://developer.apple.com/documentation/usernotifications".to_string()),
                config_fields: vec![
                    ConfigField { key: "key_id".to_string(), label: "Key ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "team_id".to_string(), label: "Team ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "bundle_id".to_string(), label: "Bundle ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "private_key".to_string(), label: "Private Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "sandbox".to_string(), label: "Sandbox Mode".to_string(), field_type: ConfigFieldType::Boolean, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "onesignal".to_string(),
                name: "OneSignal".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec!["android".to_string(), "ios".to_string(), "web".to_string()],
                icon_url: Some("https://cdn.brandfetch.io/onesignal.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://documentation.onesignal.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "app_id".to_string(), label: "App ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "rest_api_key".to_string(), label: "REST API Key".to_string(), field_type: ConfigFieldType::Password, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "pushy".to_string(),
                name: "Pushy".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec!["android".to_string(), "ios".to_string()],
                icon_url: Some("https://cdn.brandfetch.io/pushy.me/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://pushy.me/docs".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "app_id".to_string(), label: "App ID".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "braze_airship".to_string(),
                name: "Braze (Airship)".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec!["android".to_string(), "ios".to_string(), "web".to_string()],
                icon_url: Some("https://cdn.brandfetch.io/braze.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://www.braze.com/docs/".to_string()),
                config_fields: vec![
                    ConfigField { key: "api_key".to_string(), label: "API Key".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "app_id".to_string(), label: "App ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "base_url".to_string(), label: "Base URL".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
        ],
    }
}

fn build_chat_channel() -> ChannelDefinition {
    ChannelDefinition {
        channel_id: "chat".to_string(),
        channel_name: "Chat".to_string(),
        providers: vec![
            ProviderDefinition {
                provider_id: "slack".to_string(),
                name: "Slack".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/slack.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://api.slack.com/".to_string()),
                config_fields: vec![
                    ConfigField { key: "type".to_string(), label: "Auth Type".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "webhook_url".to_string(), label: "Webhook URL".to_string(), field_type: ConfigFieldType::Text, required: false },
                    ConfigField { key: "bot_token".to_string(), label: "Bot Token".to_string(), field_type: ConfigFieldType::Password, required: false },
                    ConfigField { key: "channel_id".to_string(), label: "Channel ID".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "telegram".to_string(),
                name: "Telegram".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/telegram.org/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://core.telegram.org/bots/api".to_string()),
                config_fields: vec![
                    ConfigField { key: "bot_token".to_string(), label: "Bot Token".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "chat_id".to_string(), label: "Chat ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "whatsapp_business".to_string(),
                name: "WhatsApp Business".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/whatsapp.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://developers.facebook.com/docs/whatsapp/".to_string()),
                config_fields: vec![
                    ConfigField { key: "phone_number_id".to_string(), label: "Phone Number ID".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "access_token".to_string(), label: "Access Token".to_string(), field_type: ConfigFieldType::Password, required: true },
                    ConfigField { key: "api_version".to_string(), label: "API Version".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "discord".to_string(),
                name: "Discord".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/discord.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://discord.com/developers/docs".to_string()),
                config_fields: vec![
                    ConfigField { key: "type".to_string(), label: "Auth Type".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "webhook_url".to_string(), label: "Webhook URL".to_string(), field_type: ConfigFieldType::Text, required: false },
                    ConfigField { key: "bot_token".to_string(), label: "Bot Token".to_string(), field_type: ConfigFieldType::Password, required: false },
                    ConfigField { key: "channel_id".to_string(), label: "Channel ID".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "ms_teams".to_string(),
                name: "Microsoft Teams".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some("https://cdn.brandfetch.io/microsoft.com/w/512/h/512/theme/dark/icon.jpeg".to_string()),
                docs_url: Some("https://learn.microsoft.com/microsoftteams/".to_string()),
                config_fields: vec![
                    ConfigField { key: "webhook_url".to_string(), label: "Webhook URL".to_string(), field_type: ConfigFieldType::Text, required: true },
                    ConfigField { key: "channel_name".to_string(), label: "Channel Name".to_string(), field_type: ConfigFieldType::Text, required: false },
                ],
                smtp_fallback: None,
            },
        ],
    }
}

impl From<ProviderRegistry> for ProvidersResponse {
    fn from(registry: ProviderRegistry) -> Self {
        Self {
            version: registry.version,
            last_updated: registry.last_updated,
            channels: registry.channels.into_iter().map(|c| c.into()).collect(),
        }
    }
}

impl From<ChannelDefinition> for ChannelDto {
    fn from(channel: ChannelDefinition) -> Self {
        Self {
            channel_id: channel.channel_id,
            channel_name: channel.channel_name,
            providers: channel.providers.into_iter().map(|p| p.into()).collect(),
        }
    }
}

impl From<ProviderDefinition> for ProviderDto {
    fn from(provider: ProviderDefinition) -> Self {
        Self {
            provider_id: provider.provider_id,
            name: provider.name,
            scope: match provider.scope {
                ProviderScope::Global => "global".to_string(),
                ProviderScope::Regional => "regional".to_string(),
            },
            primary_regions: provider.primary_regions,
            platforms: provider.platforms,
            icon_url: provider.icon_url,
            docs_url: provider.docs_url,
            config_fields: provider.config_fields.into_iter().map(|f| f.into()).collect(),
            smtp_fallback: provider.smtp_fallback.map(|s| s.into()),
        }
    }
}

impl From<ConfigField> for ConfigFieldDto {
    fn from(field: ConfigField) -> Self {
        Self {
            key: field.key,
            label: field.label,
            field_type: match field.field_type {
                ConfigFieldType::Text => "text".to_string(),
                ConfigFieldType::Password => "password".to_string(),
                ConfigFieldType::Email => "email".to_string(),
                ConfigFieldType::Number => "number".to_string(),
                ConfigFieldType::Boolean => "boolean".to_string(),
            },
            required: field.required,
        }
    }
}

impl From<SmtpFallbackConfig> for SmtpFallbackDto {
    fn from(config: SmtpFallbackConfig) -> Self {
        Self {
            fields: config.fields.into_iter().map(|f| f.into()).collect(),
        }
    }
}
