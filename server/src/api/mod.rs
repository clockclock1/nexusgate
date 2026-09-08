mod auth;
mod handlers;
mod ws;

use crate::state::AppState;
use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/health", get(|| async { "ok" }));

    let protected = Router::new()
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me))
        .route("/api/dashboard", get(handlers::dashboard))
        .route("/api/metrics/dashboard", get(handlers::dashboard))
        .route("/api/metrics/traffic", get(handlers::traffic))
        .route("/api/metrics/connections", get(handlers::connections))
        .route("/api/metrics/logs", get(handlers::logs))
        .route("/api/metrics/topology", get(handlers::topology))
        .route("/api/metrics/p2p", get(handlers::p2p_stats))
        .route("/api/server", get(handlers::server_info))
        .route("/api/server/info", get(handlers::server_info))
        .route("/api/server/config", get(handlers::server_config).put(handlers::update_server_config))
        .route("/api/server/restart", post(handlers::server_restart))
        .route("/api/nodes", get(handlers::list_nodes).post(handlers::create_node))
        .route(
            "/api/nodes/:id",
            get(handlers::get_node)
                .put(handlers::update_node)
                .delete(handlers::delete_node),
        )
        .route("/api/nodes/:id/token", post(handlers::regen_token))
        .route("/api/nodes/:id/services", get(handlers::node_services))
        .route("/api/nodes/:id/routes", get(handlers::node_routes))
        .route("/api/nodes/:id/connections", get(handlers::node_connections))
        .route("/api/nodes/:id/traffic", get(handlers::node_traffic))
        .route("/api/nodes/:id/logs", get(handlers::node_logs))
        .route(
            "/api/services",
            get(handlers::list_services).post(handlers::create_service),
        )
        .route(
            "/api/services/:id",
            get(handlers::get_service)
                .put(handlers::update_service)
                .delete(handlers::delete_service),
        )
        .route(
            "/api/routes",
            get(handlers::list_routes).post(handlers::create_route),
        )
        .route(
            "/api/routes/:id",
            get(handlers::get_route)
                .put(handlers::update_route)
                .delete(handlers::delete_route),
        )
        .route("/api/connections", get(handlers::connections))
        .route("/api/traffic", get(handlers::traffic))
        .route("/api/logs", get(handlers::logs))
        .route("/api/p2p", get(handlers::topology))
        .route("/api/settings", get(handlers::get_settings).put(handlers::update_settings))
        .route("/api/users", get(handlers::list_users).post(handlers::create_user))
        .route(
            "/api/users/:id",
            put(handlers::update_user).delete(handlers::delete_user),
        )
        .route("/ws/dashboard", get(ws::ws_dashboard))
        .route("/ws/logs", get(ws::ws_logs))
        .route("/ws/connections", get(ws::ws_connections))
        .route("/ws/traffic", get(ws::ws_traffic))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    public
        .merge(protected)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
