use crate::control_plane::send_connect;
use crate::data_plane::register_pending;
use crate::state::AppState;
use p2p_common::ProtocolKind;
use p2p_security::generate_data_token;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn run_tcp_gateway(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "TCP gateway listening");
    loop {
        let (public, peer) = listener.accept().await?;
        let state = state.clone();
        let listen_port = addr.port();
        tokio::spawn(async move {
            if let Err(e) = handle_public(state, public, peer, listen_port).await {
                warn!(%peer, error = %e, "gateway connection failed");
            }
        });
    }
}

async fn handle_public(
    state: AppState,
    public: tokio::net::TcpStream,
    peer: SocketAddr,
    listen_port: u16,
) -> anyhow::Result<()> {
    let rule = state
        .routes
        .match_port(ProtocolKind::Tcp, listen_port)
        .ok_or_else(|| anyhow::anyhow!("no route for port {listen_port}"))?;

    let connection_id = Uuid::new_v4().to_string();
    let data_token = generate_data_token();
    let node_id = rule.node_id.as_str().to_string();
    let local_addr = rule.local_addr.clone();

    register_pending(
        &state,
        connection_id.clone(),
        data_token.clone(),
        node_id.clone(),
        local_addr.clone(),
        "tcp".into(),
        public,
    );

    send_connect(
        &state,
        &node_id,
        &connection_id,
        &data_token,
        &local_addr,
        ProtocolKind::Tcp,
    )
    .await?;

    state.push_log(
        "INFO",
        "gateway",
        Some(node_id),
        format!("public {peer} -> CONNECT {connection_id} => {local_addr}"),
    );
    Ok(())
}
