//! Flow schedules: run an assigned flow once at a time or every interval.
//!
//! A schedule owns its execution payload (channel/title/body/recipients)
//! so the worker can deliver without further input. Flows referenced by
//! any schedule are hidden from the `/flows` list — they are managed
//! here, though the builder stays reachable by id. Framework-free by rule.

pub mod entities;
pub mod services;

pub use entities::{Schedule, ScheduleKind};
pub use services::ScheduleService;
