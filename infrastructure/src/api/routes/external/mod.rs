//! External API surface — the product API customers integrate with.
//!
//! Served under `/v1`. v0 authenticates with the dashboard session
//! (external API keys land in M6) and serves instant notification sends
//! through each project's default flow. Deliberately separate from the
//! user surface ([`crate::api::routes::app`]) so product and dashboard
//! logic never share routes or middleware.

pub mod dto;
pub mod handlers;
mod routes;

pub use routes::router;
