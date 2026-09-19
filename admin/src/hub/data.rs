use crate::hub::HubState;
use bytes::BytesMut;
use p2p_dataplane::{copy_bidirectional, PrefixedStream};
use p2p_protocol::{extract_line, parse_data_handshake};
use p2p_transport::tune_tcp;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, warn};

pub async fn run_hub_data(state: HubState, addr: SocketAddr) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    info!(%addr, "hub data listening");
    let cleaner = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            cleaner.cleanup_stale_pending(Duration::from_secs(30));
        }
    });
    loop {
        let (stream, peer) = listener.accept().await?;
        tune_tcp(&stream, true, p2p_transport::DEFAULT_TCP_BUFFER_BYTES);
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_data(state, stream).await {
                warn!(%peer, error = %e, "hub data session error");
            }
        });
    }
}

async fn handle_data(state: HubState, mut stream: TcpStream) -> anyhow::Result<()> {
    // Line-framed handshake. A single read(N) can swallow visitor bytes that
    // were coalesced behind `DATA ...\n` on the same TCP segment (server may
    // write handshake then immediately forward the public request).
    let (cid, token, leftover) = read_handshake_line(&mut stream).await?;
    let stream = PrefixedStream::new(leftover, stream);

    let slot = {
        let pending = state
            .pending_paths
            .get(&cid)
            .ok_or_else(|| anyhow::anyhow!("unknown connection_id {cid}"))?;
        if pending.data_token != token {
            anyhow::bail!("bad data token for {cid}");
        }
        pending.stream_slot.clone()
    };

    let maybe_pair = {
        let mut guard = slot.lock().await;
        if guard.is_none() {
            *guard = Some(stream);
            None
        } else {
            let peer = guard.take().expect("checked");
            Some((peer, stream))
        }
    };

    if let Some((peer, stream)) = maybe_pair {
        state.pending_paths.remove(&cid);
        info!(%cid, "hub relay bridging peers");
        let _ = copy_bidirectional(peer, stream).await;
    }
    Ok(())
}

async fn read_handshake_line(
    stream: &mut TcpStream,
) -> anyhow::Result<(String, String, BytesMut)> {
    let mut buf = BytesMut::with_capacity(256);
    let deadline = tokio::time::sleep(Duration::from_secs(10));
    tokio::pin!(deadline);
    let line = loop {
        tokio::select! {
            _ = &mut deadline => anyhow::bail!("data handshake timeout"),
            n = stream.read_buf(&mut buf) => {
                let n = n?;
                if n == 0 {
                    anyhow::bail!("empty data handshake");
                }
                if let Some(line) = extract_line(&mut buf) {
                    break line;
                }
                if buf.len() > 512 {
                    anyhow::bail!("data handshake too large");
                }
            }
        }
    };
    let (cid, token) = parse_data_handshake(&line)?;
    Ok((cid, token, buf))
}
