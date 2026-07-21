mod config;

pub use config::WhatsAppConfig;

use reqwest::Client;
use serde::Serialize;

pub struct WhatsAppTemplateMessage {
    pub to: String,
    pub template_name: String,
    pub language_code: String,
    pub variables: Vec<String>,
}

#[derive(Serialize)]
struct Parameter {
    #[serde(rename = "type")]
    param_type: String,
    text: String,
}

#[derive(Serialize)]
struct Component {
    #[serde(rename = "type")]
    comp_type: String,
    parameters: Vec<Parameter>,
}

#[derive(Serialize)]
struct Template {
    name: String,
    language: Language,
    components: Vec<Component>,
}

#[derive(Serialize)]
struct Language {
    code: String,
}

#[derive(Serialize)]
struct WhatsAppPayload {
    messaging_product: String,
    to: String,
    #[serde(rename = "type")]
    msg_type: String,
    template: Template,
}

pub struct WhatsAppProvider {
    config: WhatsAppConfig,
    client: Client,
}

impl WhatsAppProvider {
    pub fn new(config: WhatsAppConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    pub async fn send_template_message(&self, msg: &WhatsAppTemplateMessage) -> Result<(), String> {
        let url = format!(
            "https://graph.facebook.com/v18.0/{}/messages",
            self.config.phone_number_id
        );

        let parameters = msg.variables
            .iter()
            .map(|v| Parameter {
                param_type: "text".to_string(),
                text: v.clone(),
            })
            .collect();

        let payload = WhatsAppPayload {
            messaging_product: "whatsapp".to_string(),
            to: msg.to.clone(),
            msg_type: "template".to_string(),
            template: Template {
                name: msg.template_name.clone(),
                language: Language { code: msg.language_code.clone() },
                components: vec![Component {
                    comp_type: "body".to_string(),
                    parameters,
                }],
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.access_token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("WhatsApp API request failed: {e}"))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let err = response.text().await.unwrap_or_default();
            Err(format!("WhatsApp API error: {err}"))
        }
    }
}
