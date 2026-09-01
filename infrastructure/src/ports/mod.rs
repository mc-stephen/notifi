//! Ports — the traits the domain depends on and `infra` implements.
//!
//! Framework-free by rule: no axum/sqlx/reqwest types may appear here.

pub mod auth_store;
pub mod oauth;
pub mod projects_store;

pub use auth_store::{AuthStore, BoxFut, OnboardingInput, StoreError};
pub use oauth::{
    AuthorizeStart, OAuthError, OAuthIdentityProvider, OAuthProfile, OAuthRuntime,
};
pub use projects_store::{ProjectSummary, ProjectsStore};
