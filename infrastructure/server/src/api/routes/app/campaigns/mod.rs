//! Project campaigns (`/app/campaigns`): one channel × one audience ×
//! one message × one timing. Flow-less doctrinely — delivery resolves
//! per-row against the project's connected providers for the channel.

pub mod dto;
pub mod handlers;
pub mod routes;
