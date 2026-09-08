//! Relay path scaffold: server-mediated 1:1 data bridging.

use p2p_common::{ConnectionId, Result};
use tracing::debug;

/// Metadata for an active relay bridge (PublicConn ↔ DataConn).
#[derive(Debug, Clone)]
pub struct RelayBridge {
    pub connection_id: ConnectionId,
    pub node_id: String,
    pub protocol: String,
}

impl RelayBridge {
    pub fn new(connection_id: ConnectionId, node_id: impl Into<String>, protocol: impl Into<String>) -> Self {
        Self {
            connection_id,
            node_id: node_id.into(),
            protocol: protocol.into(),
        }
    }
}

/// Relay manager scaffold — actual bridging lives in the server gateway.
#[derive(Debug, Default)]
pub struct RelayManager {
    active: usize,
}

impl RelayManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&mut self, bridge: &RelayBridge) {
        self.active += 1;
        debug!(
            connection_id = %bridge.connection_id,
            node = %bridge.node_id,
            "relay bridge begin"
        );
    }

    pub fn end(&mut self, connection_id: &ConnectionId) {
        self.active = self.active.saturating_sub(1);
        debug!(%connection_id, "relay bridge end");
    }

    pub fn active_count(&self) -> usize {
        self.active
    }
}

pub async fn relay_ready() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_counts() {
        let mut m = RelayManager::new();
        let b = RelayBridge::new(ConnectionId::new("c"), "n1", "tcp");
        m.begin(&b);
        assert_eq!(m.active_count(), 1);
        m.end(&b.connection_id);
        assert_eq!(m.active_count(), 0);
    }
}
