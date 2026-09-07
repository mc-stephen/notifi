//! The admin slice: platform administrators (separate from platform customers).

pub mod entities;
pub mod services;

pub use entities::{AdminSession, AdminSessionId, AdminUser, AdminUserId};
pub use services::AdminService;
