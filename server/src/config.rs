use p2p_common::TransportKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Server node: only opens penetration (mapped) ports publicly.
/// Control/management goes outbound to Admin Hub — no public control/API listen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    /// TCP penetration entry (mapped public port).
    #[serde(default = "default_gateway_port")]
    pub gateway_port: u16,
    /// QUIC penetration entry (mapped public UDP port).
    #[serde(default = "default_gateway_quic_port")]
    pub gateway_quic_port: u16,
    /// KCP penetration entry (mapped public UDP port).
    #[serde(default = "default_gateway_kcp_port")]
    pub gateway_kcp_port: u16,
    /// Which penetration entries to listen: tcp | quic | kcp (default all three).
    #[serde(default = "default_gateway_transports")]
    pub gateway_transports: Vec<TransportKind>,
    /// Localhost-only management API (not publicly exposed).
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default = "default_db")]
    pub database_url: String,
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_jwt_ttl")]
    pub jwt_ttl_secs: i64,
    #[serde(default = "default_admin_user")]
    pub admin_user: String,
    #[serde(default = "default_admin_pass")]
    pub admin_password: String,
    #[serde(default)]
    pub max_connections: u64,
    #[serde(default)]
    pub enable_relay: bool,
    #[serde(default)]
    pub enable_p2p: bool,

    /// Required: Admin Hub host (control plane).
    #[serde(default)]
    pub hub_host: Option<String>,
    #[serde(default = "default_hub_control_port")]
    pub hub_control_port: u16,
    #[serde(default = "default_hub_data_port")]
    pub hub_data_port: u16,
    #[serde(default)]
    pub hub_token: Option<String>,
    #[serde(default)]
    pub hub_server_id: Option<String>,

    /// TCP data plane: edge dials here for direct tunnels (skip Hub :7101).
    #[serde(default = "default_data_port")]
    pub data_port: u16,
    /// Host (or `host:port`) advertised to edges for direct dial.
    /// If only a host is given, `data_port` is appended.
    #[serde(default)]
    pub data_advertise: Option<String>,
    /// TCP_NODELAY on tunnel sockets (true = lower latency for small packets).
    #[serde(default = "default_tcp_nodelay")]
    pub tcp_nodelay: bool,
    /// SO_RCVBUF / SO_SNDBUF hint in bytes (0 = leave OS default).
    #[serde(default = "default_tcp_buffer_bytes")]
    pub tcp_buffer_bytes: usize,

    /// Virtual overlay (management mesh). Server is an equal Overlay Server.
    #[serde(default)]
    pub overlay: p2p_overlay::OverlayConfig,

    // ---- legacy fields (ignored; kept so old toml still loads) ----
    #[serde(default)]
    pub control_port: Option<u16>,
    #[serde(default)]
    pub data_quic_port: Option<u16>,
    #[serde(default)]
    pub data_kcp_port: Option<u16>,
    #[serde(default)]
    pub data_transport: Option<TransportKind>,
    #[serde(default)]
    pub data_transports: Vec<TransportKind>,
}

