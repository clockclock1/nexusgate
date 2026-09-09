use p2p_common::{
    ConnectionId, NatInfo, NodeId, PathKind, PeerCandidate, PeerPathPurpose, PeerRole, ProtocolKind,
};
use serde::{Deserialize, Serialize};

/// Snapshot of a peer online on the Admin Hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubPeerInfo {
    pub node_id: String,
    pub role: PeerRole,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub online: bool,
}

/// Control-channel messages. Business data NEVER goes on the control channel
/// except compact management forwards mediated by the Admin Hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ControlMessage {
    Hello {
        version: String,
        #[serde(default)]
        features: Vec<String>,
    },
    Auth {
        node_id: String,
        token: String,
        /// Optional role when dialing the Admin Hub (`edge` / `server`).
        #[serde(default)]
        role: Option<PeerRole>,
    },
    AuthOk {
        node_id: String,
        #[serde(default)]
        server_time: Option<i64>,
        #[serde(default)]
        role: Option<PeerRole>,
    },
    Register {
        node_id: String,
        #[serde(default)]
        hostname: Option<String>,
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        labels: Vec<String>,
    },
    RegisterService {
        service_id: String,
        name: String,
        protocol: ProtocolKind,
        local_addr: String,
        #[serde(default)]
        public_port: Option<u16>,
        #[serde(default)]
        domain: Option<String>,
    },
    UnregisterService {
        service_id: String,
    },
    Heartbeat {
        #[serde(default)]
        seq: u64,
        #[serde(default)]
        rtt_ms: Option<u64>,
    },
    Connect {
        connection_id: String,
        data_token: String,
        local_addr: String,
        protocol: ProtocolKind,
        #[serde(default)]
        path: PathKind,
        #[serde(default)]
        service_id: Option<String>,
    },
    Accept {
        connection_id: String,
        #[serde(default)]
        ok: bool,
        #[serde(default)]
        reason: Option<String>,
    },
    Close {
        connection_id: String,
        #[serde(default)]
        reason: Option<String>,
    },
    NatInfo {
        info: NatInfo,
    },
    PeerInfo {
        peer_node_id: String,
        candidates: Vec<PeerCandidate>,
        connection_id: String,
    },
    PathProbe {
        connection_id: String,
        path: PathKind,
        #[serde(default)]
        probe_id: Option<String>,
    },
    PathResult {
        connection_id: String,
        path: PathKind,
        success: bool,
        #[serde(default)]
        latency_ms: Option<u64>,
        #[serde(default)]
        probe_id: Option<String>,
    },
    PathSwitch {
        connection_id: String,
        path: PathKind,
    },
    ConfigPush {
        #[serde(default)]
        revision: u64,
        #[serde(default)]
        payload: serde_json::Value,
    },
    ConfigUpdate {
        #[serde(default)]
        revision: u64,
        #[serde(default)]
        ack: bool,
    },
    Error {
        code: String,
        message: String,
        #[serde(default)]
        connection_id: Option<String>,
    },
    DataReady {
        connection_id: String,
        #[serde(default)]
        path: PathKind,
    },

    /// Hub → peers: full online peer roster (servers + edges).
    HubPeers {
        peers: Vec<HubPeerInfo>,
    },

    /// Edge/Server → Hub: request a path to another peer (P2P first, then relay).
    OpenPeerPath {
        request_id: String,
        target_id: String,
        #[serde(default)]
        purpose: PeerPathPurpose,
        #[serde(default)]
        prefer_p2p: bool,
    },

    /// Hub → both peers: dial Hub data port (relay) or use candidates (p2p).
    PeerPathOffer {
        request_id: String,
        connection_id: String,
        data_token: String,
        peer_node_id: String,
        path: PathKind,
        purpose: PeerPathPurpose,
        #[serde(default)]
        candidates: Vec<PeerCandidate>,
    },

    /// Browser/Admin → Server (via Hub control): forward an HTTP management call.
    MgmtForward {
        request_id: String,
        method: String,
        path: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
        #[serde(default)]
        body_b64: Option<String>,
    },

    /// Server → Hub → Admin: result of MgmtForward.
    MgmtForwardResult {
        request_id: String,
        status: u16,
        #[serde(default)]
        headers: Vec<(String, String)>,
        #[serde(default)]
        body_b64: Option<String>,
        #[serde(default)]
        error: Option<String>,
    },
}

