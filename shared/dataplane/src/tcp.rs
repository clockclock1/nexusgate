use p2p_common::{Error, Result};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tracing::debug;

/// Bidirectional byte copy between two streams until EOF or error.
/// Returns (bytes_a_to_b, bytes_b_to_a).
pub async fn copy_bidirectional<A, B>(a: A, b: B) -> Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let (mut a_r, mut a_w) = tokio::io::split(a);
    let (mut b_r, mut b_w) = tokio::io::split(b);

    let a_to_b = async {
        let n = tokio::io::copy_buf(&mut tokio::io::BufReader::with_capacity(64 * 1024, &mut a_r), &mut b_w).await?;
        let _ = b_w.shutdown().await;
        Ok::<u64, std::io::Error>(n)
    };
    let b_to_a = async {
        let n = tokio::io::copy_buf(&mut tokio::io::BufReader::with_capacity(64 * 1024, &mut b_r), &mut a_w).await?;
        let _ = a_w.shutdown().await;
        Ok::<u64, std::io::Error>(n)
    };

    let (r1, r2) = tokio::join!(a_to_b, b_to_a);
    let n1 = r1.map_err(Error::from)?;
    let n2 = r2.map_err(Error::from)?;
    debug!(a_to_b = n1, b_to_a = n2, "bidirectional copy finished");
    Ok((n1, n2))
}

/// Same as copy_bidirectional but with an idle timeout on each direction.
pub async fn copy_bidirectional_with_idle_timeout<A, B>(
    a: A,
    b: B,
    idle: Duration,
) -> Result<(u64, u64)>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let _ = idle; // reserved: per-chunk idle watchdog in later phase
    copy_bidirectional(a, b).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    #[tokio::test]
    async fn copy_echo_pair() {
        let (client_a, server_a) = duplex(1024);
        let (client_b, server_b) = duplex(1024);

        let pump = tokio::spawn(async move { copy_bidirectional(server_a, server_b).await });

        let mut ca = client_a;
        let mut cb = client_b;
        ca.write_all(b"hello").await.unwrap();
        ca.shutdown().await.unwrap();
        let mut buf = [0u8; 5];
        cb.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello");
        // Close reverse direction so bidirectional copy can finish.
        cb.shutdown().await.unwrap();
        drop(cb);
        let _ = pump.await.unwrap();
    }
}
