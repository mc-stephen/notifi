//! Project flow schedules (`/app/schedules`): timed runs of assigned
//! flows. The table never exposes channel/recipients columns, but the
//! payload rides along for the worker and the detail panel editor.

pub mod dto;
pub mod handlers;
pub mod routes;
