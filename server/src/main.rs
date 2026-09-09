use clap::Parser;
use p2p_server::api;
use p2p_server::config::ServerConfig;
use p2p_server::control_plane;
use p2p_server::data_plane;
use p2p_server::db;
use p2p_server::gateway;
use p2p_server::state::AppState;
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "p2p-server", about = "NexusGate Super Node")]
struct Args {
    /// Config file path (default: server.toml beside this executable)
    #[arg(short, long, default_value_t = ServerConfig::default_path().display().to_string())]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let args = Args::parse();
    let config = ServerConfig::load(&args.config)?;
    tracing::info!(
        control = config.control_port,
        data = config.data_port,
        gateway = config.gateway_port,
        api = config.api_port,
        "starting p2p-server"
    );

    let pool = db::init_db(&config).await?;
    let state = AppState::new(config.clone(), std::path::PathBuf::from(&args.config), pool);
    db::load_routes_into(&state.db, &state.routes).await?;

    let listen = config.listen.clone();
    let control_addr: SocketAddr = format!("{}:{}", listen, config.control_port).parse()?;
    let data_addr: SocketAddr = format!("{}:{}", listen, config.data_port).parse()?;
    let gateway_addr: SocketAddr = format!("{}:{}", listen, config.gateway_port).parse()?;
    let api_addr: SocketAddr = format!("{}:{}", listen, config.api_port).parse()?;

    let s1 = state.clone();
    tokio::spawn(async move {
        if let Err(e) = control_plane::run_control_plane(s1, control_addr).await {
            tracing::error!(error = %e, "control plane exited");
        }
    });
    let s2 = state.clone();
    tokio::spawn(async move {
        if let Err(e) = data_plane::run_data_plane(s2, data_addr).await {
            tracing::error!(error = %e, "data plane exited");
        }
    });
    let s3 = state.clone();
    tokio::spawn(async move {
        if let Err(e) = gateway::run_tcp_gateway(s3, gateway_addr).await {
            tracing::error!(error = %e, "gateway exited");
        }
    });
    let s4 = state.clone();
    tokio::spawn(async move {
        data_plane::cleanup_pending(s4).await;
    });

    let s5 = state.clone();
    tokio::spawn(async move {
        p2p_server::hub_client::run_hub_client(s5).await;
    });

    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind(api_addr).await?;
    tracing::info!(%api_addr, "management API listening");
    axum::serve(listener, app).await?;
    Ok(())
}
