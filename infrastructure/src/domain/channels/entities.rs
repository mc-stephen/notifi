use serde::{Deserialize, Serialize};

/// The canonical registry of all supported channels and providers.
/// Loaded from static definitions — the compiler enforces structural correctness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRegistry {
    pub version: String,
    pub last_updated: String,
    pub channels: Vec<ChannelDefinition>,
}

/// A channel type (email, sms, push, chat).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelDefinition {
    pub channel_id: String,
    pub channel_name: String,
    pub providers: Vec<ProviderDefinition>,
}

/// A single provider within a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDefinition {
    pub provider_id: String,
    pub name: String,
    pub scope: ProviderScope,
    #[serde(default)]
    pub primary_regions: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    /// URL to the provider's logo/icon (optional).
    #[serde(default)]
    pub icon_url: Option<String>,
    /// Config fields for the primary API connection.
    #[serde(default)]
    pub config_fields: Vec<ConfigField>,
    /// Optional SMTP fallback fields for email providers.
    #[serde(default)]
    pub smtp_fallback: Option<SmtpFallbackConfig>,
}

/// Whether a provider operates globally or is region-focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderScope {
    Global,
    Regional,
}

/// A single configuration field for a provider's connect form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: ConfigFieldType,
    #[serde(default)]
    pub required: bool,
}

/// The type of input a config field expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFieldType {
    Text,
    Password,
    Email,
    Number,
    Boolean,
}

/// SMTP fallback config — the same shape for every email provider that supports it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpFallbackConfig {
    pub fields: Vec<ConfigField>,
}

/// A project-level provider configuration (stored in the database).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProviderConfig {
    pub id: String,
    pub project_id: String,
    pub channel_id: String,
    pub provider_id: String,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_fallback: Option<serde_json::Value>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Input for creating/updating a project provider config.
#[derive(Debug, Clone, Deserialize)]
pub struct ProjectProviderConfigInput {
    pub channel_id: String,
    pub provider_id: String,
    pub config: serde_json::Value,
    #[serde(default)]
    pub smtp_fallback: Option<serde_json::Value>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}
