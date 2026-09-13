//! Management channel over the overlay mesh (no TUN required).

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Outbound / inbound management HTTP-ish request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgmtRequest {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MgmtResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub error: Option<String>,
}

impl MgmtResponse {
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            status: 502,
            headers: vec![],
            body: vec![],
            error: Some(msg.into()),
        }
    }
}

pub const MGMT_MAX_BODY: usize = 48 * 1024;
pub const MGMT_DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct RegisterServiceEvent {
    pub edge_id: String,
    pub service_id: String,
    pub name: String,
    pub protocol: String,
    pub local_addr: String,
}
