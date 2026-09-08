use p2p_common::PeerCandidate;
use serde::{Deserialize, Serialize};

/// Gather local candidates for ICE-like negotiation (scaffold).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CandidateGatherer {
    pub local_addrs: Vec<String>,
}

impl CandidateGatherer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_local(&mut self, addr: impl Into<String>) {
        self.local_addrs.push(addr.into());
    }

    pub fn gather(&self) -> Vec<PeerCandidate> {
        self.local_addrs
            .iter()
            .enumerate()
            .map(|(i, addr)| PeerCandidate {
                addr: addr.clone(),
                priority: (1000 - i as u32).max(1),
                protocol: "udp".into(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_candidates() {
        let mut g = CandidateGatherer::new();
        g.add_local("192.168.1.10:40000");
        let c = g.gather();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].addr, "192.168.1.10:40000");
    }
}
