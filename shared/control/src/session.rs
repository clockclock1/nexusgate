use crate::codec::ControlCodec;
use crate::heartbeat::HeartbeatConfig;
use futures::{SinkExt, StreamExt};
use p2p_common::{Error, Result};
use p2p_protocol::ControlMessage;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio::time::{interval, timeout};
use tokio_util::codec::Framed;
use tracing::{debug, warn};

/// Bidirectional control session over a framed stream.
pub struct ControlSession<S> {
    framed: Framed<S, ControlCodec>,
    last_rx: Instant,
    heartbeat: HeartbeatConfig,
    seq: u64,
}

impl<S> ControlSession<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: S, heartbeat: HeartbeatConfig) -> Self {
        Self {
            framed: Framed::new(stream, ControlCodec),
            last_rx: Instant::now(),
            heartbeat,
            seq: 0,
        }
    }

    pub fn last_rx(&self) -> Instant {
        self.last_rx
    }

    pub fn is_expired(&self) -> bool {
        self.heartbeat.is_expired(self.last_rx.elapsed())
    }

    pub async fn send(&mut self, msg: ControlMessage) -> Result<()> {
        self.framed
            .send(msg)
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Option<ControlMessage>> {
        match self.framed.next().await {
            Some(Ok(msg)) => {
                self.last_rx = Instant::now();
                Ok(Some(msg))
            }
            Some(Err(e)) => Err(Error::Io(e)),
            None => Ok(None),
        }
    }

    pub async fn recv_timeout(&mut self, dur: Duration) -> Result<Option<ControlMessage>> {
        match timeout(dur, self.recv()).await {
            Ok(r) => r,
            Err(_) => Err(Error::Timeout("control recv".into())),
        }
    }

    pub async fn send_heartbeat(&mut self) -> Result<()> {
        self.seq = self.seq.wrapping_add(1);
        self.send(ControlMessage::heartbeat(self.seq)).await
    }

    /// Split into send/recv channels driven by a background loop.
    /// Returns (tx_to_peer, rx_from_peer).
    pub fn into_channels(
        mut self,
        buffer: usize,
    ) -> (
        mpsc::Sender<ControlMessage>,
        mpsc::Receiver<ControlMessage>,
        tokio::task::JoinHandle<()>,
    )
    where
        S: Send + 'static,
    {
        let (out_tx, mut out_rx) = mpsc::channel::<ControlMessage>(buffer);
        let (in_tx, in_rx) = mpsc::channel::<ControlMessage>(buffer);
        let hb_interval = self.heartbeat.interval;

        let handle = tokio::spawn(async move {
            let mut tick = interval(hb_interval);
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        if self.is_expired() {
                            warn!("control session heartbeat timeout");
                            break;
                        }
                        if let Err(e) = self.send_heartbeat().await {
                            debug!("heartbeat send failed: {e}");
                            break;
                        }
                    }
                    msg = out_rx.recv() => {
                        match msg {
                            Some(m) => {
                                if let Err(e) = self.send(m).await {
                                    debug!("control send failed: {e}");
                                    break;
                                }
                            }
                            None => break,
                        }
                    }
                    msg = self.recv() => {
                        match msg {
                            Ok(Some(m)) => {
                                // Ignore peer heartbeats at session layer; still mark alive via recv.
                                if matches!(m, ControlMessage::Heartbeat { .. }) {
                                    continue;
                                }
                                if in_tx.send(m).await.is_err() {
                                    break;
                                }
                            }
                            Ok(None) => break,
                            Err(e) => {
                                debug!("control recv failed: {e}");
                                break;
                            }
                        }
                    }
                }
            }
        });

        (out_tx, in_rx, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    #[tokio::test]
    async fn session_send_recv() {
        let (a, b) = duplex(4096);
        let mut sa = ControlSession::new(a, HeartbeatConfig::default());
        let mut sb = ControlSession::new(b, HeartbeatConfig::default());

        sa.send(ControlMessage::Hello {
            version: "1.0".into(),
            features: vec![],
        })
        .await
        .unwrap();

        let msg = sb.recv().await.unwrap().unwrap();
        assert_eq!(msg.message_type(), "HELLO");
    }
}
