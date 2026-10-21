//! The admin slice: platform administrators (separate from platform customers).

pub mod entities;
pub mod notifications;
pub mod projects;
pub mod services;
pub mod users;

pub use entities::{
    AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession, AdminSessionId, AdminStatus,
    AdminUser, AdminUserId,
};
pub use notifications::{AdminNotificationsService, Audience, BroadcastInput, BroadcastResult};
pub use projects::{AdminProjectsService, ProjectDetail};
pub use services::AdminService;
pub use users::{AdminUsersService, UserDetail};
