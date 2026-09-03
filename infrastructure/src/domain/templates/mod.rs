//! The templates slice: reusable, per-channel message definitions.
//!
//! A template is a named message definition owned by a project. It houses
//! multiple per-channel content variants in a flexible `content` JSON blob
//! (e.g. email → `subject`/`html`/`text`, sms → `sms`, push/in-app →
//! `push: {title, body}`), a primary `channel` hint for the dashboard editor,
//! and a linked list of `attachments` (metadata + URL).

pub mod entities;
pub mod services;

pub use entities::{Attachment, Template};
pub use services::TemplateService;
