use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    /// Host:port of p2p-server management API (HTTP).
    #[serde(default = "default_api_upstream")]
    pub api_upstream: String,
}

fn default_listen() -> String {
    "0.0.0.0".into()
}
fn default_listen_port() -> u16 {
    8088
}
fn default_api_upstream() -> String {
    "127.0.0.1:3000".into()
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            listen_port: default_listen_port(),
            api_upstream: default_api_upstream(),
        }
    }
}

impl AdminConfig {
    pub fn default_path() -> PathBuf {
        p2p_common::default_config_beside_exe("admin.toml")
    }

    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            return Ok(toml::from_str(&text)?);
        }

        let cfg = Self::default();
        p2p_common::write_config_file(
            path,
            &format!(
                "# NexusGate Admin Panel（首次运行自动生成）\n\
                 # listen_port: 面板端口\n\
                 # api_upstream: 服务端管理 API 地址 host:port\n\
                 #\n\
                 {}",
                toml::to_string_pretty(&cfg)?
            ),
        )?;
        tracing::warn!(path = %path.display(), "config not found, wrote defaults");
        Ok(cfg)
    }

    pub fn bind_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.listen, self.listen_port).parse()?)
    }

    pub fn http_upstream_base(&self) -> String {
        format!("http://{}", self.api_upstream.trim_end_matches('/'))
    }

    pub fn ws_upstream_base(&self) -> String {
        format!("ws://{}", self.api_upstream.trim_end_matches('/'))
    }
}
