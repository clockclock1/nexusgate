mod config;
mod proxy;
mod static_files;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::routing::{any, get};
use axum::Router;
use clap::Parser;
use config::AdminConfig;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    pub http: reqwest::Client,
    pub http_upstream: String,
    pub ws_upstream: String,
}

#[derive(Parser, Debug)]
#[command(name = "p2p-admin", about = "NexusGate Admin Panel (embedded SPA + API proxy)")]
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

    let state = AppState {
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?,
        http_upstream: cfg.http_upstream_base(),
        ws_upstream: cfg.ws_upstream_base(),
    };

    tracing::info!(
        %addr,
        upstream = %cfg.api_upstream,
        assets = static_files::embedded_asset_count(),
        "starting p2p-admin"
    );

    // axum 0.7 wildcard path syntax: /*rest
    let app = Router::new()
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
