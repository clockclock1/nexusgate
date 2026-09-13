use p2p_common::TransportKind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    pub node_id: String,
    pub token: String,
    #[serde(default = "default_server")]
    pub server: String,
    #[serde(default = "default_control_port")]
    pub control_port: u16,
    #[serde(default = "default_data_port")]
    pub data_port: u16,
    #[serde(default = "default_data_quic_port")]
    pub data_quic_port: u16,
    #[serde(default = "default_data_kcp_port")]
    pub data_kcp_port: u16,
    #[serde(default)]
    pub data_transport: TransportKind,
    #[serde(default)]
    pub transports: Vec<TransportKind>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub services: Vec<LocalService>,

    #[serde(default)]
    pub hub_host: Option<String>,
    #[serde(default = "default_hub_control_port")]
    pub hub_control_port: u16,
    #[serde(default = "default_hub_data_port")]
    pub hub_data_port: u16,
    #[serde(default)]
    pub hub_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalService {
    pub service_id: String,
    pub name: String,
    #[serde(default = "default_proto")]
    pub protocol: String,
    pub local_addr: String,
}

fn default_server() -> String {
    "127.0.0.1".into()
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
fn default_proto() -> String {
    "tcp".into()
}
fn default_hub_control_port() -> u16 {
    7100
}
fn default_hub_data_port() -> u16 {
    7101
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            node_id: "edge-demo".into(),
            token: "REPLACE_WITH_TOKEN_FROM_ADMIN".into(),
            server: default_server(),
            control_port: default_control_port(),
            data_port: default_data_port(),
            data_quic_port: default_data_quic_port(),
            data_kcp_port: default_data_kcp_port(),
            data_transport: TransportKind::Tcp,
            transports: vec![TransportKind::Tcp, TransportKind::Quic, TransportKind::Kcp],
            name: Some("demo-edge".into()),
            services: vec![LocalService {
                service_id: "web".into(),
                name: "local-web".into(),
                protocol: default_proto(),
                local_addr: "127.0.0.1:8000".into(),
            }],
            hub_host: None,
            hub_control_port: default_hub_control_port(),
            hub_data_port: default_hub_data_port(),
            hub_token: None,
        }
    }
}

impl EdgeConfig {
    pub fn default_path() -> PathBuf {
        p2p_common::default_config_beside_exe("edge.toml")
    }

    pub fn advertised_transports(&self) -> Vec<TransportKind> {
        if self.transports.is_empty() {
            vec![TransportKind::Tcp, TransportKind::Quic, TransportKind::Kcp]
        } else {
            self.transports.clone()
        }
    }

    pub fn port_for_transport(&self, t: TransportKind) -> u16 {
        match t {
            TransportKind::Tcp => self.data_port,
            TransportKind::Quic => self.data_quic_port,
            TransportKind::Kcp => self.data_kcp_port,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(p2p_common::load_toml_config(path)?)
    }

    pub fn load_or_init(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            return Self::load(path);
        }

        let cfg = Self::default();
        let body = toml::to_string_pretty(&cfg)?;
        let content = format!(
            "# NexusGate Edge 配置文件（首次运行自动生成）\n\
             # data_transport / transports: tcp | quic | kcp\n\
             # Hub 字段必须写在 [[services]] 之前。\n\
             #\n\
             {body}"
        );
        p2p_common::write_config_file(path, &content)?;
        anyhow::bail!(
            "已生成默认配置文件: {}\n请编辑 node_id / token / server 后重新运行。",
            path.display()
        );
    }
}
