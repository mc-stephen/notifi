//! Auth HTTP surface (mounted at `/v1/auth`).

pub mod dto;
pub mod handlers;
pub mod middleware;
pub mod routes;

pub use middleware::{CurrentUser, Problem, SESSION_COOKIE};
