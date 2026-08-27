//! reqwest-backed [`OAuthIdentityProvider`] for GitHub and Google.
//!
//! Hand-rolled against the plain OAuth2/OIDC endpoints (both providers are
//! well-documented and stable) instead of pulling a protocol crate: authorize
//! URL building, PKCE S256, token exchange, and profile resolution — all with
//! client secrets that never leave the server.
//!
//! Google profile data is read from the `id_token` (JWT payload) returned by
//! the token endpoint itself, not from the userinfo HTTP endpoint — fewer
//! round trips and no dependence on hosts that some networks block. Signature
//! verification is intentionally skipped: the id_token arrives directly from
//! Google over an authenticated TLS channel (we hold the client secret), the
//! same trust boundary as the access token itself.

use std::sync::Arc;
use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::domain::auth::value_objects::{new_token, urlsafe_b64};
use crate::ports::auth_store::BoxFut;
use crate::ports::oauth::{
    AuthorizeStart, OAuthError, OAuthIdentityProvider, OAuthProfile,
};

const GITHUB_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_URL: &str = "https://api.github.com/user";
const GITHUB_EMAILS_URL: &str = "https://api.github.com/user/emails";

const GOOGLE_AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

const GITHUB_NO_EMAIL_HINT: &str =
    "GitHub did not return a verified email — add a public email at github.com/settings/emails";

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
        // Static builder options cannot fail; fall back to a default client
        // rather than panicking if reqwest ever surprises us.
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(25))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            github,
            google,
            api_base_url,
        }
    }

    fn creds(&self, provider: &str) -> Result<&ProviderCredentials, OAuthError> {
        match provider {
            "github" => self
                .github
                .as_ref()
                .ok_or_else(|| OAuthError::NotConfigured(provider.to_string())),
            "google" => self
                .google
                .as_ref()
                .ok_or_else(|| OAuthError::NotConfigured(provider.to_string())),
            other => Err(OAuthError::Flow(format!("unsupported provider '{other}'"))),
        }
    }

    fn redirect_uri(&self, provider: &str) -> String {
        format!(
            "{}/v1/auth/oauth/{provider}/callback",
            self.api_base_url.trim_end_matches('/')
        )
    }

    /// Sends a read-only GET once, retrying a single time on transport
    /// errors (safe for idempotent requests; never used for POSTs).
    async fn get_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, OAuthError> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let cloned = request
                .try_clone()
                .ok_or_else(|| OAuthError::Flow("request is not cloneable".to_string()))?;
            match cloned.send().await {
                Ok(response) => return Ok(response),
                Err(err) if attempt < 2 => {
                    tracing::warn!(attempt, error = %err, "transient transport error; retrying once");
                }
                Err(err) => return Err(OAuthError::Flow(err.to_string())),
            }
        }
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
        let profile_request = self
            .http
            .get(GITHUB_USER_URL)
            .bearer_auth(&token.access_token)
            .header("User-Agent", "notifi-dashboard");
        let user: serde_json::Value = self
            .get_with_retry(profile_request)
            .await?
            .error_for_status()
            .map_err(|e| OAuthError::Flow(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::Flow(e.to_string()))?;

        // Email resolution order:
        //   1. /user/emails list (primary + verified, else any verified)
        //   2. the profile's public `email` field (GitHub only publishes
        //      addresses the user verified)
        //   3. actionable error
        //
        // NOTE: /user/emails answers 404 when the account exposes no public
        // email address — treat that as "no list", not a failure.
        #[derive(Deserialize)]
        struct GitHubEmail {
            email: String,
            primary: bool,
            verified: bool,
        }
        let emails_request = self
            .http
            .get(GITHUB_EMAILS_URL)
            .bearer_auth(&token.access_token)
            .header("User-Agent", "notifi-dashboard");
        let emails: Option<Vec<GitHubEmail>> = match self.get_with_retry(emails_request).await {
            Ok(response) => {
                if response.status() == StatusCode::NOT_FOUND {
                    None
                } else {
                    Some(
                        response
                            .error_for_status()
                            .map_err(|e| OAuthError::Flow(e.to_string()))?
                            .json()
                            .await
                            .map_err(|e| OAuthError::Flow(e.to_string()))?,
                    )
                }
            }
            Err(err) => return Err(err),
        };

        let profile_email = user["email"].as_str().filter(|s| !s.is_empty());

        let email = emails
            .as_ref()
            .and_then(|list| {
                list.iter()
                    .find(|e| e.primary && e.verified)
                    .or_else(|| list.iter().find(|e| e.verified))
                    .map(|e| e.email.clone())
            })
            .or_else(|| profile_email.map(str::to_string))
            .ok_or_else(|| OAuthError::Flow(GITHUB_NO_EMAIL_HINT.to_string()))?;

        Ok(OAuthProfile {
            subject: user["id"].to_string(),
            email,
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
            /// Present because we request the `openid` scope.
            #[serde(default)]
            id_token: Option<String>,
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

        // Primary path: the id_token already carries the full profile.
        if let Some(profile) = token.id_token.as_deref().and_then(decode_id_token) {
            return Ok(profile);
        }

        // Fallback: the userinfo endpoint (kept for tokens issued without
        // the openid scope).
        #[derive(Deserialize)]
        struct UserInfo {
            sub: String,
            email: String,
            email_verified: bool,
            name: Option<String>,
            picture: Option<String>,
        }
        let userinfo_request = self
            .http
            .get(GOOGLE_USERINFO_URL)
            .bearer_auth(&token.access_token);
        let info: UserInfo = self
            .get_with_retry(userinfo_request)
            .await?
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

/// Parses a Google OIDC id_token (JWT) payload into an [`OAuthProfile`].
///
/// Signature verification is deliberately skipped — see the module docs for
/// why that is safe in this direct token-exchange context. Returns `None`
/// when the token is malformed so callers can fall back to userinfo.
fn decode_id_token(id_token: &str) -> Option<OAuthProfile> {
    let payload_part = id_token.split('.').nth(1)?;
    let bytes = urlsafe_b64_decode(payload_part).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;

    Some(OAuthProfile {
        subject: value.get("sub")?.as_str()?.to_string(),
        email: value.get("email")?.as_str()?.to_string(),
        email_verified: value.get("email_verified")?.as_bool().unwrap_or(false),
        name: value.get("name").and_then(|v| v.as_str()).map(str::to_string),
        avatar_url: value
            .get("picture")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

/// URL-safe base64 without padding → raw bytes (inverse of
/// [`crate::domain::auth::value_objects::urlsafe_b64`]).
fn urlsafe_b64_decode(input: &str) -> Result<Vec<u8>, OAuthError> {
    const CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for byte in input.bytes() {
        if byte == b'=' {
            break;
        }
        let value = CHARS
            .iter()
            .position(|&c| c == byte)
            .ok_or_else(|| OAuthError::Flow("invalid base64url input".to_string()))?
            as u32;
        acc = (acc << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

/// Convenience constructor used by the composition root.
pub fn http_oauth_provider(
    github: Option<ProviderCredentials>,
    google: Option<ProviderCredentials>,
    api_base_url: String,
) -> Arc<dyn OAuthIdentityProvider> {
    Arc::new(HttpOAuthIdentityProvider::new(github, google, api_base_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_google_id_token_into_a_profile() {
        let payload = r#"{"sub":"42","email":"Ada@Example.COM","email_verified":true,"name":"Ada Lovelace","picture":"https://example.com/p.png","aud":"ignored","iss":"ignored"}"#;
        let body = urlsafe_b64(payload.as_bytes());
        // header.signature.payload — only the middle segment matters
        let id_token = format!("eyJhbGciOiJSUzI1NiJ9.{body}.c2lnbmF0dXJl");

        let profile = decode_id_token(&id_token).expect("valid id_token");

        assert_eq!(profile.subject, "42");
        assert_eq!(profile.email, "Ada@Example.COM");
        assert!(profile.email_verified);
        assert_eq!(profile.name.as_deref(), Some("Ada Lovelace"));
        assert_eq!(profile.avatar_url.as_deref(), Some("https://example.com/p.png"));
    }

    #[test]
    fn rejects_malformed_id_tokens() {
        assert!(decode_id_token("not-a-jwt").is_none());
        assert!(decode_id_token("one.two").is_none());
        assert!(decode_id_token(".not-base64.").is_none());
        assert!(decode_id_token(".e30.").is_none()); // "{}" — missing fields
    }

    #[test]
    fn base64url_roundtrip() {
        for sample in [b"".as_slice(), b"f", b"fo", b"foo", b"foobar", b"hello world"] {
            let encoded = urlsafe_b64(sample);
            let decoded = urlsafe_b64_decode(&encoded).expect("valid encoding");
            assert_eq!(decoded, sample);
        }
    }
}
