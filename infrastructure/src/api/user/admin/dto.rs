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
