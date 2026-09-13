//! STUN-like reflexive discovery + bidirectional UDP hole punch helpers.

use crate::frame::{encode_msg, OverlayMsg};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::net::UdpSocket;

static TX: AtomicU64 = AtomicU64::new(1);

pub fn next_tx_id() -> String {
    let n = TX.fetch_add(1, Ordering::Relaxed);
    format!("{:x}-{}", n, std::process::id())
}

pub fn is_private_ip(addr: SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(v4) => {
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unicast_link_local() || v6.is_unique_local() || v6.is_unspecified()
        }
    }
}

/// Prefer a public/reflexive endpoint over a private bind address when advertising.
pub fn prefer_advertise(local: Option<SocketAddr>, reflexive: Option<SocketAddr>) -> Option<SocketAddr> {
    match (reflexive, local) {
        (Some(r), _) if !is_private_ip(r) => Some(r),
        (_, Some(l)) if !is_unspecified(l) => Some(l),
        (Some(r), _) => Some(r),
        (_, Some(l)) => Some(l),
        _ => None,
    }
}

fn is_unspecified(addr: SocketAddr) -> bool {
    match addr.ip() {
        std::net::IpAddr::V4(v) => v.is_unspecified(),
        std::net::IpAddr::V6(v) => v.is_unspecified(),
    }
}

pub async fn send_stun_query(
    sock: &UdpSocket,
    to: SocketAddr,
    node_id: &str,
) -> anyhow::Result<()> {
    let msg = OverlayMsg::StunQuery {
        node_id: node_id.to_string(),
        tx_id: next_tx_id(),
    };
    sock.send_to(&encode_msg(&msg)?, to).await?;
    Ok(())
}

pub async fn send_punch(
    sock: &UdpSocket,
    to: SocketAddr,
    from_id: &str,
    to_id: &str,
) -> anyhow::Result<()> {
    let msg = OverlayMsg::Punch {
        from_id: from_id.to_string(),
        to_id: to_id.to_string(),
        tx_id: next_tx_id(),
    };
    sock.send_to(&encode_msg(&msg)?, to).await?;
    Ok(())
}

pub async fn send_punch_ack(
    sock: &UdpSocket,
    to: SocketAddr,
    from_id: &str,
    to_id: &str,
    tx_id: String,
) -> anyhow::Result<()> {
    let msg = OverlayMsg::PunchAck {
        from_id: from_id.to_string(),
        to_id: to_id.to_string(),
        tx_id,
    };
    sock.send_to(&encode_msg(&msg)?, to).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefer_public_reflexive() {
        let local: SocketAddr = "192.168.1.2:51820".parse().unwrap();
        let reflexive: SocketAddr = "203.0.113.9:51820".parse().unwrap();
        assert_eq!(
            prefer_advertise(Some(local), Some(reflexive)),
            Some(reflexive)
        );
    }
}
