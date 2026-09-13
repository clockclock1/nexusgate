mod admin_api;
mod config;
mod hub;
mod proxy;
mod registry;
mod static_files;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::routing::{any, delete, get, put};
use axum::Router;
use clap::Parser;
use config::AdminConfig;
use hub::HubState;
use registry::ServerRegistry;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub http: reqwest::Client,
    pub registry: ServerRegistry,
    pub hub: Option<HubState>,
    /// Internal management mesh (preferred path for panel → server API).
    pub mesh: Option<p2p_overlay::OverlayMesh>,
}

#[derive(Parser, Debug)]
#[command(
    name = "p2p-admin",
    about = "NexusGate Admin Panel + Hub (SPA, API proxy, P2P/relay mesh)"
)]
struct Args {
    /// Config file path (default: admin.toml beside this executable)
    #[arg(short, long, default_value_t = AdminConfig::default_path().display().to_string())]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let args = Args::parse();
    let cfg = AdminConfig::load(&args.config)?;
    let addr = cfg.bind_addr()?;
    let servers_n = cfg.servers.len();
    let default_server = cfg.default_server.clone();

    let hub = if cfg.hub_enabled {
        let hub = HubState::new(cfg.hub_token.clone());
        let control_addr = cfg.hub_control_addr()?;
        let data_addr = cfg.hub_data_addr()?;
        let h1 = hub.clone();
        tokio::spawn(async move {
            if let Err(e) = hub::run_hub_control(h1, control_addr).await {
                tracing::error!(error = %e, "hub control exited");
            }
        });
        let h2 = hub.clone();
        tokio::spawn(async move {
            if let Err(e) = hub::run_hub_data(h2, data_addr).await {
                tracing::error!(error = %e, "hub data exited");
            }
        });
        tracing::info!(
            control = %control_addr,
            data = %data_addr,
            "admin hub enabled (servers/edges dial in; P2P first, relay fallback)"
        );
        Some(hub)
    } else {
        None
    };

    // Internal management mesh (equal peer with p2p-server).
    let mesh = if cfg.overlay.enabled {
        let mut o = cfg.overlay.clone();
        o.role = p2p_overlay::OverlayRole::Server;
        if o.node_id.trim().is_empty() {
            o.node_id = "admin".into();
        }
        if o.fixed_vip.is_none() {
            o.fixed_vip = Some("10.88.0.1".into());
        }
        match p2p_overlay::start_overlay(o).await {
            Ok(m) => {
                tracing::info!(vip = %m.vip(), "management mesh ready (admin)");
                Some(m)
            }
            Err(e) => {
                tracing::warn!(error = %e, "management mesh failed; Hub/HTTP fallback");
                None
            }
        }
    } else {
        None
    };

    let state = AppState {
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        registry: ServerRegistry::new(cfg, std::path::PathBuf::from(&args.config)),
        hub,
        mesh,
    };

    tracing::info!(
        %addr,
        servers = servers_n,
        ?default_server,
        assets = static_files::embedded_asset_count(),
        "starting p2p-admin"
    );

    let admin_routes = Router::new()
        .route(
            "/admin/api/servers",
            get(admin_api::list_servers).post(admin_api::upsert_server),
        )
        .route(
            "/admin/api/servers/default",
            put(admin_api::set_default_server),
        )
        .route("/admin/api/servers/:id", delete(admin_api::delete_server))
        .route(
            "/admin/api/servers/:id/probe",
            get(admin_api::probe_server),
        )
        .route("/admin/api/hub/peers", get(admin_api::hub_peers))
        .route("/admin/api/hub/status", get(admin_api::hub_status));

    let app = Router::new()
        .merge(admin_routes)
        .route("/api", any(proxy_entry))
        .route("/api/", any(proxy_entry))
        .route("/api/*rest", any(proxy_entry))
        .route("/ws", get(ws_entry))
        .route("/ws/", get(ws_entry))
        .route("/ws/*rest", get(ws_entry))
        .fallback(static_files::serve_spa)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn proxy_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request,
) -> axum::response::Response {
    proxy::proxy_http(axum::extract::State(state), req).await
}

async fn ws_entry(
    ws: axum::extract::ws::WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<AppState>,
    uri: axum::http::Uri,
) -> impl IntoResponse {
    proxy::proxy_ws(ws, axum::extract::State(state), uri).await
}
