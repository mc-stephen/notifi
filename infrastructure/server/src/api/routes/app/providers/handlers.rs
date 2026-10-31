use axum::Json;
use serde::Serialize;

use crate::channels::{device_channel, other_channel};
use crate::domain::channels::{
    ChannelDefinition, ConfigField, ConfigFieldType, ProviderDefinition, ProviderRegistry,
    ProviderScope, SmtpFallbackConfig,
};

/// Response type for GET /app/providers
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SmtpFallbackDto {
    pub fields: Vec<ConfigFieldDto>,
}

/// GET /app/providers — returns the full provider registry.
/// This is derived from the Rust types — no hardcoded JSON.
pub async fn get_providers() -> Json<ProvidersResponse> {
    let registry = build_registry();
    Json(registry.into())
}

/// Build the provider registry from Rust types. The three base channels
/// are defined here; the device family's rows derive from
/// `device_channel::catalog()` and the other channel's from
/// `other_channel::catalog()` — the compiler enforces
/// structural correctness on all of them.
fn build_registry() -> ProviderRegistry {
    let mut channels = vec![
        build_email_channel(),
        build_sms_channel(),
        build_chat_channel(),
    ];
    channels.extend(build_device_channels());
    channels.push(build_other_channel());
    ProviderRegistry {
        version: "1.5.0".to_string(),
        last_updated: chrono::Utc::now().to_rfc3339(),
        channels,
    }
}

/// The device family's registry rows — assembled from the family's own
/// catalog (`channels/device_channel/providers::catalog()`), so a new
/// vendor lands on the dashboard with its provider file and enum
/// registration alone; nothing is typed out here.
fn build_device_channels() -> Vec<ChannelDefinition> {
    device_channel::catalog()
        .into_iter()
        .map(|dest| {
            // Device rows are destination natives only — the
            // cross-platform vendors (OneSignal, Pushy) have their
            // single cards, and their controllers, in the other
            // channel's tree.
            ChannelDefinition {
                channel_id: dest.destination.to_string(),
                channel_name: dest.title.to_string(),
                providers: dest
                    .vendors
                    .into_iter()
                    .map(|v| ProviderDefinition {
                        provider_id: v.id.to_string(),
                        name: v.name.to_string(),
                        scope: ProviderScope::Global,
                        primary_regions: vec![],
                        platforms: v.platforms.iter().map(|p| p.to_string()).collect(),
                        icon_url: v.icon_url.map(str::to_string),
                        docs_url: v.docs_url.map(str::to_string),
                        config_fields: v
                            .fields
                            .into_iter()
                            .map(|f| {
                                let (field_type, options) = vendor_field_type(f.ty);
                                ConfigField {
                                    key: f.key.to_string(),
                                    label: f.label.to_string(),
                                    field_type,
                                    required: f.required,
                                    options,
                                }
                            })
                            .collect(),
                        smtp_fallback: None,
                    })
                    .collect(),
            }
        })
        .collect()
}

/// The other channel's registry rows — vendors that belong to no single
/// platform family (wrapped controllers from their owning families),
/// derived from the channel's own catalog
/// (`channels/other_channel::catalog()`), so nothing is
/// typed out here twice.
fn build_other_channel() -> ChannelDefinition {
    ChannelDefinition {
        channel_id: "other".to_string(),
        channel_name: "Other".to_string(),
        providers: other_channel::catalog()
            .into_iter()
            .map(|v| ProviderDefinition {
                provider_id: v.id.to_string(),
                name: v.name.to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: v.platforms.iter().map(|p| p.to_string()).collect(),
                icon_url: v.icon_url.map(str::to_string),
                docs_url: v.docs_url.map(str::to_string),
                config_fields: v
                    .fields
                    .into_iter()
                    .map(|f| {
                        let (field_type, options) = vendor_field_type(f.ty);
                        ConfigField {
                            key: f.key.to_string(),
                            label: f.label.to_string(),
                            field_type,
                            required: f.required,
                            options,
                        }
                    })
                    .collect(),
                smtp_fallback: None,
            })
            .collect(),
    }
}

