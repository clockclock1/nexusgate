//! Re-inject bytes already read past a framing boundary (e.g. data handshake).

use bytes::BytesMut;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// `AsyncRead`/`AsyncWrite` wrapper that yields `prefix` before `inner`.
pub struct PrefixedStream<S> {
    prefix: BytesMut,
    inner: S,
}

impl<S> PrefixedStream<S> {
    pub fn new(prefix: BytesMut, inner: S) -> Self {
        Self { prefix, inner }
    }

    pub fn into_inner(self) -> (BytesMut, S) {
        (self.prefix, self.inner)
    }
}

impl<S: AsyncRead + Unpin> AsyncRead for PrefixedStream<S> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if !self.prefix.is_empty() {
            let n = std::cmp::min(buf.remaining(), self.prefix.len());
            buf.put_slice(&self.prefix[..n]);
            let _ = self.prefix.split_to(n);
            return Poll::Ready(Ok(()));
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for PrefixedStream<S> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    #[tokio::test]
    async fn prefix_then_inner() {
        let (mut client, server) = duplex(64);
        let mut prefixed = PrefixedStream::new(BytesMut::from(&b"AB"[..]), server);
        client.write_all(b"CD").await.unwrap();
        client.shutdown().await.unwrap();

        let mut out = vec![0u8; 4];
        prefixed.read_exact(&mut out).await.unwrap();
        assert_eq!(&out, b"ABCD");
    }
}
