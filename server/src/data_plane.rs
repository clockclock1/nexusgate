use crate::state::{ActiveConnection, AppState, PendingConnection};
use bytes::BytesMut;
use chrono::Utc;
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::{extract_line, parse_data_handshake};
use p2p_transport::{self as transport, tune_tcp};
use std::net::SocketAddr;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

pub async fn run_data_plane(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, transport = "tcp", "data plane listening (edge direct dial)");
    loop {
        let (stream, peer) = listener.accept().await?;
        let cfg = state.config.read().clone();
        tune_tcp(&stream, cfg.tcp_nodelay, cfg.tcp_buffer_bytes);
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
    // Keep any bytes past the handshake line (should be rare on direct dial).
    let stream = p2p_dataplane::PrefixedStream::new(buf, stream);

    let entry = state
        .pending
        .get(&connection_id)
        .ok_or_else(|| anyhow::anyhow!("unknown connection_id"))?;
    if entry.data_token != data_token {
        anyhow::bail!("invalid data token");
    }
    let public_slot = entry.public.clone();
    let node_id = entry.node_id.clone();
    let local_addr = entry.local_addr.clone();
    let protocol = entry.protocol.clone();
    drop(entry);

    let public = public_slot
        .lock()
        .await
        .take()
        .ok_or_else(|| anyhow::anyhow!("public side missing"))?;
    state.pending.remove(&connection_id);

    let mode_label = format!("direct/{mode}");
    state.metrics.conn_opened(true);
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
    info!(%connection_id, %peer, "direct data path established");

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
    public: Box<dyn p2p_transport::IoStream>,
) {
    state.pending.insert(
        connection_id.clone(),
        PendingConnection {
            connection_id,
            data_token,
            node_id,
            local_addr,
            protocol,
            public: std::sync::Arc::new(tokio::sync::Mutex::new(Some(public))),
            created_at: Instant::now(),
        },
    );
}

/// Bridge a public visitor with a Hub relay TcpStream (fallback path).
pub async fn bridge_hub_relay(
    state: &AppState,
    connection_id: &str,
    request_id: &str,
    hub_data: TcpStream,
    mode_suffix: &str,
    node_id: &str,
    local_addr: &str,
) -> anyhow::Result<()> {
    let public = if let Some((_, pending)) = state.pending.remove(connection_id) {
        pending
            .public
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("public stream missing in pending"))?
    } else if let Some((_, pending)) = state.hub_tunnels.remove(request_id) {
        pending
            .public
            .lock()
            .await
            .take()
            .ok_or_else(|| anyhow::anyhow!("public stream already taken"))?
    } else {
        anyhow::bail!("no pending tunnel for direct/hub fallback {connection_id}/{request_id}");
    };

    let mode = format!("hub-relay/{mode_suffix}");
    state.metrics.conn_opened(false);
    state.connections.insert(
        connection_id.to_string(),
        ActiveConnection {
            conn_id: connection_id.to_string(),
            node_id: node_id.to_string(),
            protocol: mode_suffix.into(),
            mode: mode.clone(),
            status: "active".into(),
            local_addr: local_addr.to_string(),
            remote_addr: "hub".into(),
            rx_bytes: 0,
            tx_bytes: 0,
            started_at: Utc::now(),
        },
    );
    state.broadcast(serde_json::json!({
        "type": "connection_created",
        "payload": { "conn_id": connection_id, "node_id": node_id, "mode": mode }
    }));

    let (rx, tx) = copy_bidirectional(public, hub_data)
        .await
        .unwrap_or((0, 0));
    state.metrics.add_rx(rx);
    state.metrics.add_tx(tx);
    state.metrics.conn_closed();
    state.connections.remove(connection_id);
    state.broadcast(serde_json::json!({
        "type": "connection_closed",
        "payload": { "conn_id": connection_id, "rx": rx, "tx": tx }
    }));
    Ok(())
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
