//! HTTP route surfaces grouped under `routes/`.
//!
//! These are the audience-specific API surfaces — distinct from `middleware/`
//! which is cross-cutting plumbing (cors, request_id).

pub mod admin;
pub mod app;
pub mod external;
