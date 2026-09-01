//! The projects slice: project listings and the environment gate.
//!
//! The user account IS the workspace — projects are owned by their creator
//! and shared via per-project membership. The project-level `environment`
//! gate decides which env-scoped credentials are allowed right now
//! (development → dev only; production → both).

pub mod entities;
pub mod services;

pub use entities::Project;
pub use services::ProjectService;
