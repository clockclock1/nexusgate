use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    /// host:port of that Super Node management API (HTTP fallback)
    pub api_upstream: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default = "default_listen")]
    pub listen: String,
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    /// Legacy / fallback upstream (also used to seed [[servers]] when empty).
    #[serde(default = "default_api_upstream")]
    pub api_upstream: String,
    #[serde(default)]
    pub default_server: Option<String>,
    #[serde(default)]
    pub servers: Vec<ServerEntry>,

    /// Enable Admin Hub (P2P/control mesh for servers + edges).
    #[serde(default = "default_true")]
    pub hub_enabled: bool,
    #[serde(default = "default_listen")]
    pub hub_listen: String,
    #[serde(default = "default_hub_control_port")]
    pub hub_control_port: u16,
    #[serde(default = "default_hub_data_port")]
    pub hub_data_port: u16,
    /// Shared token for servers/edges dialing into the Hub.
    #[serde(default = "default_hub_token")]
    pub hub_token: String,
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
fn default_hub_control_port() -> u16 {
    7100
}
fn default_hub_data_port() -> u16 {
    7101
}
fn default_hub_token() -> String {
    "change-me-hub-token".into()
}
fn default_true() -> bool {
    true
}

impl Default for AdminConfig {
    fn default() -> Self {
        let upstream = default_api_upstream();
        Self {
            listen: default_listen(),
            listen_port: default_listen_port(),
            api_upstream: upstream.clone(),
            default_server: Some("default".into()),
            servers: vec![ServerEntry {
                id: "default".into(),
                name: "默认服务端".into(),
                api_upstream: upstream,
            }],
            hub_enabled: true,
            hub_listen: default_listen(),
            hub_control_port: default_hub_control_port(),
            hub_data_port: default_hub_data_port(),
            hub_token: default_hub_token(),
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
            let mut cfg: Self = toml::from_str(&text)?;
            cfg.normalize();
            return Ok(cfg);
        }

        let cfg = Self::default();
        cfg.save(path)?;
        tracing::warn!(path = %path.display(), "config not found, wrote defaults");
        Ok(cfg)
    }

    pub fn normalize(&mut self) {
        if self.servers.is_empty() {
            self.servers.push(ServerEntry {
                id: "default".into(),
                name: "默认服务端".into(),
                api_upstream: self.api_upstream.clone(),
            });
        }
        if self.default_server.is_none()
            || !self
                .servers
                .iter()
                .any(|s| Some(&s.id) == self.default_server.as_ref())
        {
            self.default_server = self.servers.first().map(|s| s.id.clone());
        }
        if let Some(def) = self.default_server.clone() {
            if let Some(s) = self.servers.iter().find(|s| s.id == def) {
                self.api_upstream = s.api_upstream.clone();
            }
        }
        if self.hub_token.trim().is_empty() {
            self.hub_token = default_hub_token();
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let body = toml::to_string_pretty(self)?;
        let content = format!(
            "# NexusGate Admin Panel + Hub\n\
             # listen / listen_port: Web 面板\n\
             # hub_*: P2P/中转管理中枢（服务端与客户端 dial 进来）\n\
             # [[servers]]: HTTP 反代回退地址\n\
             #\n\
             {body}"
        );
        p2p_common::write_config_file(path.as_ref(), &content)?;
        Ok(())
    }

    pub fn bind_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.listen, self.listen_port).parse()?)
    }

    pub fn hub_control_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.hub_listen, self.hub_control_port).parse()?)
    }

    pub fn hub_data_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.hub_listen, self.hub_data_port).parse()?)
    }

    pub fn find_server(&self, id: &str) -> Option<&ServerEntry> {
        self.servers.iter().find(|s| s.id == id)
    }

    pub fn resolve_upstream(&self, server_id: Option<&str>) -> anyhow::Result<&ServerEntry> {
        if let Some(id) = server_id {
            if !id.is_empty() {
                return self
                    .find_server(id)
                    .ok_or_else(|| anyhow::anyhow!("unknown server id: {id}"));
            }
        }
        let def = self
            .default_server
            .as_deref()
            .or_else(|| self.servers.first().map(|s| s.id.as_str()))
            .ok_or_else(|| anyhow::anyhow!("no servers configured"))?;
        self.find_server(def)
            .ok_or_else(|| anyhow::anyhow!("default server missing"))
    }
}

pub fn http_base(upstream: &str) -> String {
    let u = upstream.trim().trim_end_matches('/');
    if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else {
        format!("http://{u}")
    }
}

pub fn ws_base(upstream: &str) -> String {
    let u = upstream.trim().trim_end_matches('/');
    if let Some(rest) = u.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = u.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if let Some(rest) = u.strip_prefix("wss://") {
        format!("wss://{rest}")
    } else if let Some(rest) = u.strip_prefix("ws://") {
        format!("ws://{rest}")
    } else {
        format!("ws://{u}")
    }
}
