//! Request/response shapes for the admin API.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyTotpRequest {
    pub code: String,
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
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAdminRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAdminStatusRequest {
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminAccountDto {
    pub id: String,
    pub name: String,
    pub email: String,
    pub is_super_admin: bool,
    pub status: String,
    pub totp_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<crate::domain::admin::entities::AdminUser> for AdminAccountDto {
    fn from(admin: crate::domain::admin::entities::AdminUser) -> Self {
        Self {
            id: admin.id.to_string(),
            name: admin.name,
            email: admin.email.to_string(),
            is_super_admin: admin.is_super_admin,
            status: admin.status.as_str().to_string(),
            totp_enabled: admin.totp_enabled,
            created_at: admin.created_at,
            last_login_at: admin.last_login_at,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminStatusResponse {
    pub admin_exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapResponse {
    pub user_id: String,
    pub totp_secret: String,
    pub totp_uri: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpSetupResponse {
    pub totp_secret: String,
    pub totp_uri: String,
}
