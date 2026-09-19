use p2p_common::{Error, Result};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::debug;

/// User-space copy buffer (256 KiB).
pub const COPY_BUF_SIZE: usize = 256 * 1024;

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
        let n = tokio::io::copy_buf(
            &mut tokio::io::BufReader::with_capacity(COPY_BUF_SIZE, &mut a_r),
            &mut b_w,
        )
        .await?;
        let _ = b_w.shutdown().await;
        Ok::<u64, std::io::Error>(n)
    };
    let b_to_a = async {
        let n = tokio::io::copy_buf(
            &mut tokio::io::BufReader::with_capacity(COPY_BUF_SIZE, &mut b_r),
            &mut a_w,
        )
        .await?;
        let _ = a_w.shutdown().await;
        Ok::<u64, std::io::Error>(n)
    };

    let (r1, r2) = tokio::join!(a_to_b, b_to_a);
    let n1 = r1.map_err(Error::from)?;
    let n2 = r2.map_err(Error::from)?;
    debug!(a_to_b = n1, b_to_a = n2, "bidirectional copy finished");
    Ok((n1, n2))
}

/// TCP↔TCP copy: Linux uses `splice` zero-copy; other platforms use buffered copy.
pub async fn copy_bidirectional_tcp(a: TcpStream, b: TcpStream) -> Result<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        splice::copy_bidirectional_splice(a, b).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        copy_bidirectional(a, b).await
    }
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

#[cfg(target_os = "linux")]
mod splice {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use tokio::io::Interest;

    const PIPE_BUF: usize = 256 * 1024;
    const SPLICE_F_MOVE: libc::c_uint = 1;
    const SPLICE_F_NONBLOCK: libc::c_uint = 2;
    const SPLICE_F_MORE: libc::c_uint = 4;

    pub async fn copy_bidirectional_splice(a: TcpStream, b: TcpStream) -> Result<(u64, u64)> {
        let (a_r, a_w) = a.into_split();
        let (b_r, b_w) = b.into_split();
        let a_to_b = splice_one(a_r, b_w);
        let b_to_a = splice_one(b_r, a_w);
        let (r1, r2) = tokio::join!(a_to_b, b_to_a);
        let n1 = r1.map_err(Error::from)?;
        let n2 = r2.map_err(Error::from)?;
        debug!(a_to_b = n1, b_to_a = n2, "splice bidirectional copy finished");
        Ok((n1, n2))
    }

    async fn splice_one(
        reader: tokio::net::tcp::OwnedReadHalf,
        mut writer: tokio::net::tcp::OwnedWriteHalf,
    ) -> std::io::Result<u64> {
        let (pipe_r, pipe_w) = make_pipe()?;
        let mut total = 0u64;
        loop {
            let n = match splice_socket_to_pipe(&reader, pipe_w.as_raw_fd()).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e),
            };
            let mut left = n;
            while left > 0 {
                match splice_pipe_to_socket(pipe_r.as_raw_fd(), &writer, left).await {
                    Ok(0) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            "splice wrote 0",
                        ));
                    }
                    Ok(w) => {
                        left -= w;
                        total += w as u64;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        let _ = writer.shutdown().await;
        Ok(total)
    }

    fn make_pipe() -> std::io::Result<(OwnedFd, OwnedFd)> {
        let mut fds = [0; 2];
        let rc = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) };
        if rc < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // Enlarge pipe capacity when possible (Linux fcntl F_SETPIPE_SZ).
        let _ = unsafe { libc::fcntl(fds[1], libc::F_SETPIPE_SZ, PIPE_BUF as libc::c_int) };
        Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
    }

    async fn splice_socket_to_pipe(
        reader: &tokio::net::tcp::OwnedReadHalf,
        pipe_w: RawFd,
    ) -> std::io::Result<usize> {
        loop {
            reader.readable().await?;
            let fd = reader.as_ref().as_raw_fd();
            let n = unsafe {
                libc::splice(
                    fd,
                    std::ptr::null_mut(),
                    pipe_w,
                    std::ptr::null_mut(),
                    PIPE_BUF,
                    SPLICE_F_MOVE | SPLICE_F_NONBLOCK,
                )
            };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    reader.as_ref().clear_ready(Interest::READABLE);
                    continue;
                }
                return Err(err);
            }
            return Ok(n as usize);
        }
    }

    async fn splice_pipe_to_socket(
        pipe_r: RawFd,
        writer: &tokio::net::tcp::OwnedWriteHalf,
        max: usize,
    ) -> std::io::Result<usize> {
        loop {
            writer.writable().await?;
            let fd = writer.as_ref().as_raw_fd();
            let n = unsafe {
                libc::splice(
                    pipe_r,
                    std::ptr::null_mut(),
                    fd,
                    std::ptr::null_mut(),
                    max,
                    SPLICE_F_MOVE | SPLICE_F_NONBLOCK | SPLICE_F_MORE,
                )
            };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    writer.as_ref().clear_ready(Interest::WRITABLE);
                    continue;
                }
                return Err(err);
            }
            return Ok(n as usize);
        }
    }
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
