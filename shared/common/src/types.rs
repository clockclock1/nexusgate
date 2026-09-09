use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique node identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Unique connection identifier for a 1:1 data path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectionId(pub String);

impl ConnectionId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Protocol type for a service / route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolKind {
    Tcp,
    Udp,
    Http,
    Https,
}

impl ProtocolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Http => "http",
            Self::Https => "https",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tcp" => Some(Self::Tcp),
            "udp" => Some(Self::Udp),
            "http" => Some(Self::Http),
            "https" => Some(Self::Https),
            _ => None,
        }
    }
}

impl fmt::Display for ProtocolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Connection path preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PathKind {
    #[default]
    Relay,
    P2p,
}

/// Peer role in the management Hub mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerRole {
    Edge,
    Server,
    Hub,
}

impl PeerRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Edge => "edge",
            Self::Server => "server",
            Self::Hub => "hub",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "edge" | "client" => Some(Self::Edge),
            "server" | "supernode" => Some(Self::Server),
            "hub" | "admin" => Some(Self::Hub),
            _ => None,
        }
    }
}

/// Purpose of a Hub-mediated peer path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerPathPurpose {
    /// Management / control traffic between edge and server via Hub.
    Mgmt,
    /// Generic 1:1 data tunnel via Hub relay (or P2P).
    Data,
}

impl Default for PeerPathPurpose {
    fn default() -> Self {
        Self::Mgmt
    }
}

/// Service registration descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub service_id: String,
    pub name: String,
    pub protocol: ProtocolKind,
    pub local_addr: String,
    #[serde(default)]
    pub public_port: Option<u16>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

/// Route matching rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: String,
    pub protocol: ProtocolKind,
    #[serde(default)]
    pub listen_port: Option<u16>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub sni: Option<String>,
    pub node_id: NodeId,
    pub service_id: String,
    pub local_addr: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
}

fn default_true() -> bool {
    true
}

/// NAT / network info for P2P.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NatInfo {
    #[serde(default)]
    pub public_ip: Option<String>,
    #[serde(default)]
    pub public_port: Option<u16>,
    #[serde(default)]
    pub local_ip: Option<String>,
    #[serde(default)]
    pub local_port: Option<u16>,
    #[serde(default)]
    pub nat_type: Option<String>,
}

/// Peer candidate for hole punching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerCandidate {
    pub addr: String,
    pub priority: u32,
    #[serde(default)]
    pub protocol: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_id_unique() {
        let a = ConnectionId::generate();
        let b = ConnectionId::generate();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn protocol_kind_roundtrip() {
        assert_eq!(ProtocolKind::parse("TCP"), Some(ProtocolKind::Tcp));
        assert_eq!(ProtocolKind::Http.as_str(), "http");
    }
}
