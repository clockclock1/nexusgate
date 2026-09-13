use crate::config::OverlayRole;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Overlay control / data messages (JSON over UDP).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OverlayMsg {
    /// Join mesh (node or peer server).
    Join {
        node_id: String,
        token: String,
        role: OverlayRole,
        /// Self-reported UDP endpoint hint (often private); servers prefer observed `from`.
        #[serde(default)]
        endpoint: Option<String>,
        /// Peer overlay servers may announce a fixed VIP on join.
        #[serde(default)]
        vip: Option<String>,
    },
    JoinOk {
        node_id: String,
        vip: String,
        subnet_cidr: String,
        peers: Vec<PeerInfoMsg>,
        /// How this peer's UDP packet was seen by the answering server (reflexive).
        #[serde(default)]
        observed: Option<String>,
    },
    JoinDeny {
        reason: String,
    },
    /// Periodic liveness + endpoint refresh.
    Heartbeat {
        node_id: String,
        vip: String,
        endpoint: Option<String>,
    },
    /// Full/partial peer roster (servers gossip this).
    PeerList {
        peers: Vec<PeerInfoMsg>,
    },
    /// DHCP allocation announce between equal overlay servers.
    AllocAnnounce {
        node_id: String,
        vip: String,
    },
    /// Ask a peer (usually Overlay Server) what source address it sees.
    StunQuery {
        node_id: String,
        tx_id: String,
    },
    StunReply {
        node_id: String,
        tx_id: String,
        observed: String,
    },
    /// Bidirectional hole-punch probe.
    Punch {
        from_id: String,
        to_id: String,
        tx_id: String,
    },
    PunchAck {
        from_id: String,
        to_id: String,
        tx_id: String,
    },
    /// Server-assisted introduction: punch toward these candidate endpoints.
    PunchIntro {
        peer_id: String,
        peer_vip: String,
        peer_role: OverlayRole,
        endpoints: Vec<String>,
    },
    /// Encapsulated IPv4 packet (base64).
    Packet {
        from_vip: String,
        to_vip: String,
        #[serde(with = "b64")]
        payload: Vec<u8>,
    },
    /// Ask a server to relay a packet to `to_vip`.
    Relay {
        from_vip: String,
        to_vip: String,
        #[serde(with = "b64")]
        payload: Vec<u8>,
    },
    /// Management HTTP-ish request (panel → server) over the mesh. Addressed by node_id.
    MgmtReq {
        request_id: String,
        from_id: String,
        to_id: String,
        method: String,
        path: String,
        #[serde(default)]
        headers: Vec<(String, String)>,
        #[serde(with = "b64")]
        body: Vec<u8>,
    },
    MgmtRes {
        request_id: String,
        from_id: String,
        to_id: String,
        status: u16,
        #[serde(default)]
        headers: Vec<(String, String)>,
        #[serde(with = "b64")]
        body: Vec<u8>,
        #[serde(default)]
        error: Option<String>,
    },
    /// Edge announces a local service to Overlay Servers (replaces Hub REGISTER_SERVICE for mgmt).
    CtrlRegisterService {
        edge_id: String,
        service_id: String,
        name: String,
        protocol: String,
        local_addr: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfoMsg {
    pub node_id: String,
    pub vip: String,
    pub role: OverlayRole,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub endpoints: Vec<String>,
}

mod b64 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(data))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(serde::de::Error::custom)
    }
}

pub fn encode_msg(msg: &OverlayMsg) -> anyhow::Result<Vec<u8>> {
    Ok(serde_json::to_vec(msg)?)
}

pub fn decode_msg(buf: &[u8]) -> anyhow::Result<OverlayMsg> {
    Ok(serde_json::from_slice(buf)?)
}

pub fn parse_vip(s: &str) -> anyhow::Result<Ipv4Addr> {
    Ok(s.parse()?)
}
