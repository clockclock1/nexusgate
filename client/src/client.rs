use crate::config::EdgeConfig;
use p2p_common::ProtocolKind;
use p2p_control::{ControlSession, HeartbeatConfig};
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::{encode_data_handshake, ControlMessage};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

pub async fn run_edge(cfg: EdgeConfig) -> anyhow::Result<()> {
    if cfg.hub_host.as_ref().is_some_and(|h| !h.trim().is_empty()) {
        let hub_cfg = cfg.clone();
        tokio::spawn(async move {
            crate::hub_client::run_hub_client(hub_cfg).await;
        });
    }

    loop {
        match run_once(&cfg).await {
            Ok(()) => warn!("control session closed, reconnecting..."),
            Err(e) => error!(error = %e, "edge session error, reconnecting..."),
        }
        sleep(Duration::from_secs(3)).await;
    }
}

async fn run_once(cfg: &EdgeConfig) -> anyhow::Result<()> {
    let control_addr = format!("{}:{}", cfg.server, cfg.control_port);
    info!(%control_addr, node_id = %cfg.node_id, "connecting control");
    let stream = TcpStream::connect(&control_addr).await?;
    let mut session = ControlSession::new(stream, HeartbeatConfig::default());

    // Wait HELLO
    let hello = session
        .recv_timeout(Duration::from_secs(10))
        .await?
        .ok_or_else(|| anyhow::anyhow!("server closed"))?;
    if !matches!(hello, ControlMessage::Hello { .. }) {
        anyhow::bail!("expected HELLO");
    }

    session
        .send(ControlMessage::Auth {
            node_id: cfg.node_id.clone(),
            token: cfg.token.clone(),
            role: None,
        })
        .await?;

    let auth_ok = session
        .recv_timeout(Duration::from_secs(10))
        .await?
        .ok_or_else(|| anyhow::anyhow!("no AUTH_OK"))?;
    match auth_ok {
        ControlMessage::AuthOk { .. } => info!("authenticated"),
        ControlMessage::Error { code, message, .. } => {
            anyhow::bail!("auth failed: {code} {message}");
        }
        other => anyhow::bail!("unexpected {}", other.message_type()),
    }

    session
        .send(ControlMessage::Register {
            node_id: cfg.node_id.clone(),
            hostname: cfg.name.clone().or_else(|| Some(hostname())),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            labels: vec![],
        })
        .await?;

    for svc in &cfg.services {
        let protocol = ProtocolKind::parse(&svc.protocol).unwrap_or(ProtocolKind::Tcp);
        session
            .send(ControlMessage::RegisterService {
                service_id: svc.service_id.clone(),
                name: svc.name.clone(),
                protocol,
                local_addr: svc.local_addr.clone(),
                public_port: None,
                domain: None,
            })
            .await?;
    }

    let data_host = cfg.server.clone();
    let data_port = cfg.data_port;
    let (tx, mut rx, handle) = session.into_channels(256);
    let _tx = tx;

    while let Some(msg) = rx.recv().await {
        match msg {
            ControlMessage::Connect {
                connection_id,
                data_token,
                local_addr,
                ..
            } => {
                let data_host = data_host.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        handle_connect(data_host, data_port, connection_id, data_token, local_addr)
                            .await
                    {
                        warn!(error = %e, "CONNECT handling failed");
                    }
                });
            }
            ControlMessage::ConfigPush { revision, .. } => {
                info!(revision, "received config push");
            }
            ControlMessage::Error { code, message, .. } => {
                warn!(%code, %message, "server error");
            }
            _ => {}
        }
    }
    handle.abort();
    Ok(())
}

async fn handle_connect(
    data_host: String,
    data_port: u16,
    connection_id: String,
    data_token: String,
    local_addr: String,
) -> anyhow::Result<()> {
    info!(%connection_id, %local_addr, "CONNECT received");
    let local = TcpStream::connect(&local_addr).await?;
    let mut data = TcpStream::connect(format!("{data_host}:{data_port}")).await?;
    let hs = encode_data_handshake(&connection_id, &data_token);
    data.write_all(&hs).await?;
    let (rx, tx) = copy_bidirectional(local, data).await?;
    info!(%connection_id, rx, tx, "data path finished");
    Ok(())
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "edge".into())
}