fn default_listen() -> String {
    "0.0.0.0".into()
}
fn default_gateway_port() -> u16 {
    8080
}
fn default_gateway_quic_port() -> u16 {
    8443
}
fn default_gateway_kcp_port() -> u16 {
    8444
}
fn default_gateway_transports() -> Vec<TransportKind> {
    vec![TransportKind::Tcp, TransportKind::Quic, TransportKind::Kcp]
}
fn default_api_port() -> u16 {
    3000
}
fn default_db() -> String {
    "sqlite://data/p2p.db".into()
}
fn default_jwt_secret() -> String {
    "change-me-in-production-p2p-network".into()
}
fn default_jwt_ttl() -> i64 {
    86400
}
fn default_admin_user() -> String {
    "admin".into()
}
fn default_admin_pass() -> String {
    "admin123".into()
}
fn default_hub_control_port() -> u16 {
    7100
}
fn default_hub_data_port() -> u16 {
    7101
}
fn default_data_port() -> u16 {
    7001
}
fn default_tcp_nodelay() -> bool {
    true
}
fn default_tcp_buffer_bytes() -> usize {
    p2p_transport::DEFAULT_TCP_BUFFER_BYTES
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            gateway_port: default_gateway_port(),
            gateway_quic_port: default_gateway_quic_port(),
            gateway_kcp_port: default_gateway_kcp_port(),
            gateway_transports: default_gateway_transports(),
            api_port: default_api_port(),
            database_url: default_db(),
            jwt_secret: default_jwt_secret(),
            jwt_ttl_secs: default_jwt_ttl(),
            admin_user: default_admin_user(),
            admin_password: default_admin_pass(),
            max_connections: 100_000,
            enable_relay: true,
            enable_p2p: true,
            hub_host: None,
            hub_control_port: default_hub_control_port(),
            hub_data_port: default_hub_data_port(),
            hub_token: None,
            hub_server_id: None,
            data_port: default_data_port(),
            data_advertise: None,
            tcp_nodelay: default_tcp_nodelay(),
            tcp_buffer_bytes: default_tcp_buffer_bytes(),
            overlay: {
                let mut o = p2p_overlay::OverlayConfig::default();
                o.enabled = true;
                o.role = p2p_overlay::OverlayRole::Server;
                o.node_id = "default".into();
                o.fixed_vip = Some("10.88.0.2".into());
                o.bootstrap = vec!["127.0.0.1:51820".into()];
                o
            },
            control_port: None,
            data_quic_port: None,
            data_kcp_port: None,
            data_transport: None,
            data_transports: vec![],
        }
    }
}

impl ServerConfig {
    pub fn default_path() -> PathBuf {
        p2p_common::default_config_beside_exe("server.toml")
    }

    pub fn enabled_gateway_transports(&self) -> Vec<TransportKind> {
        if self.gateway_transports.is_empty() {
            default_gateway_transports()
        } else {
            self.gateway_transports.clone()
        }
    }

    pub fn gateway_port_for(&self, t: TransportKind) -> u16 {
        match t {
            TransportKind::Tcp => self.gateway_port,
            TransportKind::Quic => self.gateway_quic_port,
            TransportKind::Kcp => self.gateway_kcp_port,
        }
    }

    /// Endpoint string advertised to edges (`host:port`), if configured.
    /// When unset and hub is loopback, defaults to `127.0.0.1:data_port`.
    pub fn advertised_data_endpoint(&self) -> Option<String> {
        if let Some(raw) = self
            .data_advertise
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            return if raw.contains(':') {
                Some(raw.to_string())
            } else {
                Some(format!("{}:{}", raw, self.data_port))
            };
        }
        if let Some(hub) = self.hub_host.as_ref() {
            let h = hub.trim();
            if h == "127.0.0.1" || h == "localhost" || h == "::1" {
                return Some(format!("127.0.0.1:{}", self.data_port));
            }
        }
        None
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            return Ok(p2p_common::load_toml_config(path)?);
        }

        let cfg = Self::default();
        let body = toml::to_string_pretty(&cfg)?;
        let content = format!(
            "# NexusGate Server 节点配置（首次运行自动生成）\n\
             # 只开放穿透映射口；管控出站连接 Admin Hub（hub_host 必填）\n\
             # gateway_*: 访客入口 tcp/quic/kcp\n\
             #\n\
             {body}"
        );
        p2p_common::write_config_file(path, &content)?;
        tracing::warn!(
            path = %path.display(),
            "config not found, wrote defaults and continuing"
        );
        Ok(cfg)
    }

    pub fn ensure_db_parent(&self) -> anyhow::Result<()> {
        if let Some(rest) = self.database_url.strip_prefix("sqlite://") {
            let path = PathBuf::from(rest);
            p2p_common::ensure_parent_dir(&path)?;
        }
        Ok(())
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let body = toml::to_string_pretty(self)?;
        let content = format!(
            "# NexusGate Server 节点配置\n\
             # 穿透映射口对外开放；管理 API 仅本机；管控走 Admin Hub。\n\
             #\n\
             {body}"
        );
        p2p_common::write_config_file(path, &content)?;
        Ok(())
    }
}
