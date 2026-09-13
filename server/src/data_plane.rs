use crate::state::{ActiveConnection, AppState, PendingConnection};
use bytes::BytesMut;
use chrono::Utc;
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::{extract_line, parse_data_handshake};
use p2p_transport::{self as transport, set_nodelay};
use std::net::SocketAddr;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

pub async fn run_data_plane(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, transport = "tcp", "data plane listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        set_nodelay(&stream, true);
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_data_stream(state, stream, peer, "tcp").await {
                warn!(%peer, error = %e, "tcp data connection failed");
            }
        });
    }
}

pub async fn run_quic_data_plane(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let endpoint = transport::bind_quic_server(addr).await?;
    info!(%addr, transport = "quic", "data plane listening");
    loop {
        match transport::accept_quic_bidi(&endpoint).await {
            Ok((stream, peer)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_data_stream(state, stream, peer, "quic").await {
                        warn!(%peer, error = %e, "quic data connection failed");
                    }
                });
            }
            Err(e) => {
                warn!(error = %e, "quic accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

pub async fn run_kcp_data_plane(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let mut listener = transport::bind_kcp_server(addr).await?;
    info!(%addr, transport = "kcp", "data plane listening");
    loop {
        match transport::accept_kcp(&mut listener).await {
            Ok((stream, peer)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_data_stream(state, stream, peer, "kcp").await {
                        warn!(%peer, error = %e, "kcp data connection failed");
                    }
                });
            }
            Err(e) => {
                warn!(error = %e, "kcp accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
}

async fn handle_data_stream<S>(
    state: AppState,
    mut stream: S,
    peer: SocketAddr,
    mode: &str,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = BytesMut::with_capacity(256);
    let deadline = tokio::time::sleep(std::time::Duration::from_secs(10));
    tokio::pin!(deadline);
    let line = loop {
        tokio::select! {
            _ = &mut deadline => anyhow::bail!("data handshake timeout"),
            n = stream.read_buf(&mut buf) => {
                let n = n?;
                if n == 0 { anyhow::bail!("eof before handshake"); }
                if let Some(line) = extract_line(&mut buf) {
                    break line;
                }
            }
        }
    };
    let (connection_id, data_token) = parse_data_handshake(&line)?;

    let public = {
        let mut pending = state
            .pending
            .get_mut(&connection_id)
            .ok_or_else(|| anyhow::anyhow!("unknown connection_id"))?;
        if pending.data_token != data_token {
            anyhow::bail!("invalid data token");
        }
        pending
            .public
            .take()
            .ok_or_else(|| anyhow::anyhow!("public side missing"))?
    };
    set_nodelay(&public, true);

    let pending = state.pending.remove(&connection_id).map(|(_, v)| v);
    let node_id = pending
        .as_ref()
        .map(|p| p.node_id.clone())
        .unwrap_or_default();
    let local_addr = pending
        .as_ref()
        .map(|p| p.local_addr.clone())
        .unwrap_or_default();
    let protocol = pending
        .as_ref()
        .map(|p| p.protocol.clone())
        .unwrap_or_else(|| "tcp".into());

    let mode_label = format!("relay/{mode}");
    state.metrics.conn_opened(false);
    state.connections.insert(
        connection_id.clone(),
        ActiveConnection {
            conn_id: connection_id.clone(),
            node_id: node_id.clone(),
            protocol: protocol.clone(),
            mode: mode_label.clone(),
            status: "active".into(),
            local_addr: local_addr.clone(),
            remote_addr: peer.to_string(),
            rx_bytes: 0,
            tx_bytes: 0,
            started_at: Utc::now(),
        },
    );
    state.broadcast(serde_json::json!({
        "type": "connection_created",
        "payload": { "conn_id": connection_id, "node_id": node_id, "mode": mode_label }
    }));

    let (rx, tx) = copy_bidirectional(public, stream).await.unwrap_or((0, 0));
    state.metrics.add_rx(rx);
    state.metrics.add_tx(tx);
    state.metrics.conn_closed();
    state.connections.remove(&connection_id);
    state.broadcast(serde_json::json!({
        "type": "connection_closed",
        "payload": { "conn_id": connection_id, "rx": rx, "tx": tx }
    }));
    Ok(())
}

/// Register a pending public connection waiting for edge data dial.
pub fn register_pending(
    state: &AppState,
    connection_id: String,
    data_token: String,
    node_id: String,
    local_addr: String,
    protocol: String,
    public: TcpStream,
) {
    set_nodelay(&public, true);
    state.pending.insert(
        connection_id.clone(),
        PendingConnection {
            connection_id,
            data_token,
            node_id,
            local_addr,
            protocol,
            public: Some(public),
            data: None,
            created_at: Instant::now(),
            notify: None,
        },
    );
}

/// Background cleanup for stale pending connections.
pub async fn cleanup_pending(state: AppState) {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tick.tick().await;
        let stale: Vec<String> = state
            .pending
            .iter()
            .filter(|e| e.created_at.elapsed() > std::time::Duration::from_secs(30))
            .map(|e| e.key().clone())
            .collect();
        for id in stale {
            state.pending.remove(&id);
        }
        state.metrics.sample();
    }
}
