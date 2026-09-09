use crate::hub::HubState;
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::parse_data_handshake;
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
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_data(state, stream).await {
                warn!(%peer, error = %e, "hub data session error");
            }
        });
    }
}

async fn handle_data(state: HubState, mut stream: TcpStream) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 512];
    let n = tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buf))
        .await
        .map_err(|_| anyhow::anyhow!("data handshake timeout"))??;
    if n == 0 {
        anyhow::bail!("empty data handshake");
    }
    let line = std::str::from_utf8(&buf[..n]).map_err(|e| anyhow::anyhow!("handshake utf8: {e}"))?;
    let (cid, token) = parse_data_handshake(line)?;

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
