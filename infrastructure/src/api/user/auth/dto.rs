//! Request/response shapes for the auth API.
//!
//! Field names follow the dashboard contract (`app/dashboard/app/auth/
//! API_CONTRACT.md`) — camelCase on the wire.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::auth::entities::User;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub remember_me: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignupRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResendVerificationRequest {
    pub email: String,
}

/// Body of `POST /v1/auth/onboarding/complete` — the first project
/// collected by the dashboard flow.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteOnboardingRequest {
    pub project: OnboardingProjectDto,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingProjectDto {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

impl CompleteOnboardingRequest {
    /// Flattens into the storage-level input.
    pub fn into_input(self) -> crate::ports::auth_store::OnboardingInput {
        crate::ports::auth_store::OnboardingInput {
            project_name: self.project.name,
            project_description: self.project.description,
        }
    }
}

/// The user as seen by the dashboard — never includes the password hash.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(rename = "avatar")]
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

impl From<&User> for UserDto {
    fn from(user: &User) -> Self {
        Self {
            id: user.id.to_string(),
            name: user.name.clone(),
            email: user.email.as_str().to_string(),
            avatar_url: user.avatar_url.clone(),
            email_verified: user.email_verified(),
            created_at: user.created_at,
            last_login_at: user.last_login_at,
        }
    }
}

/// Session payload returned by login and signup (`{user, token, expiresAt,
/// onboardingCompleted}`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub user: UserDto,
    /// The raw session cookie value; the same value is set as an httpOnly
    /// cookie. Shown once.
    pub token: String,
    pub expires_at: DateTime<Utc>,
    /// Whether the account already owns an org + project (can skip
    /// onboarding).
    pub onboarding_completed: bool,
}
