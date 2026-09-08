use crate::traits::{map_io, TransportConnector, TransportListener};
use async_trait::async_trait;
use p2p_common::Result;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tracing::debug;

/// Tokio TCP listener implementing TransportListener.
pub struct TcpTransportListener {
    inner: TcpListener,
}

impl TcpTransportListener {
    pub async fn bind(addr: &str) -> Result<Self> {
        let listener = TcpListener::bind(addr).await.map_err(map_io)?;
        debug!(%addr, local = ?listener.local_addr().ok(), "tcp listener bound");
        Ok(Self { inner: listener })
    }

    pub async fn bind_reuse(addr: &str) -> Result<Self> {
        let sock_addr: SocketAddr = addr
            .parse()
            .map_err(|e| p2p_common::Error::config(format!("invalid addr {addr}: {e}")))?;
        let domain = if sock_addr.is_ipv4() {
            Domain::IPV4
        } else {
            Domain::IPV6
        };
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP)).map_err(map_io)?;
        socket.set_reuse_address(true).map_err(map_io)?;
        socket.set_nonblocking(true).map_err(map_io)?;
        socket.bind(&sock_addr.into()).map_err(map_io)?;
        socket.listen(1024).map_err(map_io)?;
        let std_listener: std::net::TcpListener = socket.into();
        let listener = TcpListener::from_std(std_listener).map_err(map_io)?;
        Ok(Self { inner: listener })
    }

    pub fn into_inner(self) -> TcpListener {
        self.inner
    }
}

#[async_trait]
impl TransportListener for TcpTransportListener {
    type Stream = TcpStream;

    async fn accept(&self) -> Result<(Self::Stream, SocketAddr)> {
        self.inner.accept().await.map_err(map_io)
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        self.inner.local_addr().map_err(map_io)
    }
}

/// Default TCP connector.
#[derive(Debug, Default, Clone)]
pub struct TcpTransportConnector {
    pub connect_timeout: Option<std::time::Duration>,
}

impl TcpTransportConnector {
    pub fn new() -> Self {
        Self {
            connect_timeout: Some(std::time::Duration::from_secs(10)),
        }
    }
}

#[async_trait]
impl TransportConnector for TcpTransportConnector {
    type Stream = TcpStream;

    async fn connect(&self, addr: &str) -> Result<Self::Stream> {
        let fut = TcpStream::connect(addr);
        match self.connect_timeout {
            Some(t) => tokio::time::timeout(t, fut)
                .await
                .map_err(|_| p2p_common::Error::Timeout(format!("connect {addr}")))?
                .map_err(map_io),
            None => fut.await.map_err(map_io),
        }
    }
}

/// Convenience connect helper.
pub async fn tcp_connect(addr: &str) -> Result<TcpStream> {
    TcpTransportConnector::new().connect(addr).await
}

/// Convenience bind helper.
pub async fn tcp_bind(addr: &str) -> Result<TcpListener> {
    Ok(TcpTransportListener::bind(addr).await?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{TransportConnector, TransportListener};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn tcp_echo() {
        let listener = TcpTransportListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let connector = TcpTransportConnector::new();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).await.unwrap();
            stream.write_all(&buf).await.unwrap();
        });

        let mut client = connector.connect(&addr.to_string()).await.unwrap();
        client.write_all(b"ping").await.unwrap();
        let mut buf = [0u8; 4];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"ping");
        server.await.unwrap();
    }
}
