use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Overlay role in the management mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlayRole {
    /// Equal-level overlay server (admin panel host OR penetration server host).
    Server,
    /// Client / edge node joining the mesh.
    Node,
}

impl Default for OverlayRole {
    fn default() -> Self {
        Self::Node
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayConfig {
    /// Enable overlay management mesh.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// UDP listen/bootstrap port for overlay.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Bind address for overlay UDP (servers usually 0.0.0.0).
    #[serde(default = "default_listen")]
    pub listen: String,
    /// Virtual subnet, e.g. 10.88.0.0/16
    #[serde(default = "default_subnet")]
    pub subnet_cidr: String,
    /// DHCP pool start (inclusive)
    #[serde(default = "default_pool_start")]
    pub dhcp_start: String,
    /// DHCP pool end (inclusive)
    #[serde(default = "default_pool_end")]
    pub dhcp_end: String,
    /// Shared join token
    #[serde(default = "default_token")]
    pub token: String,
    /// This process role
    #[serde(default)]
    pub role: OverlayRole,
    /// Stable node id in the overlay (filled by each binary if empty).
    #[serde(default)]
    pub node_id: String,
    /// Optional fixed VIP for overlay servers (admin/server). Empty = auto from pool.
    #[serde(default)]
    pub fixed_vip: Option<String>,
    /// Other known overlay servers (host:port) for equal-server sync + join bootstrap.
    #[serde(default)]
    pub bootstrap: Vec<String>,
    /// Create OS TUN device (unix). On Windows always userspace until wintun is wired.
    #[serde(default = "default_create_tun")]
    pub create_tun: bool,
}

fn default_true() -> bool {
    true
}
fn default_create_tun() -> bool {
    cfg!(unix)
}
fn default_port() -> u16 {
    51820
}
fn default_listen() -> String {
    "0.0.0.0".into()
}
fn default_subnet() -> String {
    "10.88.0.0/16".into()
}
fn default_pool_start() -> String {
    "10.88.0.10".into()
}
fn default_pool_end() -> String {
    "10.88.0.250".into()
}
fn default_token() -> String {
    "change-me-overlay-token".into()
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_port(),
            listen: default_listen(),
            subnet_cidr: default_subnet(),
            dhcp_start: default_pool_start(),
            dhcp_end: default_pool_end(),
            token: default_token(),
            role: OverlayRole::Node,
            node_id: String::new(),
            fixed_vip: None,
            bootstrap: vec![],
            create_tun: cfg!(unix),
        }
    }
}

impl OverlayConfig {
    pub fn parse_vip(s: &str) -> anyhow::Result<Ipv4Addr> {
        Ok(s.parse()?)
    }

    pub fn with_defaults(mut self, role: OverlayRole, node_id: impl Into<String>) -> Self {
        if self.node_id.trim().is_empty() {
            self.node_id = node_id.into();
        }
        self.role = role;
        if !self.enabled {
            // keep disabled unless caller enables
        }
        self
    }

    pub fn pool_bounds(&self) -> anyhow::Result<(Ipv4Addr, Ipv4Addr)> {
        Ok((self.dhcp_start.parse()?, self.dhcp_end.parse()?))
    }

    pub fn subnet_prefix(&self) -> anyhow::Result<(Ipv4Addr, u8)> {
        let (net, pfx) = self
            .subnet_cidr
            .split_once('/')
            .ok_or_else(|| anyhow::anyhow!("invalid subnet_cidr {}", self.subnet_cidr))?;
        Ok((net.parse()?, pfx.parse()?))
    }
}
