use p2p_common::{PathKind, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathProbeRequest {
    pub connection_id: String,
    pub path: PathKind,
    pub probe_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathProbeResult {
    pub connection_id: String,
    pub path: PathKind,
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub probe_id: String,
}

/// Probe a candidate path. Relay is always "available"; P2P probes the scaffold.
pub async fn probe_path(req: PathProbeRequest) -> Result<PathProbeResult> {
    let start = Instant::now();
    let success = match req.path {
        PathKind::Relay => true,
        PathKind::P2p => {
            debug!("p2p path probe scaffold");
            false
        }
    };
    // Simulate minimal work
    tokio::time::sleep(Duration::from_millis(1)).await;
    Ok(PathProbeResult {
        connection_id: req.connection_id,
        path: req.path,
        success,
        latency_ms: Some(start.elapsed().as_millis() as u64),
        probe_id: req.probe_id,
    })
}

/// Choose best path: prefer P2P if probe succeeded, else Relay.
pub fn select_path(p2p_ok: bool, relay_ok: bool) -> PathKind {
    if p2p_ok {
        PathKind::P2p
    } else if relay_ok {
        PathKind::Relay
    } else {
        PathKind::Relay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_relay_ok() {
        let r = probe_path(PathProbeRequest {
            connection_id: "c".into(),
            path: PathKind::Relay,
            probe_id: "p1".into(),
        })
        .await
        .unwrap();
        assert!(r.success);
    }

    #[test]
    fn select_prefers_p2p() {
        assert_eq!(select_path(true, true), PathKind::P2p);
        assert_eq!(select_path(false, true), PathKind::Relay);
    }
}
