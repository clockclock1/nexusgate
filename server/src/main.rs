use clap::Parser;
use p2p_common::TransportKind;
use p2p_server::api;
use p2p_server::config::ServerConfig;
use p2p_server::db;
use p2p_server::gateway;
use p2p_server::state::AppState;
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "p2p-server",
    about = "NexusGate Server Node — penetration gateway (Hub client; no public control plane)"
)]
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
    if config
        .hub_host
        .as_ref()
        .map(|h| h.trim().is_empty())
        .unwrap_or(true)
    {
        anyhow::bail!("hub_host is required: server has no local control plane; dial Admin Hub");
    }
    tracing::info!(
        gateway_tcp = config.gateway_port,
        gateway_quic = config.gateway_quic_port,
        gateway_kcp = config.gateway_kcp_port,
        api_localhost = config.api_port,
        hub = ?config.hub_host,
        "starting p2p-server (penetration node)"
    );

    let pool = db::init_db(&config).await?;
    let state = AppState::new(config.clone(), std::path::PathBuf::from(&args.config), pool);
    db::load_routes_into(&state.db, &state.routes).await?;

    let listen = config.listen.clone();
    for t in config.enabled_gateway_transports() {
        let port = config.gateway_port_for(t);
        let addr: SocketAddr = format!("{listen}:{port}").parse()?;
        let s = state.clone();
        match t {
            TransportKind::Tcp => {
                tokio::spawn(async move {
                    if let Err(e) = gateway::run_tcp_gateway(s, addr).await {
                        tracing::error!(error = %e, "tcp gateway exited");
                    }
                });
            }
            TransportKind::Quic => {
                tokio::spawn(async move {
                    if let Err(e) = gateway::run_quic_gateway(s, addr).await {
                        tracing::error!(error = %e, "quic gateway exited");
                    }
                });
            }
            TransportKind::Kcp => {
                tokio::spawn(async move {
                    if let Err(e) = gateway::run_kcp_gateway(s, addr).await {
                        tracing::error!(error = %e, "kcp gateway exited");
                    }
                });
            }
        }
    }

    let s_clean = state.clone();
    tokio::spawn(async move {
        gateway::cleanup_hub_tunnels(s_clean).await;
    });

    let s_hub = state.clone();
    tokio::spawn(async move {
        p2p_server::hub_client::run_hub_client(s_hub).await;
    });

    // Internal management mesh: answer panel API + accept edge service register.
    if config.overlay.enabled {
        let mut o = config.overlay.clone();
        o.role = p2p_overlay::OverlayRole::Server;
        if o.node_id.trim().is_empty() {
            o.node_id = config
                .hub_server_id
                .clone()
                .unwrap_or_else(|| "default".into());
        }
        if o.fixed_vip.is_none() {
            o.fixed_vip = Some("10.88.0.2".into());
        }
        if o.bootstrap.is_empty() {
            if let Some(host) = config.hub_host.as_ref() {
                o.bootstrap.push(format!("{host}:{}", o.port));
            }
        }
        let api_port = config.api_port;
        let db = state.db.clone();
        match p2p_overlay::start_overlay(o).await {
            Ok(mesh) => {
                tracing::info!(
                    vip = %mesh.vip(),
                    node_id = %mesh.node_id(),
                    "management mesh ready (server)"
                );
                mesh.set_mgmt_handler(move |req| {
                    let api_port = api_port;
                    async move {
                        match p2p_server::hub_client::execute_local_mgmt_bytes(
                            api_port,
                            req.method,
                            req.path,
                            req.headers,
                            Some(req.body),
                        )
                        .await
                        {
                            Ok((status, headers, body)) => p2p_overlay::MgmtResponse {
                                status,
                                headers,
                                body,
                                error: None,
                            },
                            Err(e) => p2p_overlay::MgmtResponse::err(e.to_string()),
                        }
                    }
                })
                .await;
                mesh.set_register_handler(move |ev| {
                    let db = db.clone();
                    async move {
                        if let Err(e) = p2p_server::hub_client::upsert_registered_service(
                            &db,
                            &ev.edge_id,
                            &ev.service_id,
                            &ev.name,
                            &ev.protocol,
                            &ev.local_addr,
                        )
                        .await
                        {
                            tracing::warn!(
                                error = %e,
                                service = %ev.service_id,
                                "overlay service register failed"
                            );
                        } else {
                            tracing::info!(
                                service = %ev.service_id,
                                edge = %ev.edge_id,
                                "overlay service registered"
                            );
                        }
                    }
                })
                .await;
                // Keep mesh alive for process lifetime (UDP loops already hold state Arc).
                tokio::spawn(async move {
                    let _mesh = mesh;
                    std::future::pending::<()>().await;
                });
            }
            Err(e) => {
                tracing::warn!(error = %e, "management mesh failed; Hub fallback only");
            }
        }
    }

    // Management API: localhost only (reached via mesh mgmt or Hub MGMT_FORWARD).
    let api_addr: SocketAddr = format!("127.0.0.1:{}", config.api_port).parse()?;
    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind(api_addr).await?;
    tracing::info!(%api_addr, "management API (localhost only)");
    axum::serve(listener, app).await?;
    Ok(())
}
