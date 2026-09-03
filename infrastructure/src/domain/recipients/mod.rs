//! The recipients slice: a brand's end-users.
//!
//! A recipient is one of the brand's own users — the people/groups the brand
//! wants to notify — distinct from the platform's auth users (brands). Each
//! recipient is scoped to a single project and carries:
//! - `id`      : our own internal ULID,
//! - `user_id` : the brand's in-house targeting key (unique within the project),
//! - `name`    : display name,
//! - `contacts`: a flexible JSON blob of contact detail (email, phone,
//!   device/push-notification ids, ...).

pub mod entities;
pub mod services;

pub use entities::Recipient;
pub use services::RecipientService;
