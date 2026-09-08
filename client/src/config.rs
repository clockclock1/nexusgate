use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub services: Vec<LocalService>,
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
fn default_proto() -> String {
    "tcp".into()
}

impl EdgeConfig {
    pub fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        Ok(p2p_common::load_toml_config(path)?)
    }
}
