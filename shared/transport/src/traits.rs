use async_trait::async_trait;
use p2p_common::{Error, Result};
use std::net::SocketAddr;
use tokio::io::{AsyncRead, AsyncWrite};

/// Generic bidirectional stream used by the data plane.
pub trait IoStream: AsyncRead + AsyncWrite + Send + Unpin + 'static {}
impl<T> IoStream for T where T: AsyncRead + AsyncWrite + Send + Unpin + 'static {}

/// Listener trait — TCP / QUIC / KCP.
#[async_trait]
pub trait TransportListener: Send + Sync {
    type Stream: IoStream;

    async fn accept(&self) -> Result<(Self::Stream, SocketAddr)>;
    fn local_addr(&self) -> Result<SocketAddr>;
}

/// Connector trait — TCP / QUIC / KCP.
#[async_trait]
pub trait TransportConnector: Send + Sync {
    type Stream: IoStream;

    async fn connect(&self, addr: &str) -> Result<Self::Stream>;
}

/// Map IO errors into crate Error.
pub fn map_io(e: std::io::Error) -> Error {
    Error::Io(e)
}

/// Enable TCP_NODELAY when possible (low-latency small packets).
pub fn set_nodelay(stream: &tokio::net::TcpStream, on: bool) {
    let _ = stream.set_nodelay(on);
}

/// Default socket buffer for high-throughput tunnels (2 MiB).
pub const DEFAULT_TCP_BUFFER_BYTES: usize = 2 * 1024 * 1024;

/// Tune a TCP stream: nodelay + optional SO_RCVBUF/SO_SNDBUF.
pub fn tune_tcp(stream: &tokio::net::TcpStream, nodelay: bool, buffer_bytes: usize) {
    set_nodelay(stream, nodelay);
    if buffer_bytes == 0 {
        return;
    }
    let sock = socket2::SockRef::from(stream);
    let _ = sock.set_recv_buffer_size(buffer_bytes);
    let _ = sock.set_send_buffer_size(buffer_bytes);
}

/// Convenience: nodelay on + default buffer size.
pub fn tune_tcp_default(stream: &tokio::net::TcpStream) {
    tune_tcp(stream, true, DEFAULT_TCP_BUFFER_BYTES);
}
