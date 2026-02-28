use axum::Router;
use axum::middleware;
use axum::routing::{delete, get, post};

use crate::handlers;
use crate::middleware::auth;
use crate::state::AppState;

pub fn build(state: AppState) -> Router {
    // Public routes (no auth)
    let public = Router::new().route("/system/health", get(handlers::system::health));

    // Protected routes (require Bearer token)
    let protected = Router::new()
        // Apps
        .route(
            "/apps",
            get(handlers::apps::list).post(handlers::apps::create),
        )
        .route(
            "/apps/{name}",
            get(handlers::apps::get)
                .put(handlers::apps::update)
                .delete(handlers::apps::delete),
        )
        // Container lifecycle
        .route("/apps/{name}/start", post(handlers::containers::start))
        .route("/apps/{name}/stop", post(handlers::containers::stop))
        .route("/apps/{name}/restart", post(handlers::containers::restart))
        .route("/apps/{name}/status", get(handlers::containers::status))
        .route("/apps/{name}/logs", get(handlers::containers::logs))
        // Deployments
        .route("/apps/{name}/deploy", post(handlers::deploy::trigger))
        .route("/apps/{name}/deployments", get(handlers::deployments::list))
        .route(
            "/apps/{name}/deployments/{id}",
            get(handlers::deployments::get),
        )
        .route(
            "/apps/{name}/deployments/{id}/rollback",
            post(handlers::deployments::rollback),
        )
        // Env vars
        .route(
            "/apps/{name}/env",
            get(handlers::env_vars::list).put(handlers::env_vars::bulk_set),
        )
        .route("/apps/{name}/env/{key}", delete(handlers::env_vars::delete))
        // System
        .route("/system/info", get(handlers::system::info))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_secret,
        ));

    // Git webhook (internal, also auth-protected)
    let hooks = Router::new()
        .route("/hooks/git/{app_name}", post(handlers::hooks::git_push))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_secret,
        ));

    Router::new()
        .nest("/api/v1", public.merge(protected))
        .merge(hooks)
        .with_state(state)
}
