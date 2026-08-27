//! The auth slice: users, sessions, one-time tokens, OAuth identities.

pub mod entities;
pub mod errors;
pub mod services;
pub mod value_objects;

pub use entities::{AuthToken, AuthTokenId, Session, SessionId, TokenPurpose, User, UserId};
pub use errors::AuthError;
pub use services::AuthService;
pub use value_objects::{Email, hash_token, new_token, urlsafe_b64, validate_password};
