use crate::config::{http_base, ws_base, AdminConfig, ServerEntry};
use anyhow::Context;
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct ServerRegistry {
    inner: Arc<RwLock<AdminConfig>>,
    path: Arc<PathBuf>,
}

impl ServerRegistry {
    pub fn new(cfg: AdminConfig, path: PathBuf) -> Self {
        Self {
            inner: Arc::new(RwLock::new(cfg)),
            path: Arc::new(path),
        }
    }

    pub fn snapshot(&self) -> AdminConfig {
        self.inner.read().clone()
    }

    pub fn list(&self) -> Vec<ServerEntry> {
        self.inner.read().servers.clone()
    }

    pub fn default_server_id(&self) -> Option<String> {
        self.inner.read().default_server.clone()
    }

    pub fn resolve(&self, server_id: Option<&str>) -> anyhow::Result<(String, String)> {
        let entry = self.resolve_entry(server_id)?;
        Ok((http_base(&entry.api_upstream), ws_base(&entry.api_upstream)))
    }

    pub fn resolve_entry(&self, server_id: Option<&str>) -> anyhow::Result<ServerEntry> {
        let cfg = self.inner.read();
        Ok(cfg.resolve_upstream(server_id)?.clone())
    }

    pub fn upsert(&self, entry: ServerEntry, make_default: bool) -> anyhow::Result<Vec<ServerEntry>> {
        if entry.id.trim().is_empty() {
            anyhow::bail!("server id required");
        }
        if entry.api_upstream.trim().is_empty() {
            anyhow::bail!("api_upstream required");
        }
        {
            let mut cfg = self.inner.write();
            if let Some(existing) = cfg.servers.iter_mut().find(|s| s.id == entry.id) {
                *existing = entry.clone();
            } else {
                cfg.servers.push(entry.clone());
            }
            if make_default || cfg.default_server.is_none() {
                cfg.default_server = Some(entry.id.clone());
            }
            cfg.normalize();
            cfg.save(self.path.as_path())
                .context("save admin.toml")?;
        }
        Ok(self.list())
    }

    pub fn remove(&self, id: &str) -> anyhow::Result<Vec<ServerEntry>> {
        {
            let mut cfg = self.inner.write();
            if cfg.servers.len() <= 1 {
                anyhow::bail!("at least one server node is required");
            }
            let before = cfg.servers.len();
            cfg.servers.retain(|s| s.id != id);
            if cfg.servers.len() == before {
                anyhow::bail!("server not found: {id}");
            }
            if cfg.default_server.as_deref() == Some(id) {
                cfg.default_server = cfg.servers.first().map(|s| s.id.clone());
            }
            cfg.normalize();
            cfg.save(self.path.as_path())
                .context("save admin.toml")?;
        }
        Ok(self.list())
    }

    pub fn set_default(&self, id: &str) -> anyhow::Result<Vec<ServerEntry>> {
        {
            let mut cfg = self.inner.write();
            if !cfg.servers.iter().any(|s| s.id == id) {
                anyhow::bail!("server not found: {id}");
            }
            cfg.default_server = Some(id.to_string());
            cfg.normalize();
            cfg.save(self.path.as_path())
                .context("save admin.toml")?;
        }
        Ok(self.list())
    }
}
