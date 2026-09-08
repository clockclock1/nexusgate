use async_trait::async_trait;
use p2p_common::{Error, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

/// Generic bidirectional stream used by the data plane.
pub trait IoStream: AsyncRead + AsyncWrite + Send + Unpin + 'static {}
impl<T> IoStream for T where T: AsyncRead + AsyncWrite + Send + Unpin + 'static {}

/// Listener trait — TCP today, QUIC listener later.
#[async_trait]
pub trait TransportListener: Send + Sync {
    type Stream: IoStream;

    async fn accept(&self) -> Result<(Self::Stream, SocketAddr)>;
    fn local_addr(&self) -> Result<SocketAddr>;
}

/// Connector trait — TCP today, QUIC/TLS later.
#[async_trait]
pub trait TransportConnector: Send + Sync {
    type Stream: IoStream;

    async fn connect(&self, addr: &str) -> Result<Self::Stream>;
}

/// Placeholder for future QUIC transport.
#[derive(Debug, Default)]
pub struct QuicTransportHooks;

impl QuicTransportHooks {
    pub fn is_available(&self) -> bool {
        false
    }

    pub fn note(&self) -> &'static str {
        "QUIC transport is reserved for a future phase"
    }
}

/// Placeholder for TLS wrapping.
#[derive(Debug, Default)]
pub struct TlsTransportHooks;

impl TlsTransportHooks {
    pub fn is_available(&self) -> bool {
        false
    }

    pub fn note(&self) -> &'static str {
        "TLS wrapping is reserved for a future phase"
    }
}

/// Map IO errors into crate Error.
pub fn map_io(e: std::io::Error) -> Error {
    Error::Io(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hooks_unavailable() {
        assert!(!QuicTransportHooks.is_available());
        assert!(!TlsTransportHooks.is_available());
    }
}
