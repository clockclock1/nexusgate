//! KCP reliable-UDP data-plane transport (tokio_kcp).

use p2p_common::{Error, Result};
use std::net::SocketAddr;
use tokio_kcp::{KcpConfig, KcpListener, KcpNoDelayConfig, KcpStream};
use tracing::debug;

fn turbo_config() -> KcpConfig {
    let mut cfg = KcpConfig::default();
    cfg.nodelay = KcpNoDelayConfig::fastest();
    cfg.wnd_size = (1024, 1024);
    cfg.mtu = 1400;
    cfg.flush_write = true;
    cfg.stream = true;
    cfg
}

/// Bind a KCP listener.
pub async fn bind_kcp_server(addr: SocketAddr) -> Result<KcpListener> {
    let listener = KcpListener::bind(turbo_config(), addr)
        .await
        .map_err(|e| Error::internal(format!("kcp bind: {e}")))?;
    debug!(%addr, "kcp data listener bound");
    Ok(listener)
}

/// Accept one KCP stream.
pub async fn accept_kcp(listener: &mut KcpListener) -> Result<(KcpStream, SocketAddr)> {
    listener
        .accept()
        .await
        .map_err(|e| Error::internal(format!("kcp accept: {e}")))
}

/// Dial a KCP server.
pub async fn connect_kcp(addr: SocketAddr) -> Result<KcpStream> {
    KcpStream::connect(&turbo_config(), addr)
        .await
        .map_err(|e| Error::internal(format!("kcp connect: {e}")))
}

/// Convenience: parse host:port and connect.
pub async fn connect_kcp_addr(addr: &str) -> Result<KcpStream> {
    let sock: SocketAddr = addr
        .parse()
        .map_err(|e| Error::config(format!("invalid kcp addr {addr}: {e}")))?;
    connect_kcp(sock).await
}