impl ControlMessage {
    pub fn message_type(&self) -> &'static str {
        match self {
            Self::Hello { .. } => "HELLO",
            Self::Auth { .. } => "AUTH",
            Self::AuthOk { .. } => "AUTH_OK",
            Self::Register { .. } => "REGISTER",
            Self::RegisterService { .. } => "REGISTER_SERVICE",
            Self::UnregisterService { .. } => "UNREGISTER_SERVICE",
            Self::Heartbeat { .. } => "HEARTBEAT",
            Self::Connect { .. } => "CONNECT",
            Self::Accept { .. } => "ACCEPT",
            Self::Close { .. } => "CLOSE",
            Self::NatInfo { .. } => "NAT_INFO",
            Self::PeerInfo { .. } => "PEER_INFO",
            Self::PathProbe { .. } => "PATH_PROBE",
            Self::PathResult { .. } => "PATH_RESULT",
            Self::PathSwitch { .. } => "PATH_SWITCH",
            Self::ConfigPush { .. } => "CONFIG_PUSH",
            Self::ConfigUpdate { .. } => "CONFIG_UPDATE",
            Self::Error { .. } => "ERROR",
            Self::DataReady { .. } => "DATA_READY",
            Self::HubPeers { .. } => "HUB_PEERS",
            Self::OpenPeerPath { .. } => "OPEN_PEER_PATH",
            Self::PeerPathOffer { .. } => "PEER_PATH_OFFER",
            Self::MgmtForward { .. } => "MGMT_FORWARD",
            Self::MgmtForwardResult { .. } => "MGMT_FORWARD_RESULT",
        }
    }

    pub fn connect(
        connection_id: &ConnectionId,
        data_token: &str,
        local_addr: &str,
        protocol: ProtocolKind,
    ) -> Self {
        Self::Connect {
            connection_id: connection_id.as_str().to_string(),
            data_token: data_token.to_string(),
            local_addr: local_addr.to_string(),
            protocol,
            path: PathKind::Relay,
            service_id: None,
        }
    }

    pub fn auth(node_id: &NodeId, token: &str) -> Self {
        Self::Auth {
            node_id: node_id.as_str().to_string(),
            token: token.to_string(),
            role: None,
        }
    }

    pub fn auth_with_role(node_id: &str, token: &str, role: PeerRole) -> Self {
        Self::Auth {
            node_id: node_id.to_string(),
            token: token.to_string(),
            role: Some(role),
        }
    }

    pub fn heartbeat(seq: u64) -> Self {
        Self::Heartbeat { seq, rtt_ms: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_hello() {
        let msg = ControlMessage::Hello {
            version: "1.0".into(),
            features: vec!["p2p".into()],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("HELLO"));
        let back: ControlMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(back.message_type(), "HELLO");
    }

    #[test]
    fn serialize_connect() {
        let msg = ControlMessage::connect(
            &ConnectionId::new("c1"),
            "tok",
            "127.0.0.1:80",
            ProtocolKind::Tcp,
        );
        let json = serde_json::to_vec(&msg).unwrap();
        let back: ControlMessage = serde_json::from_slice(&json).unwrap();
        match back {
            ControlMessage::Connect {
                connection_id,
                data_token,
                ..
            } => {
                assert_eq!(connection_id, "c1");
                assert_eq!(data_token, "tok");
            }
            _ => panic!("expected CONNECT"),
        }
    }

    #[test]
    fn serialize_hub_auth_role() {
        let msg = ControlMessage::auth_with_role("s1", "tok", PeerRole::Server);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("AUTH"));
        let back: ControlMessage = serde_json::from_str(&json).unwrap();
        match back {
            ControlMessage::Auth { role, .. } => assert_eq!(role, Some(PeerRole::Server)),
            _ => panic!("expected AUTH"),
        }
    }
}
