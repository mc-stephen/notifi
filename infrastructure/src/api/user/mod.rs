//! User API surface — the dashboard backend, versioned under `/v1`.
//!
//! Versioning contract: breaking changes ship as `/v2` alongside `/v1`;
//! v1 never changes incompatibly. New dashboard features mount inside
//! [`v1_router`] (auth today; onboarding/organizations next).

pub mod auth;
pub mod channel_configs;
pub mod logs;
pub mod projects;
pub mod providers;
pub mod recipients;
pub mod templates;

use axum::Router;

use crate::api::state::AppState;

/// The `/v1` router: nests feature routers under their path segments.
pub fn v1_router(state: &AppState) -> Router<AppState> {
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

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/providers", provider_routes)
        .nest("/projects", project_routes)
        .nest("/projects/{project_id}/recipients", recipient_routes)
        .nest("/projects/{project_id}/templates", template_routes)
        .nest("/projects/{project_id}/channel-configs", channel_config_routes)
        .nest("/logs", log_routes)
}
