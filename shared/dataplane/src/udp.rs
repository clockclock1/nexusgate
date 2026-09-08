//! UDP session / NAT mapping scaffold for future UDP proxying.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpSessionKey {
    pub connection_id: String,
    pub client_addr: String,
}

#[derive(Debug, Clone)]
pub struct UdpSession {
    pub connection_id: String,
    pub client_addr: SocketAddr,
    pub upstream_addr: SocketAddr,
    pub last_active: Instant,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

/// In-memory NAT-style UDP session table (scaffold).
#[derive(Debug, Default, Clone)]
pub struct UdpSessionTable {
    inner: Arc<DashMap<String, UdpSession>>,
}

impl UdpSessionTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&self, session: UdpSession) {
        debug!(
            connection_id = %session.connection_id,
            client = %session.client_addr,
            "udp session upsert"
        );
        self.inner
            .insert(session.connection_id.clone(), session);
    }

    pub fn get(&self, connection_id: &str) -> Option<UdpSession> {
        self.inner.get(connection_id).map(|e| e.clone())
    }

    pub fn remove(&self, connection_id: &str) -> Option<UdpSession> {
        self.inner.remove(connection_id).map(|(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn purge_idle(&self, max_idle: Duration) -> usize {
        let now = Instant::now();
        let mut removed = 0;
        self.inner.retain(|_, s| {
            if now.duration_since(s.last_active) > max_idle {
                removed += 1;
                false
            } else {
                true
            }
        });
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_table() {
        let table = UdpSessionTable::new();
        table.upsert(UdpSession {
            connection_id: "c1".into(),
            client_addr: "127.0.0.1:1".parse().unwrap(),
            upstream_addr: "127.0.0.1:2".parse().unwrap(),
            last_active: Instant::now(),
            bytes_in: 0,
            bytes_out: 0,
        });
        assert_eq!(table.len(), 1);
        assert!(table.get("c1").is_some());
        table.remove("c1");
        assert!(table.is_empty());
    }
}
