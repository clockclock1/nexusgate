use crate::config::ServerConfig;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use p2p_metrics::Metrics;
use p2p_protocol::ControlMessage;
use p2p_router::RouteTable;
use p2p_security::JwtManager;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRecord {
    pub node_id: String,
    pub name: String,
    pub status: String,
    pub token_hash: String,
    pub public_ip: Option<String>,
    pub nat_type: Option<String>,
    pub version: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Clone)]
pub struct OnlineNode {
    pub node_id: String,
    pub name: String,
    pub version: String,
    pub public_ip: Option<String>,
    pub nat_type: Option<String>,
    pub connected_at: Instant,
    pub last_seen: Instant,
    pub tx: mpsc::Sender<ControlMessage>,
}

pub struct PendingConnection {
    pub connection_id: String,
    pub data_token: String,
    pub node_id: String,
    pub local_addr: String,
    pub protocol: String,
    pub public: Option<TcpStream>,
    pub data: Option<TcpStream>,
    pub created_at: Instant,
    pub notify: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveConnection {
    pub conn_id: String,
    pub node_id: String,
    pub protocol: String,
    pub mode: String,
    pub status: String,
    pub local_addr: String,
    pub remote_addr: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub node_id: Option<String>,
    pub message: String,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<parking_lot::RwLock<ServerConfig>>,
    pub db: SqlitePool,
    pub jwt: JwtManager,
    pub metrics: Arc<Metrics>,
    pub routes: RouteTable,
    pub online: Arc<DashMap<String, OnlineNode>>,
    pub pending: Arc<DashMap<String, PendingConnection>>,
    pub connections: Arc<DashMap<String, ActiveConnection>>,
    pub logs: Arc<parking_lot::RwLock<Vec<LogEntry>>>,
    pub event_tx: broadcast::Sender<serde_json::Value>,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(config: ServerConfig, db: SqlitePool) -> Self {
        let jwt = JwtManager::new(config.jwt_secret.clone(), config.jwt_ttl_secs);
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            config: Arc::new(parking_lot::RwLock::new(config)),
            db,
            jwt,
            metrics: Metrics::new(),
            routes: RouteTable::new(),
            online: Arc::new(DashMap::new()),
            pending: Arc::new(DashMap::new()),
            connections: Arc::new(DashMap::new()),
            logs: Arc::new(parking_lot::RwLock::new(Vec::new())),
            event_tx,
            started_at: Instant::now(),
        }
    }

    pub fn push_log(&self, level: &str, target: &str, node_id: Option<String>, message: impl Into<String>) {
        let entry = LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level: level.into(),
            target: target.into(),
            node_id,
            message: message.into(),
        };
        {
            let mut logs = self.logs.write();
            logs.push(entry.clone());
            if logs.len() > 2000 {
                let drain = logs.len() - 2000;
                logs.drain(0..drain);
            }
        }
        let _ = self.event_tx.send(serde_json::json!({
            "type": "log",
            "payload": entry,
        }));
    }

    pub fn broadcast(&self, event: serde_json::Value) {
        let _ = self.event_tx.send(event);
    }

    pub fn refresh_online_metric(&self) {
        self.metrics
            .set_nodes_online(self.online.len() as u64);
    }
}
