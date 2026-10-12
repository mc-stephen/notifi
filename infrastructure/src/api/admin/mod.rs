//! Platform administration (`/admin`). New admin features mount
//! inside [`admin_router`]. User dashboard backend lives in
//! [`crate::api::app`].

pub mod billing;
pub mod dto;
pub mod handlers;
pub mod logs;
pub mod notifications;
pub mod projects;
pub mod routes;
pub mod support;
pub mod users;

use axum::Router;

use crate::api::state::AppState;

/// The `/admin` router: platform administration (accounts, users,
/// support). Mounted separately from [`crate::api::app::app_router`]
/// per the API layout.
pub fn admin_router(state: &AppState) -> Router<AppState> {
    let mut admin_routes = routes::router();
    // Admin routes need the admin service (for bootstrap, login, TOTP).
    if let Some(admin_service) = state.admin.clone() {
        admin_routes = admin_routes.layer(axum::Extension(admin_service));
    }
    // Admin routes also need auth service for CurrentUser extractor (fallback).
    if let Some(auth_service) = state.auth.clone() {
        admin_routes = admin_routes.layer(axum::Extension(auth_service));
    }
    // Admin support routes need the ticket service (absent -> 503).
    if let Some(ticket_service) = state.tickets.clone() {
        admin_routes = admin_routes.layer(axum::Extension(ticket_service));
    }
    // Admin user routes need the admin-users service (absent -> 503).
    if let Some(users_service) = state.admin_users.clone() {
        admin_routes = admin_routes.layer(axum::Extension(users_service));
    }
    // Admin project routes need the admin-projects service (absent -> 503).
    if let Some(projects_service) = state.admin_projects.clone() {
        admin_routes = admin_routes.layer(axum::Extension(projects_service));
    }
    // Admin notification routes need the service (absent -> 503).
    if let Some(notifications_service) = state.admin_notifications.clone() {
        admin_routes = admin_routes.layer(axum::Extension(notifications_service));
    }
    // Admin log routes need the audit service (absent -> 503).
    if let Some(audit_service) = state.audit.clone() {
        admin_routes = admin_routes.layer(axum::Extension(audit_service));
    }
    // Admin billing routes need the billing service (absent -> 503).
    if let Some(billing_service) = state.billing.clone() {
        admin_routes = admin_routes.layer(axum::Extension(billing_service));
    }
    admin_routes
}