/// Maps the vendor catalogs' connect-form field types onto the
/// registry's enum. The device family and the other channel share the
/// vocabulary; compiler-exhaustive both ways.
fn vendor_field_type(ty: device_channel::FieldType) -> (ConfigFieldType, Vec<String>) {
    use device_channel::FieldType;
    match ty {
        FieldType::Text => (ConfigFieldType::Text, Vec::new()),
        FieldType::Password => (ConfigFieldType::Password, Vec::new()),
        FieldType::Email => (ConfigFieldType::Email, Vec::new()),
        FieldType::Select(options) => (
            ConfigFieldType::Select,
            options.iter().map(|o| o.to_string()).collect(),
        ),
        FieldType::Boolean => (ConfigFieldType::Boolean, Vec::new()),
        FieldType::Hidden => (ConfigFieldType::Hidden, Vec::new()),
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
                    ConfigField {
                        key: "host".to_string(),
                        label: "SMTP Host".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "port".to_string(),
                        label: "SMTP Port".to_string(),
                        field_type: ConfigFieldType::Number,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "username".to_string(),
                        label: "Username".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "password".to_string(),
                        label: "Password".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_email".to_string(),
                        label: "From Email".to_string(),
                        field_type: ConfigFieldType::Email,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_name".to_string(),
                        label: "From Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "sendgrid".to_string(),
                name: "Twilio SendGrid".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/sendgrid.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://docs.sendgrid.com/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_email".to_string(),
                        label: "From Email".to_string(),
                        field_type: ConfigFieldType::Email,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_name".to_string(),
                        label: "From Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField {
                            key: "smtp_host".to_string(),
                            label: "SMTP Host".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_port".to_string(),
                            label: "SMTP Port".to_string(),
                            field_type: ConfigFieldType::Number,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_username".to_string(),
                            label: "SMTP Username".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_password".to_string(),
                            label: "SMTP Password".to_string(),
                            field_type: ConfigFieldType::Password,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_from_email".to_string(),
                            label: "From Email".to_string(),
                            field_type: ConfigFieldType::Email,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_tls".to_string(),
                            label: "TLS".to_string(),
                            field_type: ConfigFieldType::Boolean,
                            required: false,
                            options: Vec::new(),
                        },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "resend".to_string(),
                name: "Resend".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/resend.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://resend.com/docs".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_email".to_string(),
                        label: "From Email".to_string(),
                        field_type: ConfigFieldType::Email,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_name".to_string(),
                        label: "From Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField {
                            key: "smtp_host".to_string(),
                            label: "SMTP Host".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_port".to_string(),
                            label: "SMTP Port".to_string(),
                            field_type: ConfigFieldType::Number,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_username".to_string(),
                            label: "SMTP Username".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_password".to_string(),
                            label: "SMTP Password".to_string(),
                            field_type: ConfigFieldType::Password,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_from_email".to_string(),
                            label: "From Email".to_string(),
                            field_type: ConfigFieldType::Email,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_tls".to_string(),
                            label: "TLS".to_string(),
                            field_type: ConfigFieldType::Boolean,
                            required: false,
                            options: Vec::new(),
                        },
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
                    ConfigField {
                        key: "access_key".to_string(),
                        label: "Access Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "secret_key".to_string(),
                        label: "Secret Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "region".to_string(),
                        label: "Region".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_email".to_string(),
                        label: "From Email".to_string(),
                        field_type: ConfigFieldType::Email,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_name".to_string(),
                        label: "From Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField {
                            key: "smtp_host".to_string(),
                            label: "SMTP Host".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_port".to_string(),
                            label: "SMTP Port".to_string(),
                            field_type: ConfigFieldType::Number,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_username".to_string(),
                            label: "SMTP Username".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_password".to_string(),
                            label: "SMTP Password".to_string(),
                            field_type: ConfigFieldType::Password,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_from_email".to_string(),
                            label: "From Email".to_string(),
                            field_type: ConfigFieldType::Email,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_tls".to_string(),
                            label: "TLS".to_string(),
                            field_type: ConfigFieldType::Boolean,
                            required: false,
                            options: Vec::new(),
                        },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "postmark".to_string(),
                name: "Postmark".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/postmarkapp.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://postmarkapp.com/developer".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "server_token".to_string(),
                        label: "Server Token".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_email".to_string(),
                        label: "From Email".to_string(),
                        field_type: ConfigFieldType::Email,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_name".to_string(),
                        label: "From Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField {
                            key: "smtp_host".to_string(),
                            label: "SMTP Host".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_port".to_string(),
                            label: "SMTP Port".to_string(),
                            field_type: ConfigFieldType::Number,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_username".to_string(),
                            label: "SMTP Username".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_password".to_string(),
                            label: "SMTP Password".to_string(),
                            field_type: ConfigFieldType::Password,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_from_email".to_string(),
                            label: "From Email".to_string(),
                            field_type: ConfigFieldType::Email,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_tls".to_string(),
                            label: "TLS".to_string(),
                            field_type: ConfigFieldType::Boolean,
                            required: false,
                            options: Vec::new(),
                        },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "mailgun".to_string(),
                name: "Mailgun".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/mailgun.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://documentation.mailgun.com/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "domain".to_string(),
                        label: "Domain".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_email".to_string(),
                        label: "From Email".to_string(),
                        field_type: ConfigFieldType::Email,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_name".to_string(),
                        label: "From Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField {
                            key: "smtp_host".to_string(),
                            label: "SMTP Host".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_port".to_string(),
                            label: "SMTP Port".to_string(),
                            field_type: ConfigFieldType::Number,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_username".to_string(),
                            label: "SMTP Username".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_password".to_string(),
                            label: "SMTP Password".to_string(),
                            field_type: ConfigFieldType::Password,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_from_email".to_string(),
                            label: "From Email".to_string(),
                            field_type: ConfigFieldType::Email,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_tls".to_string(),
                            label: "TLS".to_string(),
                            field_type: ConfigFieldType::Boolean,
                            required: false,
                            options: Vec::new(),
                        },
                    ],
                }),
            },
            ProviderDefinition {
                provider_id: "brevo".to_string(),
                name: "Brevo".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/brevo.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://developers.brevo.com/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_email".to_string(),
                        label: "From Email".to_string(),
                        field_type: ConfigFieldType::Email,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_name".to_string(),
                        label: "From Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: Some(SmtpFallbackConfig {
                    fields: vec![
                        ConfigField {
                            key: "smtp_host".to_string(),
                            label: "SMTP Host".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_port".to_string(),
                            label: "SMTP Port".to_string(),
                            field_type: ConfigFieldType::Number,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_username".to_string(),
                            label: "SMTP Username".to_string(),
                            field_type: ConfigFieldType::Text,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_password".to_string(),
                            label: "SMTP Password".to_string(),
                            field_type: ConfigFieldType::Password,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_from_email".to_string(),
                            label: "From Email".to_string(),
                            field_type: ConfigFieldType::Email,
                            required: true,
                            options: Vec::new(),
                        },
                        ConfigField {
                            key: "smtp_tls".to_string(),
                            label: "TLS".to_string(),
                            field_type: ConfigFieldType::Boolean,
                            required: false,
                            options: Vec::new(),
                        },
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
                icon_url: Some(
                    "https://cdn.brandfetch.io/twilio.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://www.twilio.com/docs".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "account_sid".to_string(),
                        label: "Account SID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "auth_token".to_string(),
                        label: "Auth Token".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_number".to_string(),
                        label: "From Number".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "termii".to_string(),
                name: "Termii".to_string(),
                scope: ProviderScope::Regional,
                primary_regions: vec!["NG".to_string(), "GH".to_string(), "KE".to_string()],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/termii.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://developer.termii.com/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "sender_id".to_string(),
                        label: "Sender ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "base_url".to_string(),
                        label: "Base URL".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "africas_talking".to_string(),
                name: "Africa's Talking".to_string(),
                scope: ProviderScope::Regional,
                primary_regions: vec![
                    "NG".to_string(),
                    "GH".to_string(),
                    "KE".to_string(),
                    "UG".to_string(),
                ],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/africastalking.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://docs.africastalking.com/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "username".to_string(),
                        label: "Username".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "sender_id".to_string(),
                        label: "Sender ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
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
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "sender_id".to_string(),
                        label: "Sender ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
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
                    ConfigField {
                        key: "username".to_string(),
                        label: "Account Username (email)".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "sender_id".to_string(),
                        label: "Sender ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "infobip".to_string(),
                name: "Infobip".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/infobip.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://www.infobip.com/docs".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "base_url".to_string(),
                        label: "Base URL".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "sender_id".to_string(),
                        label: "Sender ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "vonage".to_string(),
                name: "Vonage".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/vonage.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://developer.nexmo.com/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "api_secret".to_string(),
                        label: "API Secret".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "from_number".to_string(),
                        label: "From Number".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "bird".to_string(),
                name: "Bird".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/bird.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://bird.com/developer".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "api_key".to_string(),
                        label: "API Key".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "base_url".to_string(),
                        label: "Base URL".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "sender_id".to_string(),
                        label: "Sender ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
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
                icon_url: Some(
                    "https://cdn.brandfetch.io/slack.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://api.slack.com/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "type".to_string(),
                        label: "Auth Type".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "webhook_url".to_string(),
                        label: "Webhook URL".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "bot_token".to_string(),
                        label: "Bot Token".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: false,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "channel_id".to_string(),
                        label: "Channel ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "telegram".to_string(),
                name: "Telegram".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/telegram.org/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://core.telegram.org/bots/api".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "bot_token".to_string(),
                        label: "Bot Token".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "chat_id".to_string(),
                        label: "Chat ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "whatsapp_business".to_string(),
                name: "WhatsApp Business".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/whatsapp.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://developers.facebook.com/docs/whatsapp/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "phone_number_id".to_string(),
                        label: "Phone Number ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "access_token".to_string(),
                        label: "Access Token".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "api_version".to_string(),
                        label: "API Version".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "discord".to_string(),
                name: "Discord".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/discord.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://discord.com/developers/docs".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "type".to_string(),
                        label: "Auth Type".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "webhook_url".to_string(),
                        label: "Webhook URL".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "bot_token".to_string(),
                        label: "Bot Token".to_string(),
                        field_type: ConfigFieldType::Password,
                        required: false,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "channel_id".to_string(),
                        label: "Channel ID".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
                ],
                smtp_fallback: None,
            },
            ProviderDefinition {
                provider_id: "ms_teams".to_string(),
                name: "Microsoft Teams".to_string(),
                scope: ProviderScope::Global,
                primary_regions: vec![],
                platforms: vec![],
                icon_url: Some(
                    "https://cdn.brandfetch.io/microsoft.com/w/512/h/512/theme/dark/icon.jpeg"
                        .to_string(),
                ),
                docs_url: Some("https://learn.microsoft.com/microsoftteams/".to_string()),
                config_fields: vec![
                    ConfigField {
                        key: "webhook_url".to_string(),
                        label: "Webhook URL".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: true,
                        options: Vec::new(),
                    },
                    ConfigField {
                        key: "channel_name".to_string(),
                        label: "Channel Name".to_string(),
                        field_type: ConfigFieldType::Text,
                        required: false,
                        options: Vec::new(),
                    },
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
            config_fields: provider
                .config_fields
                .into_iter()
                .map(|f| f.into())
                .collect(),
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
                ConfigFieldType::Select => "select".to_string(),
                ConfigFieldType::Hidden => "hidden".to_string(),
            },
            required: field.required,
            options: field.options,
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

#[cfg(test)]
mod registry_tests {
    use super::*;

    #[test]
    fn registry_sections_cover_every_family_with_other_last() {
        let registry = build_registry();
        assert_eq!(registry.version, "1.5.0");

        let ids: Vec<&str> = registry
            .channels
            .iter()
            .map(|c| c.channel_id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "email", "sms", "chat", "android", "apple", "web", "window", "other"
            ]
        );
    }

    #[test]
    fn device_sections_list_only_the_destination_natives() {
        // The cross-platform wrappers (OneSignal, Pushy) left the device
        // families — each section pins to exactly its natives now.
        let registry = build_registry();
        for (channel_id, ids) in [
            ("android", vec!["fcm"]),
            ("apple", vec!["apns"]),
            ("web", vec!["web_push"]),
            ("window", vec!["wns"]),
        ] {
            let channel = registry
                .channels
                .iter()
                .find(|c| c.channel_id == channel_id)
                .unwrap_or_else(|| panic!("device section {channel_id} missing"));
            let got: Vec<&str> = channel
                .providers
                .iter()
                .map(|p| p.provider_id.as_str())
                .collect();
            assert_eq!(got, ids, "device section {channel_id} drifted");
        }
    }

    #[test]
    fn other_section_carries_the_relay_and_the_gateway() {
        let registry = build_registry();
        let gateway = registry
            .channels
            .iter()
            .find(|c| c.channel_id == "other")
            .expect("other section")
            .providers
            .iter()
            .find(|p| p.provider_id == "gateway")
            .expect("gateway card in the other section");

        let mode = gateway
            .config_fields
            .iter()
            .find(|f| f.key == "mode")
            .expect("mode field");
        assert!(matches!(mode.field_type, ConfigFieldType::Select));
        assert_eq!(mode.options, vec!["polling", "https"]);

        // The static route: both halves generate at connect — the topic is
        // hidden (recipe-only, no form render) and the secret stays
        // prefilled-and-regenerable on the form.
        let topic = gateway
            .config_fields
            .iter()
            .find(|f| f.key == "topic")
            .expect("topic field");
        assert!(matches!(topic.field_type, ConfigFieldType::Hidden));
        let secret = gateway
            .config_fields
            .iter()
            .find(|f| f.key == "secret_key")
            .expect("secret field");
        assert!(matches!(secret.field_type, ConfigFieldType::Text));
        assert!(secret.required);

        // Only select fields carry options — text/password stay bare.
        for field in &gateway.config_fields {
            if !matches!(field.field_type, ConfigFieldType::Select) {
                assert!(field.options.is_empty(), "{} leaked options", field.key);
            }
        }
    }

    #[test]
    fn other_section_derives_from_the_channel_catalog() {
        let registry = build_registry();
        let other = registry
            .channels
            .iter()
            .find(|c| c.channel_id == "other")
            .expect("other section built from other_channel::catalog");
        assert_eq!(other.channel_name, "Other");

        let ids: Vec<&str> = other
            .providers
            .iter()
            .map(|p| p.provider_id.as_str())
            .collect();
        // The cross-platform wrappers lead (onesignal, pushy — one key
        // serves phone families, pushy adds web), then the two
        // direction flavors: we post (relay), they poll (gateway).
        // The gateway moved in from the old linux family — its route is
        // `/gateway/{topic}` now.
        assert_eq!(ids, vec!["onesignal", "pushy", "relay", "gateway"]);

        // The onesignal card serves both phone families from one
        // connect — the whole point of the other channel's wrapper.
        let onesignal = other
            .providers
            .iter()
            .find(|p| p.provider_id == "onesignal")
            .expect("onesignal card");
        assert_eq!(onesignal.platforms, vec!["android", "apple"]);
        assert_eq!(onesignal.name, "OneSignal (Android + Apple)");
        let one_keys: Vec<&str> = onesignal
            .config_fields
            .iter()
            .map(|f| f.key.as_str())
            .collect();
        assert_eq!(one_keys, vec!["app_id", "api_key", "rest_api_key"]);

        // The relay recipe carries everything the webhook family's
        // provider renders; headers stay optional.
        let relay = other
            .providers
            .iter()
            .find(|p| p.provider_id == "relay")
            .expect("relay card");
        let relay_required: Vec<(&str, bool)> = relay
            .config_fields
            .iter()
            .map(|f| (f.key.as_str(), f.required))
            .collect();
        assert_eq!(
            relay_required,
            vec![
                ("target_url", true),
                ("method", true),
                ("headers", false),
                ("payload_template", true),
            ]
        );
    }
}
