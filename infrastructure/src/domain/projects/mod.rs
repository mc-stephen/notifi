//! The projects slice: project listings and the active switch.
//!
//! The user account IS the workspace — projects are owned by their creator
//! and shared via per-project membership. The project-level `active` flag
//! is the kill switch for all of a project's delivery and external traffic.

pub mod entities;
pub mod services;

pub use entities::{Project, ProjectInvite, ProjectMember};
pub use services::{MEMBER_ROLES, ProjectMembersService, ProjectService};
