use crate::state::{AppState, PendingTunnel};
use p2p_common::{PeerPathPurpose, ProtocolKind};
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::ControlMessage;
use p2p_transport::{self as transport, tune_tcp};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};
use uuid::Uuid;

pub async fn run_tcp_gateway(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, entry = "tcp", "penetration gateway listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        let cfg = state.config.read().clone();
        tune_tcp(&stream, cfg.tcp_nodelay, cfg.tcp_buffer_bytes);
        let state = state.clone();
        let listen_port = addr.port();
        tokio::spawn(async move {
            if let Err(e) = handle_public(state, stream, peer, listen_port, "tcp").await {
                warn!(%peer, error = %e, "tcp gateway connection failed");
            }
        });
    }
}

pub async fn run_quic_gateway(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let endpoint = transport::bind_quic_server(addr).await?;
    info!(%addr, entry = "quic", "penetration gateway listening");
    loop {
        match transport::accept_quic_bidi(&endpoint).await {
            Ok((stream, peer)) => {
                let state = state.clone();
                let listen_port = addr.port();
                tokio::spawn(async move {
                    if let Err(e) = handle_public(state, stream, peer, listen_port, "quic").await {
                        warn!(%peer, error = %e, "quic gateway connection failed");
                    }
                });
            }
            Err(e) => {
                warn!(error = %e, "quic gateway accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

pub async fn run_kcp_gateway(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let mut listener = transport::bind_kcp_server(addr).await?;
    info!(%addr, entry = "kcp", "penetration gateway listening");
    loop {
        match transport::accept_kcp(&mut listener).await {
            Ok((stream, peer)) => {
                let state = state.clone();
                let listen_port = addr.port();
                tokio::spawn(async move {
                    if let Err(e) = handle_public(state, stream, peer, listen_port, "kcp").await {
                        warn!(%peer, error = %e, "kcp gateway connection failed");
                    }
                });
            }
            Err(e) => {
                warn!(error = %e, "kcp gateway accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

async fn handle_public<S>(
    state: AppState,
    public: S,
    peer: SocketAddr,
    listen_port: u16,
    entry: &str,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Match by listen port; prefer ProtocolKind::Tcp for TCP entry, else any rule on that port.
    let rule = state
        .routes
        .match_port(ProtocolKind::Tcp, listen_port)
        .or_else(|| {
            state.routes.list().into_iter().find(|r| {
                r.enabled && r.listen_port == Some(listen_port)
            })
        })
        .ok_or_else(|| anyhow::anyhow!("no route for port {listen_port}"))?;

    let node_id = rule.node_id.as_str().to_string();
    let local_addr = rule.local_addr.clone();
    if !state.edge_online_via_hub(&node_id) {
        anyhow::bail!("edge offline on hub: {node_id}");
    }

    let request_id = Uuid::new_v4().to_string();
    state.hub_tunnels.insert(
        request_id.clone(),
        PendingTunnel {
            request_id: request_id.clone(),
            node_id: node_id.clone(),
            local_addr: local_addr.clone(),
            protocol: entry.into(),
            public: Arc::new(tokio::sync::Mutex::new(Some(Box::new(public)))),
            created_at: Instant::now(),
        },
    );

    state.hub_send(ControlMessage::OpenPeerPath {
        request_id: request_id.clone(),
        target_id: node_id.clone(),
        purpose: PeerPathPurpose::Data,
        prefer_p2p: true,
        local_addr: Some(local_addr.clone()),
        data_port: rule.data_port,
    })?;

    state.push_log(
        "INFO",
        "gateway",
        Some(node_id.clone()),
        format!("public {peer} entry={entry} -> hub tunnel {request_id} => {local_addr}"),
    );

    // Hub client completes bridge on PeerPathOffer; waiter cleaned on timeout by cleanup task.
    let _ = request_id;
    Ok(())
}

/// Accept a Hub data leg and bridge it with the waiting public visitor stream.
pub async fn complete_hub_tunnel(
    state: &AppState,
    request_id: &str,
    hub_data: TcpStream,
) -> anyhow::Result<()> {
    let Some((_, pending)) = state.hub_tunnels.remove(request_id) else {
        anyhow::bail!("no pending tunnel for {request_id}");
    };
    let public = pending
        .public
        .lock()
        .await
        .take()
        .ok_or_else(|| anyhow::anyhow!("public stream already taken"))?;
    let mode = format!("hub-relay/{}", pending.protocol);
    state.metrics.conn_opened(false);
    state.connections.insert(
        request_id.to_string(),
        crate::state::ActiveConnection {
            conn_id: request_id.to_string(),
            node_id: pending.node_id.clone(),
            protocol: pending.protocol.clone(),
            mode: mode.clone(),
            status: "active".into(),
            local_addr: pending.local_addr.clone(),
            remote_addr: "hub".into(),
            rx_bytes: 0,
            tx_bytes: 0,
            started_at: chrono::Utc::now(),
        },
    );
    state.broadcast(serde_json::json!({
        "type": "connection_created",
        "payload": { "conn_id": request_id, "node_id": pending.node_id, "mode": mode }
    }));

    let (rx, tx) = copy_bidirectional(public, hub_data)
        .await
        .unwrap_or((0, 0));
    state.metrics.add_rx(rx);
    state.metrics.add_tx(tx);
    state.metrics.conn_closed();
    state.connections.remove(request_id);
    state.broadcast(serde_json::json!({
        "type": "connection_closed",
        "payload": { "conn_id": request_id, "rx": rx, "tx": tx }
    }));
    Ok(())
}

pub async fn cleanup_hub_tunnels(state: AppState) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tick.tick().await;
        let stale: Vec<String> = state
            .hub_tunnels
            .iter()
            .filter(|e| e.created_at.elapsed() > std::time::Duration::from_secs(30))
            .map(|e| e.key().clone())
            .collect();
        for id in stale {
            state.hub_tunnels.remove(&id);
        }
    }
}
