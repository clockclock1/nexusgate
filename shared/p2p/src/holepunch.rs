use p2p_common::{PeerCandidate, Result};
use tracing::debug;

/// Hole-punch attempt result (scaffold — real UDP punch in later phase).
#[derive(Debug, Clone)]
pub struct HolePunchResult {
    pub success: bool,
    pub selected: Option<PeerCandidate>,
    pub detail: String,
}

/// Attempt coordinated hole punch between local and remote candidates.
pub async fn attempt_hole_punch(
    local: &[PeerCandidate],
    remote: &[PeerCandidate],
) -> Result<HolePunchResult> {
    debug!(
        local = local.len(),
        remote = remote.len(),
        "hole punch scaffold invoked"
    );
    // Phase 1-7 MVP: scaffold only; always report not established so relay is used.
    Ok(HolePunchResult {
        success: false,
        selected: remote.first().cloned(),
        detail: "P2P hole punch not yet implemented; use relay".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn punch_scaffold() {
        let remote = vec![PeerCandidate {
            addr: "1.2.3.4:5".into(),
            priority: 1,
            protocol: "udp".into(),
        }];
        let r = attempt_hole_punch(&[], &remote).await.unwrap();
        assert!(!r.success);
    }
}
