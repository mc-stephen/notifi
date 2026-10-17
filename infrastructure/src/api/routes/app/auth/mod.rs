//! Auth HTTP surface (mounted at `/app/auth`).

pub mod dto;
pub mod handlers;
pub mod middleware;
pub mod routes;

pub use middleware::{ADMIN_SESSION_COOKIE, CurrentUser, Problem, SESSION_COOKIE};
