//! reqwest-backed [`OAuthIdentityProvider`] for GitHub and Google.
//!
//! Hand-rolled against the plain OAuth2/OIDC endpoints (both providers are
//! well-documented and stable) instead of pulling a protocol crate: authorize
//! URL building, PKCE S256, token exchange, and userinfo fetches — all with
//! client secrets that never leave the server.

use std::sync::Arc;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::ports::oauth::{
    AuthorizeStart, OAuthError, OAuthIdentityProvider, OAuthProfile,
};
use crate::ports::auth_store::BoxFut;
use crate::domain::auth::value_objects::{new_token, urlsafe_b64};

const GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";
const GITHUB_EMAILS_URL: &str = "https://api.github.com/user/emails";

const GOOGLE_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

/// Credentials for a single provider; `None` disables it.
#[derive(Debug, Clone)]
pub struct ProviderCredentials {
    pub client_id: String,
    pub client_secret: String,
}

pub struct HttpOAuthIdentityProvider {
    http: reqwest::Client,
    github: Option<ProviderCredentials>,
    google: Option<ProviderCredentials>,
    /// Public base URL of this API; callback URLs are derived from it.
    api_base_url: String,
}

impl HttpOAuthIdentityProvider {
    pub fn new(
        github: Option<ProviderCredentials>,
        google: Option<ProviderCredentials>,
        api_base_url: String,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            github,
            google,
            api_base_url,
        }
    }

    fn creds(&self, provider: &str) -> Result<&ProviderCredentials, OAuthError> {
        match provider {
            "github" => self.github.as_ref().ok_or_else(|| OAuthError::NotConfigured(provider.to_string())),
            "google" => self.google.as_ref().ok_or_else(|| OAuthError::NotConfigured(provider.to_string())),
            other => Err(OAuthError::Flow(format!("unsupported provider '{other}'"))),
        }
    }

    fn redirect_uri(&self, provider: &str) -> String {
        format!(
            "{}/v1/auth/oauth/{provider}/callback",
            self.api_base_url.trim_end_matches('/')
        )
    }

    async fn github_exchange(
        &self,
        creds: &ProviderCredentials,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthProfile, OAuthError> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }

        let token: TokenResponse = self
            .http
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", creds.client_id.as_str()),
                ("client_secret", creds.client_secret.as_str()),
                ("code", code),
                ("redirect_uri", redirect_uri),
            ])
            .send()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?;

        // GitHub requires a User-Agent on API calls.
        let user: serde_json::Value = self
            .http
            .get(GITHUB_USER_URL)
            .bearer_auth(&token.access_token)
            .header("User-Agent", "notifi-dashboard")
            .send()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?;

        #[derive(Deserialize)]
        struct GitHubEmail {
            email: String,
            primary: bool,
            verified: bool,
        }
        let emails: Vec<GitHubEmail> = self
            .http
            .get(GITHUB_EMAILS_URL)
            .bearer_auth(&token.access_token)
            .header("User-Agent", "notifi-dashboard")
            .send()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?;

        // Prefer the primary verified address; fall back to any verified one.
        let email = emails
            .iter()
            .find(|e| e.primary && e.verified)
            .or_else(|| emails.iter().find(|e| e.verified))
            .ok_or_else(|| {
                OAuthError::Flow("no verified email on the GitHub account".to_string())
            })?;

        Ok(OAuthProfile {
            subject: user["id"].to_string(),
            email: email.email.clone(),
            email_verified: true,
            name: user["name"]
                .as_str()
                .map(str::to_string)
                .or_else(|| user["login"].as_str().map(str::to_string)),
            avatar_url: user["avatar_url"].as_str().map(str::to_string),
        })
    }

    async fn google_exchange(
        &self,
        creds: &ProviderCredentials,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
    ) -> Result<OAuthProfile, OAuthError> {
        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }

        let token: TokenResponse = self
            .http
            .post(GOOGLE_TOKEN_URL)
            .form(&[
                ("client_id", creds.client_id.as_str()),
                ("client_secret", creds.client_secret.as_str()),
                ("code", code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri),
                ("code_verifier", pkce_verifier),
            ])
            .send()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?;

        #[derive(Deserialize)]
        struct UserInfo {
            sub: String,
            email: String,
            email_verified: bool,
            name: Option<String>,
            picture: Option<String>,
        }
        let info: UserInfo = self
            .http
            .get(GOOGLE_USERINFO_URL)
            .bearer_auth(&token.access_token)
            .send()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?;

        Ok(OAuthProfile {
            subject: info.sub,
            email: info.email,
            email_verified: info.email_verified,
            name: info.name,
            avatar_url: info.picture,
        })
    }
}

impl OAuthIdentityProvider for HttpOAuthIdentityProvider {
    fn authorize_url(
        &self,
        provider: &str,
        csrf_state: &str,
    ) -> Result<AuthorizeStart, OAuthError> {
        let creds = self.creds(provider)?;
        let redirect_uri = self.redirect_uri(provider);
        let redirect_uri = urlencoding::encode(&redirect_uri).into_owned();
        // Random high-entropy verifier; only its S256 challenge travels to
        // the consent screen.
        let pkce_verifier = new_token().0;

        let url = match provider {
            "github" => format!(
                "{GITHUB_AUTHORIZE_URL}?client_id={}&redirect_uri={redirect_uri}&state={csrf_state}&scope={}",
                urlencoding::encode(&creds.client_id),
                urlencoding::encode("read:user user:emails"),
            ),
            "google" => {
                let challenge = urlsafe_b64(&Sha256::digest(pkce_verifier.as_bytes()));
                format!(
                    "{GOOGLE_AUTHORIZE_URL}?client_id={}&redirect_uri={redirect_uri}&state={csrf_state}&response_type=code&scope={}&code_challenge={challenge}&code_challenge_method=S256",
                    urlencoding::encode(&creds.client_id),
                    urlencoding::encode("openid email profile"),
                )
            }
            other => return Err(OAuthError::Flow(format!("unsupported provider '{other}'"))),
        };

        Ok(AuthorizeStart { url, pkce_verifier })
    }

    fn exchange_code(
        &self,
        provider: &str,
        code: &str,
        pkce_verifier: &str,
    ) -> BoxFut<'_, Result<OAuthProfile, OAuthError>> {
        // Own everything the async block touches — the boxed future must
        // outlive the borrowed parameters.
        let provider = provider.to_string();
        let code = code.to_string();
        let pkce_verifier = pkce_verifier.to_string();
        Box::pin(async move {
            let creds = match self.creds(&provider) {
                Ok(creds) => creds.clone(),
                Err(e) => return Err(e),
            };
            let redirect_uri = self.redirect_uri(&provider);
            match provider.as_str() {
                "github" => self.github_exchange(&creds, &code, &redirect_uri).await,
                "google" => {
                    self.google_exchange(&creds, &code, &pkce_verifier, &redirect_uri)
                        .await
                }
                other => Err(OAuthError::Flow(format!("unsupported provider '{other}'"))),
            }
        })
    }
}

/// Convenience constructor used by the composition root.
pub fn http_oauth_provider(
    github: Option<ProviderCredentials>,
    google: Option<ProviderCredentials>,
    api_base_url: String,
) -> Arc<dyn OAuthIdentityProvider> {
    Arc::new(HttpOAuthIdentityProvider::new(github, google, api_base_url))
}
