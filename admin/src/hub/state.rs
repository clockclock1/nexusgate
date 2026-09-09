use dashmap::DashMap;
use p2p_common::{PathKind, PeerPathPurpose, PeerRole};
use p2p_protocol::{ControlMessage, HubPeerInfo};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct HubPeerSession {
    pub node_id: String,
    pub role: PeerRole,
    pub name: Option<String>,
    pub version: Option<String>,
    pub connected_at: Instant,
    pub tx: mpsc::Sender<ControlMessage>,
}

pub struct PendingPeerPath {
    pub connection_id: String,
    pub data_token: String,
    pub a_id: String,
    pub b_id: String,
    pub purpose: PeerPathPurpose,
    pub path: PathKind,
    pub created_at: Instant,
    pub stream_slot: Arc<tokio::sync::Mutex<Option<TcpStream>>>,
}

#[derive(Clone)]
pub struct HubState {
    pub token: Arc<String>,
    pub peers: Arc<DashMap<String, HubPeerSession>>,
    pub pending_paths: Arc<DashMap<String, PendingPeerPath>>,
    pub mgmt_waiters: Arc<DashMap<String, oneshot::Sender<ControlMessage>>>,
}

impl HubState {
    pub fn new(hub_token: String) -> Self {
        Self {
            token: Arc::new(hub_token),
            peers: Arc::new(DashMap::new()),
            pending_paths: Arc::new(DashMap::new()),
            mgmt_waiters: Arc::new(DashMap::new()),
        }
    }

    pub fn verify_token(&self, token: &str) -> bool {
        !self.token.is_empty() && token == self.token.as_str()
    }

    pub fn roster(&self) -> Vec<HubPeerInfo> {
        self.peers
            .iter()
            .map(|e| {
                let p = e.value();
                HubPeerInfo {
                    node_id: p.node_id.clone(),
                    role: p.role,
                    name: p.name.clone(),
                    version: p.version.clone(),
                    online: true,
                }
            })
            .collect()
    }

    pub fn broadcast_roster(&self) {
        let peers = self.roster();
        let msg = ControlMessage::HubPeers { peers };
        for e in self.peers.iter() {
            let _ = e.value().tx.try_send(msg.clone());
        }
    }

    pub fn send_to(&self, node_id: &str, msg: ControlMessage) -> bool {
        self.peers
            .get(node_id)
            .map(|p| p.tx.try_send(msg).is_ok())
            .unwrap_or(false)
    }

    pub fn is_server_online(&self, id: &str) -> bool {
        self.peers
            .get(id)
            .map(|p| p.role == PeerRole::Server)
            .unwrap_or(false)
    }

    pub fn remove_peer(&self, node_id: &str) {
        self.peers.remove(node_id);
        self.broadcast_roster();
    }

    /// Prefer P2P (scaffold → fail) then open a Hub relay path between two peers.
    pub async fn open_peer_path(
        &self,
        from_id: &str,
        target_id: &str,
        request_id: &str,
        purpose: PeerPathPurpose,
        prefer_p2p: bool,
    ) -> anyhow::Result<()> {
        if !self.peers.contains_key(from_id) {
            anyhow::bail!("source peer offline");
        }
        if !self.peers.contains_key(target_id) {
            anyhow::bail!("target peer offline");
        }

        let mut path = PathKind::Relay;
        if prefer_p2p {
            // Scaffold: real hole-punch not wired yet; fall back to relay.
            let p2p_ok = p2p_p2p::probe_path(p2p_p2p::PathProbeRequest {
                connection_id: request_id.to_string(),
                path: PathKind::P2p,
                probe_id: request_id.to_string(),
            })
            .await
            .map(|r| r.success)
            .unwrap_or(false);
            path = p2p_p2p::select_path(p2p_ok, true);
        }

        let connection_id = uuid::Uuid::new_v4().to_string();
        let data_token = p2p_security::generate_data_token();

        self.pending_paths.insert(
            connection_id.clone(),
            PendingPeerPath {
                connection_id: connection_id.clone(),
                data_token: data_token.clone(),
                a_id: from_id.to_string(),
                b_id: target_id.to_string(),
                purpose,
                path,
                created_at: Instant::now(),
                stream_slot: Arc::new(tokio::sync::Mutex::new(None)),
            },
        );

        let offer_a = ControlMessage::PeerPathOffer {
            request_id: request_id.to_string(),
            connection_id: connection_id.clone(),
            data_token: data_token.clone(),
            peer_node_id: target_id.to_string(),
            path,
            purpose,
            candidates: vec![],
        };
        let offer_b = ControlMessage::PeerPathOffer {
            request_id: request_id.to_string(),
            connection_id,
            data_token,
            peer_node_id: from_id.to_string(),
            path,
            purpose,
            candidates: vec![],
        };
        if !self.send_to(from_id, offer_a) {
            anyhow::bail!("failed to notify source");
        }
        if !self.send_to(target_id, offer_b) {
            anyhow::bail!("failed to notify target");
        }
        Ok(())
    }

    /// Forward HTTP management call to an online server peer; wait for result.
    pub async fn mgmt_forward(
        &self,
        server_id: &str,
        method: &str,
        path: &str,
        headers: Vec<(String, String)>,
        body: Option<Vec<u8>>,
        timeout: Duration,
    ) -> anyhow::Result<ControlMessage> {
        if !self.is_server_online(server_id) {
            anyhow::bail!("server not connected to hub: {server_id}");
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.mgmt_waiters.insert(request_id.clone(), tx);

        let body_b64 = body.map(|b| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(b)
        });
        let ok = self.send_to(
            server_id,
            ControlMessage::MgmtForward {
                request_id: request_id.clone(),
                method: method.to_string(),
                path: path.to_string(),
                headers,
                body_b64,
            },
        );
        if !ok {
            self.mgmt_waiters.remove(&request_id);
            anyhow::bail!("failed to send mgmt forward");
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(_)) => {
                self.mgmt_waiters.remove(&request_id);
                anyhow::bail!("mgmt forward canceled");
            }
            Err(_) => {
                self.mgmt_waiters.remove(&request_id);
                anyhow::bail!("mgmt forward timeout");
            }
        }
    }

    pub fn complete_mgmt(&self, request_id: &str, msg: ControlMessage) {
        if let Some((_, tx)) = self.mgmt_waiters.remove(request_id) {
            let _ = tx.send(msg);
        }
    }

    pub fn cleanup_stale_pending(&self, max_age: Duration) {
        self.pending_paths
            .retain(|_, p| p.created_at.elapsed() < max_age);
    }
}
