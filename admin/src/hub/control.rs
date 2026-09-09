use crate::hub::HubState;
use p2p_common::PeerRole;
use p2p_control::{ControlSession, HeartbeatConfig};
use p2p_protocol::ControlMessage;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

pub async fn run_hub_control(state: HubState, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "hub control listening");
    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_peer(state, stream).await {
                warn!(%peer, error = %e, "hub control session ended");
            }
        });
    }
}

async fn handle_peer(state: HubState, stream: TcpStream) -> anyhow::Result<()> {
    let mut session = ControlSession::new(stream, HeartbeatConfig::default());
    session
        .send(ControlMessage::Hello {
            version: env!("CARGO_PKG_VERSION").into(),
            features: vec!["hub".into(), "p2p".into(), "relay".into(), "mgmt".into()],
        })
        .await?;

    let auth = session
        .recv_timeout(Duration::from_secs(15))
        .await?
        .ok_or_else(|| anyhow::anyhow!("peer closed before AUTH"))?;
    let (node_id, role) = match auth {
        ControlMessage::Auth {
            node_id,
            token,
            role,
        } => {
            if !state.verify_token(&token) {
                let _ = session
                    .send(ControlMessage::Error {
                        code: "AUTH_FAILED".into(),
                        message: "invalid hub token".into(),
                        connection_id: None,
                    })
                    .await;
                anyhow::bail!("auth failed for {node_id}");
            }
            let role = role.unwrap_or(PeerRole::Edge);
            if !matches!(role, PeerRole::Edge | PeerRole::Server) {
                anyhow::bail!("invalid hub role");
            }
            (node_id, role)
        }
        other => anyhow::bail!("expected AUTH, got {}", other.message_type()),
    };

    session
        .send(ControlMessage::AuthOk {
            node_id: node_id.clone(),
            server_time: Some(unix_now()),
            role: Some(role),
        })
        .await?;

    let (tx, mut rx, handle) = session.into_channels(256);
    state.peers.insert(
        node_id.clone(),
        crate::hub::HubPeerSession {
            node_id: node_id.clone(),
            role,
            name: None,
            version: None,
            connected_at: std::time::Instant::now(),
            tx: tx.clone(),
        },
    );
    info!(%node_id, ?role, "hub peer online");
    state.broadcast_roster();

    while let Some(msg) = rx.recv().await {
        if let Err(e) = handle_hub_msg(&state, &node_id, msg).await {
            warn!(%node_id, error = %e, "hub msg handling failed");
        }
    }

    handle.abort();
    state.remove_peer(&node_id);
    info!(%node_id, "hub peer offline");
    Ok(())
}

async fn handle_hub_msg(state: &HubState, from_id: &str, msg: ControlMessage) -> anyhow::Result<()> {
    match msg {
        ControlMessage::Register {
            hostname,
            version,
            ..
        } => {
            if let Some(mut p) = state.peers.get_mut(from_id) {
                if hostname.is_some() {
                    p.name = hostname;
                }
                if version.is_some() {
                    p.version = version;
                }
            }
            state.broadcast_roster();
        }
        ControlMessage::OpenPeerPath {
            request_id,
            target_id,
            purpose,
            prefer_p2p,
        } => {
            if let Err(e) = state
                .open_peer_path(from_id, &target_id, &request_id, purpose, prefer_p2p)
                .await
            {
                state.send_to(
                    from_id,
                    ControlMessage::Error {
                        code: "OPEN_PATH_FAILED".into(),
                        message: e.to_string(),
                        connection_id: None,
                    },
                );
            }
        }
        ControlMessage::MgmtForwardResult {
            request_id,
            status,
            headers,
            body_b64,
            error,
        } => {
            let rid = request_id.clone();
            state.complete_mgmt(
                &rid,
                ControlMessage::MgmtForwardResult {
                    request_id,
                    status,
                    headers,
                    body_b64,
                    error,
                },
            );
        }
        ControlMessage::Heartbeat { .. } => {}
        ControlMessage::PeerInfo {
            peer_node_id,
            candidates,
            connection_id,
        } => {
            let _ = state.send_to(
                &peer_node_id,
                ControlMessage::PeerInfo {
                    peer_node_id: peer_node_id.clone(),
                    candidates,
                    connection_id,
                },
            );
        }
        ControlMessage::PathResult { .. } | ControlMessage::PathProbe { .. } => {}
        other => {
            tracing::debug!(from = %from_id, ty = other.message_type(), "ignored hub control msg");
        }
    }
    Ok(())
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
