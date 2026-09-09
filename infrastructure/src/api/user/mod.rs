//! User dashboard backend (`/app`) and platform administration
//! (`/admin`). New dashboard features mount inside [`app_router`];
//! new admin features mount inside [`admin_router`].

pub mod admin;
pub mod auth;
pub mod channel_configs;
pub mod logs;
pub mod notifications;
pub mod projects;
pub mod providers;
pub mod recipients;
pub mod support;
pub mod templates;

use axum::Router;

use crate::api::state::AppState;

/// The `/app` router: user-dashboard backend (auth, projects, support,
/// notifications, logs). Nests feature routers under their path segments.
pub fn app_router(state: &AppState) -> Router<AppState> {
    // Auth routes pull their service from a scoped Extension; without it
    // (no database) they answer 503 problem documents.
    let auth_routes = match state.auth.clone() {
        Some(service) => auth::routes::router().layer(axum::Extension(service)),
        None => auth::routes::router(),
    };
    // The OAuth runtime is optional independently of the service; without
    // it (no provider credentials) the /oauth routes answer 503. Layered
    // conditionally so the extension carries the bare `Arc<OAuthRuntime>`
    // the handlers extract — NOT `Extension<Option<Arc<...>>>`, which would
    // silently never match.
    let auth_routes = match state.oauth.clone() {
        Some(runtime) => auth_routes.layer(axum::Extension(runtime)),
        None => auth_routes,
    };

    let mut project_routes = projects::routes::router();
    // Project routes require session auth via `CurrentUser`, which needs the
    // auth service extension.
    if let Some(auth_service) = state.auth.clone() {
        project_routes = project_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.projects.clone() {
        project_routes = project_routes.layer(axum::Extension(service));
    }

    let mut log_routes = logs::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        log_routes = log_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.audit.clone() {
        log_routes = log_routes.layer(axum::Extension(service));
    }

    let mut recipient_routes = recipients::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        recipient_routes = recipient_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.recipients.clone() {
        recipient_routes = recipient_routes.layer(axum::Extension(service));
    }

    let mut template_routes = templates::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        template_routes = template_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.templates.clone() {
        template_routes = template_routes.layer(axum::Extension(service));
    }

    let provider_routes = providers::routes::routes();

    let mut channel_config_routes = channel_configs::routes::routes();
    if let Some(auth_service) = state.auth.clone() {
        channel_config_routes = channel_config_routes.layer(axum::Extension(auth_service));
    }
    if let Some(store) = state.channel_providers.clone() {
        channel_config_routes = channel_config_routes.layer(axum::Extension(store));
    }
    if let Some(svc) = state.notifications.clone() {
        channel_config_routes = channel_config_routes.layer(axum::Extension(svc));
    }
    channel_config_routes = channel_config_routes.layer(axum::Extension(state.provider_tester.clone()));

    let mut support_routes = support::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        support_routes = support_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.tickets.clone() {
        support_routes = support_routes.layer(axum::Extension(service));
    }

    let mut notification_routes = notifications::routes::router();
    if let Some(auth_service) = state.auth.clone() {
        notification_routes = notification_routes.layer(axum::Extension(auth_service));
    }
    if let Some(service) = state.notifications.clone() {
        notification_routes = notification_routes.layer(axum::Extension(service));
    }

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/providers", provider_routes)
        .nest("/projects", project_routes)
        .nest("/projects/{project_id}/recipients", recipient_routes)
        .nest("/projects/{project_id}/templates", template_routes)
        .nest("/projects/{project_id}/channel-configs", channel_config_routes)
        .nest("/support", support_routes)
        .nest("/notifications", notification_routes)
        .nest("/logs", log_routes)
}

/// The `/admin` router: platform administration (accounts, users,
/// support). Mounted separately from [`app_router`] per the API layout.
pub fn admin_router(state: &AppState) -> Router<AppState> {
    let mut admin_routes = admin::routes::router();
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
    admin_routes
}
