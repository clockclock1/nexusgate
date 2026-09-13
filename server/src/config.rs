use p2p_common::TransportKind;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_control_port")]
    pub control_port: u16,
    #[serde(default = "default_data_port")]
    pub data_port: u16,
    /// QUIC data-plane UDP port (used when data_transport=quic).
    #[serde(default = "default_data_quic_port")]
    pub data_quic_port: u16,
    /// KCP data-plane UDP port (used when data_transport=kcp).
    #[serde(default = "default_data_kcp_port")]
    pub data_kcp_port: u16,
    /// Preferred wire transport for edge ↔ server data plane: tcp | quic | kcp.
    #[serde(default)]
    pub data_transport: TransportKind,
    /// Also listen for these transports (in addition to preferred).
    #[serde(default)]
    pub data_transports: Vec<TransportKind>,
    #[serde(default = "default_gateway_port")]
    pub gateway_port: u16,
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
}

fn default_listen() -> String {
    "0.0.0.0".into()
}
fn default_control_port() -> u16 {
    7000
}
fn default_data_port() -> u16 {
    7001
}
fn default_data_quic_port() -> u16 {
    7002
}
fn default_data_kcp_port() -> u16 {
    7003
}
fn default_gateway_port() -> u16 {
    8080
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            control_port: default_control_port(),
            data_port: default_data_port(),
            data_quic_port: default_data_quic_port(),
            data_kcp_port: default_data_kcp_port(),
            data_transport: TransportKind::Tcp,
            data_transports: vec![],
            gateway_port: default_gateway_port(),
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
        }
    }
}

impl ServerConfig {
    pub fn default_path() -> PathBuf {
        p2p_common::default_config_beside_exe("server.toml")
    }

    pub fn enabled_data_transports(&self) -> Vec<TransportKind> {
        let mut out = Vec::new();
        out.push(self.data_transport);
        for t in &self.data_transports {
            if !out.contains(t) {
                out.push(*t);
            }
        }
        if !out.contains(&TransportKind::Tcp) {
            out.push(TransportKind::Tcp);
        }
        out
    }

    pub fn port_for_transport(&self, t: TransportKind) -> u16 {
        match t {
            TransportKind::Tcp => self.data_port,
            TransportKind::Quic => self.data_quic_port,
            TransportKind::Kcp => self.data_kcp_port,
        }
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            return Ok(p2p_common::load_toml_config(path)?);
        }

        let cfg = Self::default();
        let body = toml::to_string_pretty(&cfg)?;
        let content = format!(
            "# NexusGate Server 配置文件（首次运行自动生成）\n\
             # data_transport: tcp | quic | kcp（edge↔server 数据面）\n\
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
            "# NexusGate Server 配置文件\n\
             # 可由管理面板「系统设置」写入；端口等变更通常需重启后生效。\n\
             #\n\
             {body}"
        );
        p2p_common::write_config_file(path, &content)?;
        Ok(())
    }
}
