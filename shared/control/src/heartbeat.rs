use std::time::Duration;

/// Heartbeat configuration for control sessions.
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between heartbeat sends.
    pub interval: Duration,
    /// Consider peer dead after this many missed intervals.
    pub timeout: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(15),
            timeout: Duration::from_secs(60),
        }
    }
}

impl HeartbeatConfig {
    pub fn is_expired(&self, since_last: Duration) -> bool {
        since_last > self.timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry() {
        let cfg = HeartbeatConfig::default();
        assert!(!cfg.is_expired(Duration::from_secs(10)));
        assert!(cfg.is_expired(Duration::from_secs(120)));
    }
}
