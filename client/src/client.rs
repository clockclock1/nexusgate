use crate::config::EdgeConfig;
use p2p_common::{ProtocolKind, TransportKind};
use p2p_control::{ControlSession, HeartbeatConfig};
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::{encode_data_handshake, ControlMessage};
use p2p_transport::{self as transport, set_nodelay};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
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
    set_nodelay(&stream, true);
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
            transports: cfg.advertised_transports(),
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
    let default_ports = (
        cfg.data_port,
        cfg.data_quic_port,
        cfg.data_kcp_port,
    );
    let (tx, mut rx, handle) = session.into_channels(256);
    let _tx = tx;

    while let Some(msg) = rx.recv().await {
        match msg {
            ControlMessage::Connect {
                connection_id,
                data_token,
                local_addr,
                transport,
                data_port,
                ..
            } => {
                let data_host = data_host.clone();
                let port = data_port.unwrap_or_else(|| match transport {
                    TransportKind::Tcp => default_ports.0,
                    TransportKind::Quic => default_ports.1,
                    TransportKind::Kcp => default_ports.2,
                });
                tokio::spawn(async move {
                    if let Err(e) = handle_connect(
                        data_host,
                        port,
                        transport,
                        connection_id,
                        data_token,
                        local_addr,
                    )
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
    transport: TransportKind,
    connection_id: String,
    data_token: String,
    local_addr: String,
) -> anyhow::Result<()> {
    info!(%connection_id, %local_addr, %transport, data_port, "CONNECT received");
    let local = TcpStream::connect(&local_addr).await?;
    set_nodelay(&local, true);
    let hs = encode_data_handshake(&connection_id, &data_token);
    let addr = format!("{data_host}:{data_port}");

    match transport {
        TransportKind::Tcp => {
            let mut data = TcpStream::connect(&addr).await?;
            set_nodelay(&data, true);
            data.write_all(&hs).await?;
            finish_relay(&connection_id, local, data).await
        }
        TransportKind::Quic => {
            let sock: std::net::SocketAddr = addr
                .parse()
                .or_else(|_| resolve_host_port(&data_host, data_port))?;
            let mut data = transport::connect_quic_bidi(sock).await?;
            data.write_all(&hs).await?;
            finish_relay(&connection_id, local, data).await
        }
        TransportKind::Kcp => {
            let sock: std::net::SocketAddr = addr
                .parse()
                .or_else(|_| resolve_host_port(&data_host, data_port))?;
            let mut data = transport::connect_kcp(sock).await?;
            data.write_all(&hs).await?;
            finish_relay(&connection_id, local, data).await
        }
    }
}

async fn finish_relay<S>(
    connection_id: &str,
    local: TcpStream,
    data: S,
) -> anyhow::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (rx, tx) = copy_bidirectional(local, data).await?;
    info!(%connection_id, rx, tx, "data path finished");
    Ok(())
}

fn resolve_host_port(host: &str, port: u16) -> anyhow::Result<std::net::SocketAddr> {
    use std::net::ToSocketAddrs;
    let mut addrs = (host, port).to_socket_addrs()?;
    addrs
        .next()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve {host}:{port}"))
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "edge".into())
}
