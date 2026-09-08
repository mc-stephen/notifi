//! The admin slice: platform administrators (separate from platform customers).

pub mod entities;
pub mod services;
pub mod users;

pub use entities::{
    AdminPasswordResetToken, AdminPasswordResetTokenId, AdminSession, AdminSessionId, AdminUser,
    AdminUserId,
};
pub use services::AdminService;
pub use users::{AdminUsersService, UserDetail};
