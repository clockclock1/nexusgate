use crate::state::{AppState, OnlineNode};
use p2p_common::NodeId;
use p2p_control::{ControlSession, HeartbeatConfig};
use p2p_protocol::ControlMessage;
use p2p_security::verify_node_token;
use std::net::SocketAddr;
use std::time::Instant;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

pub async fn run_control_plane(state: AppState, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "control plane listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_control(state, stream, peer).await {
                warn!(%peer, error = %e, "control session ended");
            }
        });
    }
}

async fn handle_control(state: AppState, stream: TcpStream, peer: SocketAddr) -> anyhow::Result<()> {
    let mut session = ControlSession::new(stream, HeartbeatConfig::default());

    // HELLO
    session
        .send(ControlMessage::Hello {
            version: env!("CARGO_PKG_VERSION").into(),
            features: vec!["tcp".into(), "relay".into(), "p2p".into()],
        })
        .await?;

    // Expect AUTH
    let auth = session
        .recv_timeout(std::time::Duration::from_secs(15))
        .await?
        .ok_or_else(|| anyhow::anyhow!("control closed before auth"))?;
    let (node_id, token) = match auth {
        ControlMessage::Auth { node_id, token, .. } => (node_id, token),
        other => {
            session
                .send(ControlMessage::Error {
                    code: "AUTH_REQUIRED".into(),
                    message: format!("expected AUTH, got {}", other.message_type()),
                    connection_id: None,
                })
                .await?;
            anyhow::bail!("auth required");
        }
    };

    let Some((token_hash, enabled, name)) = crate::db::get_node_token_hash(&state.db, &node_id).await? else {
        session
            .send(ControlMessage::Error {
                code: "UNKNOWN_NODE".into(),
                message: "node not registered".into(),
                connection_id: None,
            })
            .await?;
        anyhow::bail!("unknown node");
    };
    if !enabled || !verify_node_token(&NodeId::new(&node_id), &token, &token_hash) {
        session
            .send(ControlMessage::Error {
                code: "AUTH_FAILED".into(),
                message: "invalid token or disabled".into(),
                connection_id: None,
            })
            .await?;
        anyhow::bail!("auth failed");
    }

    session
        .send(ControlMessage::AuthOk {
            node_id: node_id.clone(),
            server_time: Some(chrono::Utc::now().timestamp()),
            role: None,
        })
        .await?;

    let (tx, mut rx, handle) = session.into_channels(256);
    let online = OnlineNode {
        node_id: node_id.clone(),
        name: name.clone(),
        version: "0.1.0".into(),
        public_ip: Some(peer.ip().to_string()),
        nat_type: None,
        connected_at: Instant::now(),
        last_seen: Instant::now(),
        tx: tx.clone(),
    };
    state.online.insert(node_id.clone(), online);
    state.refresh_online_metric();
    state.push_log("INFO", "control", Some(node_id.clone()), format!("node online from {peer}"));
    state.broadcast(serde_json::json!({
        "type": "node_online",
        "payload": { "node_id": node_id, "name": name }
    }));
    let _ = sqlx::query("UPDATE nodes SET status = 'online', public_ip = ? WHERE node_id = ?")
        .bind(peer.ip().to_string())
        .bind(&node_id)
        .execute(&state.db)
        .await;

    while let Some(msg) = rx.recv().await {
        if let Some(mut n) = state.online.get_mut(&node_id) {
            n.last_seen = Instant::now();
        }
        match msg {
            ControlMessage::Register {
                hostname,
                version,
                ..
            } => {
                if let Some(mut n) = state.online.get_mut(&node_id) {
                    if let Some(v) = version {
                        n.version = v.clone();
                        let _ = sqlx::query("UPDATE nodes SET version = ? WHERE node_id = ?")
                            .bind(&v)
                            .bind(&node_id)
                            .execute(&state.db)
                            .await;
                    }
                    let _ = hostname;
                }
            }
            ControlMessage::RegisterService {
                service_id,
                name,
                protocol,
                local_addr,
                ..
            } => {
                let now = chrono::Utc::now().to_rfc3339();
                let _ = sqlx::query(
                    r#"INSERT INTO services (service_id, name, node_id, protocol, local_addr, enabled, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, 1, ?, ?)
                       ON CONFLICT(service_id) DO UPDATE SET name=excluded.name, protocol=excluded.protocol,
                       local_addr=excluded.local_addr, updated_at=excluded.updated_at"#,
                )
                .bind(&service_id)
                .bind(&name)
                .bind(&node_id)
                .bind(protocol.as_str())
                .bind(&local_addr)
                .bind(&now)
                .bind(&now)
                .execute(&state.db)
                .await;
                state.push_log(
                    "INFO",
                    "service",
                    Some(node_id.clone()),
                    format!("registered service {name} -> {local_addr}"),
                );
            }
            ControlMessage::NatInfo { info } => {
                if let Some(mut n) = state.online.get_mut(&node_id) {
                    n.nat_type = info.nat_type.clone();
                    n.public_ip = info.public_ip.or(n.public_ip.clone());
                }
            }
            ControlMessage::Accept {
                connection_id,
                ok,
                reason,
            } => {
                if !ok {
                    state.push_log(
                        "WARN",
                        "connect",
                        Some(node_id.clone()),
                        format!("ACCEPT failed {connection_id}: {:?}", reason),
                    );
                }
            }
            ControlMessage::Error { code, message, .. } => {
                state.push_log(
                    "ERROR",
                    "edge",
                    Some(node_id.clone()),
                    format!("{code}: {message}"),
                );
            }
            _ => {}
        }
    }

    handle.abort();
    state.online.remove(&node_id);
    state.refresh_online_metric();
    let _ = sqlx::query("UPDATE nodes SET status = 'offline' WHERE node_id = ?")
        .bind(&node_id)
        .execute(&state.db)
        .await;
    state.push_log("INFO", "control", Some(node_id.clone()), "node offline");
    state.broadcast(serde_json::json!({
        "type": "node_offline",
        "payload": { "node_id": node_id }
    }));
    Ok(())
}

/// Send CONNECT to an online edge node.
pub async fn send_connect(
    state: &AppState,
    node_id: &str,
    connection_id: &str,
    data_token: &str,
    local_addr: &str,
    protocol: p2p_common::ProtocolKind,
) -> anyhow::Result<()> {
    let Some(node) = state.online.get(node_id) else {
        anyhow::bail!("node offline: {node_id}");
    };
    node.tx
        .send(ControlMessage::Connect {
            connection_id: connection_id.into(),
            data_token: data_token.into(),
            local_addr: local_addr.into(),
            protocol,
            path: p2p_common::PathKind::Relay,
            service_id: None,
        })
        .await
        .map_err(|_| anyhow::anyhow!("control send failed"))?;
    Ok(())
}
