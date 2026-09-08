//! Lightweight in-memory counters for nodes / connections / traffic.

use parking_lot::Mutex;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct Metrics {
    pub nodes_online: AtomicU64,
    pub connections_active: AtomicU64,
    pub connections_total: AtomicU64,
    pub p2p_connections: AtomicU64,
    pub relay_connections: AtomicU64,
    pub rx_bytes: AtomicU64,
    pub tx_bytes: AtomicU64,
    pub tcp_connections: AtomicU64,
    pub udp_sessions: AtomicU64,
    history: Mutex<Vec<TrafficSample>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrafficSample {
    pub ts: i64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub connections: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSnapshot {
    pub nodes_online: u64,
    pub connections_active: u64,
    pub connections_total: u64,
    pub p2p_connections: u64,
    pub relay_connections: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub tcp_connections: u64,
    pub udp_sessions: u64,
    pub p2p_rate: f64,
    pub relay_rate: f64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn add_rx(&self, n: u64) {
        self.rx_bytes.fetch_add(n, Ordering::Relaxed);
    }

    pub fn add_tx(&self, n: u64) {
        self.tx_bytes.fetch_add(n, Ordering::Relaxed);
    }

    pub fn conn_opened(&self, p2p: bool) {
        self.connections_active.fetch_add(1, Ordering::Relaxed);
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.tcp_connections.fetch_add(1, Ordering::Relaxed);
        if p2p {
            self.p2p_connections.fetch_add(1, Ordering::Relaxed);
        } else {
            self.relay_connections.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn conn_closed(&self) {
        self.connections_active.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn set_nodes_online(&self, n: u64) {
        self.nodes_online.store(n, Ordering::Relaxed);
    }

    pub fn sample(&self) {
        let sample = TrafficSample {
            ts: chrono_now(),
            rx_bytes: self.rx_bytes.load(Ordering::Relaxed),
            tx_bytes: self.tx_bytes.load(Ordering::Relaxed),
            connections: self.connections_active.load(Ordering::Relaxed),
        };
        let mut h = self.history.lock();
        h.push(sample);
        if h.len() > 720 {
            let drain = h.len() - 720;
            h.drain(0..drain);
        }
    }

    pub fn history(&self) -> Vec<TrafficSample> {
        self.history.lock().clone()
    }

    pub fn snapshot(&self) -> DashboardSnapshot {
        let p2p = self.p2p_connections.load(Ordering::Relaxed) as f64;
        let relay = self.relay_connections.load(Ordering::Relaxed) as f64;
        let total = (p2p + relay).max(1.0);
        DashboardSnapshot {
            nodes_online: self.nodes_online.load(Ordering::Relaxed),
            connections_active: self.connections_active.load(Ordering::Relaxed),
            connections_total: self.connections_total.load(Ordering::Relaxed),
            p2p_connections: self.p2p_connections.load(Ordering::Relaxed),
            relay_connections: self.relay_connections.load(Ordering::Relaxed),
            rx_bytes: self.rx_bytes.load(Ordering::Relaxed),
            tx_bytes: self.tx_bytes.load(Ordering::Relaxed),
            tcp_connections: self.tcp_connections.load(Ordering::Relaxed),
            udp_sessions: self.udp_sessions.load(Ordering::Relaxed),
            p2p_rate: p2p / total,
            relay_rate: relay / total,
        }
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters() {
        let m = Metrics::new();
        m.conn_opened(false);
        m.add_rx(100);
        let s = m.snapshot();
        assert_eq!(s.connections_active, 1);
        assert_eq!(s.rx_bytes, 100);
        assert!(s.relay_rate > 0.0);
    }
}
