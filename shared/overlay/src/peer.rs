//! Peer table with endpoint candidates + direct-path health.

use crate::config::OverlayRole;
use dashmap::DashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PeerEntry {
    pub node_id: String,
    pub vip: Ipv4Addr,
    pub role: OverlayRole,
    /// Best known working UDP endpoint (prefer confirmed direct).
    pub endpoint: Option<SocketAddr>,
    /// Candidate UDP endpoints (observed / reflexive / self-reported).
    pub candidates: Vec<SocketAddr>,
    /// True after recent successful PunchAck / live Packet from this addr.
    pub direct_ok: bool,
    pub last_seen: Instant,
    pub last_punch: Instant,
}

impl PeerEntry {
    pub fn add_candidate(&mut self, addr: SocketAddr) {
        if is_unusable(addr) {
            return;
        }
        if !self.candidates.iter().any(|c| *c == addr) {
            self.candidates.push(addr);
        }
        if self.endpoint.is_none() {
            self.endpoint = Some(addr);
        }
    }

    pub fn mark_direct(&mut self, addr: SocketAddr) {
        self.add_candidate(addr);
        self.endpoint = Some(addr);
        self.direct_ok = true;
        self.last_seen = Instant::now();
        self.last_punch = Instant::now();
    }

    pub fn all_targets(&self) -> Vec<SocketAddr> {
        let mut out = Vec::new();
        if let Some(ep) = self.endpoint {
            out.push(ep);
        }
        for c in &self.candidates {
            if !out.contains(c) {
                out.push(*c);
            }
        }
        out
    }
}

fn is_unusable(addr: SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(v4) => {
            v4.is_unspecified() || v4.is_broadcast() || v4.is_multicast()
        }
        std::net::IpAddr::V6(v6) => v6.is_unspecified() || v6.is_multicast(),
    }
}

#[derive(Clone, Default)]
pub struct PeerTable {
    by_id: Arc<DashMap<String, PeerEntry>>,
    by_vip: Arc<DashMap<Ipv4Addr, String>>,
}

impl PeerTable {
    pub fn upsert(&self, entry: PeerEntry) {
        self.by_vip.insert(entry.vip, entry.node_id.clone());
        self.by_id.insert(entry.node_id.clone(), entry);
    }

    pub fn get_by_id(&self, id: &str) -> Option<PeerEntry> {
        self.by_id.get(id).map(|e| e.clone())
    }

    pub fn get_by_vip(&self, vip: Ipv4Addr) -> Option<PeerEntry> {
        let id = self.by_vip.get(&vip)?.clone();
        self.get_by_id(&id)
    }

    pub fn remove(&self, id: &str) {
        if let Some((_, e)) = self.by_id.remove(id) {
            self.by_vip.remove(&e.vip);
        }
    }

    pub fn list(&self) -> Vec<PeerEntry> {
        self.by_id.iter().map(|e| e.value().clone()).collect()
    }

    pub fn touch(&self, id: &str, endpoint: Option<SocketAddr>) {
        if let Some(mut e) = self.by_id.get_mut(id) {
            e.last_seen = Instant::now();
            if let Some(ep) = endpoint {
                e.add_candidate(ep);
            }
        }
    }

    /// Observe traffic from `from` for peer `id` (or create thin entry).
    pub fn observe(&self, id: &str, from: SocketAddr, vip: Option<Ipv4Addr>, role: OverlayRole) {
        if let Some(mut e) = self.by_id.get_mut(id) {
            e.add_candidate(from);
            e.last_seen = Instant::now();
            // Live traffic is a strong signal the path works.
            if e.endpoint == Some(from) || !e.direct_ok {
                e.endpoint = Some(from);
                e.direct_ok = true;
            }
            return;
        }
        if let Some(vip) = vip {
            let mut e = PeerEntry {
                node_id: id.to_string(),
                vip,
                role,
                endpoint: None,
                candidates: vec![],
                direct_ok: false,
                last_seen: Instant::now(),
                last_punch: Instant::now()
                    .checked_sub(Duration::from_secs(60))
                    .unwrap_or_else(Instant::now),
            };
            e.mark_direct(from);
            self.upsert(e);
        }
    }

    pub fn mark_direct(&self, id: &str, addr: SocketAddr) {
        if let Some(mut e) = self.by_id.get_mut(id) {
            e.mark_direct(addr);
        }
    }

    pub fn add_candidates(&self, id: &str, addrs: &[SocketAddr]) {
        if let Some(mut e) = self.by_id.get_mut(id) {
            for a in addrs {
                e.add_candidate(*a);
            }
        }
    }

    pub fn stale_ids(&self, max_age: Duration) -> Vec<String> {
        self.by_id
            .iter()
            .filter(|e| e.last_seen.elapsed() > max_age)
            .map(|e| e.key().clone())
            .collect()
    }

    /// Peers that need another punch burst.
    pub fn needs_punch(&self, self_id: &str, min_interval: Duration) -> Vec<PeerEntry> {
        self.by_id
            .iter()
            .filter(|e| e.node_id != self_id)
            .filter(|e| !e.direct_ok || e.last_punch.elapsed() > Duration::from_secs(25))
            .filter(|e| e.last_punch.elapsed() >= min_interval)
            .filter(|e| !e.all_targets().is_empty())
            .map(|e| e.value().clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_dedupe_and_direct() {
        let mut e = PeerEntry {
            node_id: "a".into(),
            vip: "10.88.0.1".parse().unwrap(),
            role: OverlayRole::Server,
            endpoint: None,
            candidates: vec![],
            direct_ok: false,
            last_seen: Instant::now(),
            last_punch: Instant::now(),
        };
        let a: SocketAddr = "1.2.3.4:51820".parse().unwrap();
        e.add_candidate(a);
        e.add_candidate(a);
        assert_eq!(e.candidates.len(), 1);
        e.mark_direct(a);
        assert!(e.direct_ok);
        assert_eq!(e.endpoint, Some(a));
    }
}
