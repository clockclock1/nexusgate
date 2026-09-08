//! Route matching for TCP port, HTTP Host, and HTTPS SNI.

use dashmap::DashMap;
use p2p_common::{ProtocolKind, RouteRule};
use std::sync::Arc;

#[derive(Debug, Default, Clone)]
pub struct RouteTable {
    inner: Arc<DashMap<String, RouteRule>>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&self, rule: RouteRule) {
        self.inner.insert(rule.id.clone(), rule);
    }

    pub fn remove(&self, id: &str) -> Option<RouteRule> {
        self.inner.remove(id).map(|(_, v)| v)
    }

    pub fn get(&self, id: &str) -> Option<RouteRule> {
        self.inner.get(id).map(|v| v.clone())
    }

    pub fn list(&self) -> Vec<RouteRule> {
        let mut items: Vec<_> = self.inner.iter().map(|e| e.value().clone()).collect();
        items.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
        items
    }

    pub fn clear(&self) {
        self.inner.clear();
    }

    /// Match by listen port (TCP/UDP gateway).
    pub fn match_port(&self, protocol: ProtocolKind, port: u16) -> Option<RouteRule> {
        self.list()
            .into_iter()
            .find(|r| r.enabled && r.protocol == protocol && r.listen_port == Some(port))
    }

    /// Match HTTP Host header.
    pub fn match_host(&self, host: &str) -> Option<RouteRule> {
        let host = host.split(':').next().unwrap_or(host).to_lowercase();
        self.list().into_iter().find(|r| {
            r.enabled
                && matches!(r.protocol, ProtocolKind::Http | ProtocolKind::Https)
                && r.host
                    .as_ref()
                    .map(|h| h.eq_ignore_ascii_case(&host))
                    .unwrap_or(false)
        })
    }

    /// Match HTTPS SNI.
    pub fn match_sni(&self, sni: &str) -> Option<RouteRule> {
        let sni = sni.to_lowercase();
        self.list().into_iter().find(|r| {
            r.enabled
                && r.protocol == ProtocolKind::Https
                && r.sni
                    .as_ref()
                    .map(|s| s.eq_ignore_ascii_case(&sni))
                    .unwrap_or(false)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p2p_common::NodeId;

    fn rule(id: &str, port: u16) -> RouteRule {
        RouteRule {
            id: id.into(),
            protocol: ProtocolKind::Tcp,
            listen_port: Some(port),
            host: None,
            sni: None,
            node_id: NodeId::new("n1"),
            service_id: "svc".into(),
            local_addr: "127.0.0.1:8080".into(),
            enabled: true,
            priority: 0,
        }
    }

    #[test]
    fn match_tcp_port() {
        let t = RouteTable::new();
        t.upsert(rule("r1", 9000));
        assert!(t.match_port(ProtocolKind::Tcp, 9000).is_some());
        assert!(t.match_port(ProtocolKind::Tcp, 9001).is_none());
    }

    #[test]
    fn match_http_host() {
        let t = RouteTable::new();
        let mut r = rule("r2", 80);
        r.protocol = ProtocolKind::Http;
        r.host = Some("example.com".into());
        t.upsert(r);
        assert!(t.match_host("example.com:80").is_some());
        assert!(t.match_host("other.com").is_none());
    }
}
