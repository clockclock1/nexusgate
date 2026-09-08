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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            control_port: default_control_port(),
            data_port: default_data_port(),
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
        }
    }
}

impl ServerConfig {
    pub fn default_path() -> PathBuf {
        p2p_common::default_config_beside_exe("server.toml")
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
             # 请按需修改端口、管理员账号与 jwt_secret。\n\
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
}
