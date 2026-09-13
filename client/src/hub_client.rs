//! Edge dial into Admin Hub: discover all servers, open P2P/relay peer paths.

use crate::config::EdgeConfig;
use p2p_common::{PeerPathPurpose, PeerRole};
use p2p_control::{ControlSession, HeartbeatConfig};
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::{encode_data_handshake, ControlMessage, HubPeerInfo};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn run_hub_client(cfg: EdgeConfig) {
    let Some(hub_host) = cfg.hub_host.clone().filter(|h| !h.trim().is_empty()) else {
        return;
    };
    loop {
        match run_once(&cfg, hub_host.trim()).await {
            Ok(()) => warn!("hub session closed, reconnecting..."),
            Err(e) => warn!(error = %e, "hub session error, reconnecting..."),
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn run_once(cfg: &EdgeConfig, hub_host: &str) -> anyhow::Result<()> {
    let control_addr = format!("{}:{}", hub_host, cfg.hub_control_port);
    let data_port = cfg.hub_data_port;
    let hub_token = cfg
        .hub_token
        .clone()
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| cfg.token.clone());

    info!(%control_addr, node_id = %cfg.node_id, "connecting to admin hub");
    let stream = TcpStream::connect(&control_addr).await?;
    let mut session = ControlSession::new(stream, HeartbeatConfig::default());

    let hello = session
        .recv_timeout(Duration::from_secs(10))
        .await?
        .ok_or_else(|| anyhow::anyhow!("hub closed"))?;
    if !matches!(hello, ControlMessage::Hello { .. }) {
        anyhow::bail!("expected HELLO from hub");
    }

    session
        .send(ControlMessage::auth_with_role(
            &cfg.node_id,
            &hub_token,
            PeerRole::Edge,
        ))
        .await?;

    let auth_ok = session
        .recv_timeout(Duration::from_secs(10))
        .await?
        .ok_or_else(|| anyhow::anyhow!("no AUTH_OK"))?;
    match auth_ok {
        ControlMessage::AuthOk { .. } => info!("authenticated to hub as edge"),
        ControlMessage::Error { code, message, .. } => {
            anyhow::bail!("hub auth failed: {code} {message}");
        }
        other => anyhow::bail!("unexpected {}", other.message_type()),
    }

    session
        .send(ControlMessage::Register {
            node_id: cfg.node_id.clone(),
            hostname: cfg.name.clone(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            labels: vec!["role:edge".into()],
            transports: vec![],
        })
        .await?;

    let servers: Arc<Mutex<Vec<HubPeerInfo>>> = Arc::new(Mutex::new(Vec::new()));
    let hub_host = hub_host.to_string();
    let (tx, mut rx, handle) = session.into_channels(256);

    // After first roster, open mgmt paths to every online server (P2P prefer → relay).
    let mut opened = std::collections::HashSet::new();

    while let Some(msg) = rx.recv().await {
        match msg {
            ControlMessage::HubPeers { peers } => {
                let server_peers: Vec<_> = peers
                    .into_iter()
                    .filter(|p| p.role == PeerRole::Server && p.online)
                    .collect();
                info!(servers = server_peers.len(), "hub roster (servers)");
                if let Ok(mut g) = servers.lock() {
                    *g = server_peers.clone();
                }
                for s in server_peers {
                    if opened.contains(&s.node_id) {
                        continue;
                    }
                    opened.insert(s.node_id.clone());
                    let req_id = Uuid::new_v4().to_string();
                    let _ = tx
                        .send(ControlMessage::OpenPeerPath {
                            request_id: req_id,
                            target_id: s.node_id,
                            purpose: PeerPathPurpose::Mgmt,
                            prefer_p2p: true,
                        })
                        .await;
                }
            }
            ControlMessage::PeerPathOffer {
                connection_id,
                data_token,
                path,
                purpose,
                peer_node_id,
                ..
            } => {
                info!(
                    %connection_id,
                    %peer_node_id,
                    ?path,
                    ?purpose,
                    "hub peer path offer"
                );
                let hub_host = hub_host.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        accept_peer_path(hub_host, data_port, connection_id, data_token).await
                    {
                        warn!(error = %e, "edge accept peer path failed");
                    }
                });
            }
            ControlMessage::Error { code, message, .. } => {
                warn!(%code, %message, "hub error");
            }
            _ => {}
        }
    }
    handle.abort();
    Ok(())
}

async fn accept_peer_path(
    hub_host: String,
    data_port: u16,
    connection_id: String,
    data_token: String,
) -> anyhow::Result<()> {
    let mut data = TcpStream::connect(format!("{hub_host}:{data_port}")).await?;
    let hs = encode_data_handshake(&connection_id, &data_token);
    data.write_all(&hs).await?;
    // Keep relay leg alive; hub bridges to the server peer.
    let (a, b) = tokio::io::duplex(8);
    let _ = copy_bidirectional(data, a).await;
    drop(b);
    Ok(())
}
