//! Unified auth error model mapped to RFC 9457 problem documents.
//!
//! Messages match the dashboard contract (`app/dashboard/app/auth/
//! API_CONTRACT.md`); the verify-email page branches on detail containing
//! the word "expired", so that wording is load-bearing.

use notifi_core::error::{ApiError, IntoApiError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Wrong email or password (login).
    InvalidCredentials,
    /// Signup with an email that already has an account.
    EmailAlreadyExists,
    /// Input failed validation (email shape, password policy).
    Validation(String),
    /// One-time token unknown or already used.
    TokenInvalid(String),
    /// One-time token past its expiry.
    TokenExpired(String),
    /// Missing/invalid session cookie on a protected endpoint.
    Unauthorized,
    /// Auth routes are mounted but the service is unavailable (no database).
    NotConfigured,
    /// Endpoint exists but its real implementation hasn't landed yet.
    NotImplemented(String),
    /// Persistence failure.
    Storage(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCredentials => write!(f, "invalid credentials"),
            Self::EmailAlreadyExists => write!(f, "email already exists"),
            Self::Validation(m) => write!(f, "validation failed: {m}"),
            Self::TokenInvalid(m) => write!(f, "invalid token: {m}"),
            Self::TokenExpired(m) => write!(f, "expired token: {m}"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::NotConfigured => write!(f, "auth service not configured"),
            Self::NotImplemented(m) => write!(f, "not implemented: {m}"),
            Self::Storage(m) => write!(f, "storage failure: {m}"),
        }
    }
}

impl std::error::Error for AuthError {}

impl IntoApiError for AuthError {
    fn to_api_error(&self) -> ApiError {
        match self {
            Self::InvalidCredentials => ApiError::new(
                401,
                "about:blank",
                "Unauthorized",
                "Invalid email or password.",
            ),
            Self::EmailAlreadyExists => ApiError::new(
                409,
                "about:blank",
                "Conflict",
                "An account with this email already exists.",
            ),
            Self::Validation(detail) => ApiError::bad_request(detail.clone()),
            Self::TokenInvalid(detail) => ApiError::bad_request(detail.clone()),
            Self::TokenExpired(detail) => ApiError::new(410, "about:blank", "Gone", detail.clone()),
            Self::Unauthorized => ApiError::new(
                401,
                "about:blank",
                "Unauthorized",
                "Authentication required.",
            ),
            Self::NotConfigured => ApiError::new(
                503,
                "about:blank",
                "Service Unavailable",
                "auth is unavailable because the database is not configured.",
            ),
            Self::NotImplemented(detail) => {
                ApiError::new(501, "about:blank", "Not Implemented", detail.clone())
            }
            Self::Storage(_) => {
                ApiError::internal("something went wrong on our side; try again later.")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_error_messages_and_statuses() {
        assert_eq!(AuthError::InvalidCredentials.to_api_error().status, 401);
        assert_eq!(
            AuthError::EmailAlreadyExists.to_api_error().detail,
            "An account with this email already exists."
        );
        // the verify-email page branches on this word — do not change it
        let expired = AuthError::TokenExpired("This verification link has expired.".to_string());
        let problem = expired.to_api_error();
        assert_eq!(problem.status, 410);
        assert!(problem.detail.contains("expired"));
    }
}
