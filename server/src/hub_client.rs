//! Outbound connection from server node to Admin Hub (control + tunnel relay).
//! Server does NOT listen on a control plane; Hub is the only management channel.

use crate::gateway::complete_hub_tunnel;
use crate::state::AppState;
use base64::Engine;
use p2p_common::{PeerPathPurpose, PeerRole};
use p2p_control::{ControlSession, HeartbeatConfig};
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::{encode_data_handshake, ControlMessage};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{info, warn};

pub async fn run_hub_client(state: AppState) {
    loop {
        let cfg = state.config.read().clone();
        let Some(hub_host) = cfg.hub_host.filter(|h| !h.trim().is_empty()) else {
            tracing::error!("hub_host is required (server has no local control plane)");
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };
        match run_once(&state, &hub_host).await {
            Ok(()) => warn!("hub session closed, reconnecting..."),
            Err(e) => warn!(error = %e, "hub session error, reconnecting..."),
        }
        *state.hub_tx.write() = None;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn run_once(state: &AppState, hub_host: &str) -> anyhow::Result<()> {
    let cfg = state.config.read().clone();
    let control_addr = format!("{}:{}", hub_host.trim(), cfg.hub_control_port);
    let data_port = cfg.hub_data_port;
    let server_id = cfg
        .hub_server_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(hostname);
    let token = cfg.hub_token.clone().unwrap_or_default();
    if token.is_empty() {
        anyhow::bail!("hub_token empty");
    }

    info!(%control_addr, %server_id, "connecting to admin hub");
    let stream = TcpStream::connect(&control_addr).await?;
    let mut session = ControlSession::new(stream, HeartbeatConfig::default());

    let hello = session
        .recv_timeout(Duration::from_secs(10))
        .await?
        .ok_or_else(|| anyhow::anyhow!("hub closed"))?;
    if !matches!(hello, ControlMessage::Hello { .. }) {
        anyhow::bail!("expected HELLO from hub");
    }

    session
        .send(ControlMessage::auth_with_role(
            &server_id,
            &token,
            PeerRole::Server,
        ))
        .await?;

    let auth_ok = session
        .recv_timeout(Duration::from_secs(10))
        .await?
        .ok_or_else(|| anyhow::anyhow!("no AUTH_OK"))?;
    match auth_ok {
        ControlMessage::AuthOk { .. } => info!(%server_id, "authenticated to hub"),
        ControlMessage::Error { code, message, .. } => {
            anyhow::bail!("hub auth failed: {code} {message}");
        }
        other => anyhow::bail!("unexpected {}", other.message_type()),
    }

    session
        .send(ControlMessage::Register {
            node_id: server_id.clone(),
            hostname: Some(hostname()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            labels: vec!["role:server".into()],
            transports: vec![],
        })
        .await?;

    let api_port = cfg.api_port;
    let hub_host = hub_host.to_string();
    let (tx, mut rx, handle) = session.into_channels(256);
    *state.hub_tx.write() = Some(tx.clone());

    while let Some(msg) = rx.recv().await {
        match msg {
            ControlMessage::HubPeers { peers } => {
                info!(count = peers.len(), "hub roster updated");
                *state.hub_peers.write() = peers;
                state.refresh_online_metric();
            }
            ControlMessage::RegisterService {
                service_id,
                name,
                protocol,
                local_addr,
                node_id,
                ..
            } => {
                let proto = protocol.as_str().to_string();
                let edge_id = node_id.unwrap_or_default();
                if let Err(e) = upsert_registered_service(
                    &state.db,
                    &edge_id,
                    &service_id,
                    &name,
                    &proto,
                    &local_addr,
                )
                .await
                {
                    warn!(error = %e, %service_id, "hub service register failed");
                } else {
                    info!(%service_id, %edge_id, %local_addr, "hub-forwarded service registered");
                }
            }
            ControlMessage::MgmtForward {
                request_id,
                method,
                path,
                headers,
                body_b64,
            } => {
                let tx = tx.clone();
                let api_port = api_port;
                tokio::spawn(async move {
                    let reply = match execute_local_mgmt(api_port, method, path, headers, body_b64)
                        .await
                    {
                        Ok((status, resp_headers, body)) => ControlMessage::MgmtForwardResult {
                            request_id,
                            status,
                            headers: resp_headers,
                            body_b64: Some(base64::engine::general_purpose::STANDARD.encode(body)),
                            error: None,
                        },
                        Err(e) => ControlMessage::MgmtForwardResult {
                            request_id,
                            status: 502,
                            headers: vec![],
                            body_b64: None,
                            error: Some(e.to_string()),
                        },
                    };
                    if tx.send(reply).await.is_err() {
                        warn!("failed to send mgmt result to hub");
                    }
                });
            }
            ControlMessage::PeerPathOffer {
                request_id,
                connection_id,
                data_token,
                path,
                purpose,
                peer_node_id,
                local_addr,
                ..
            } => {
                info!(
                    %request_id,
                    %connection_id,
                    %peer_node_id,
                    ?path,
                    ?purpose,
                    ?local_addr,
                    "hub peer path offer"
                );
                let hub_host = hub_host.clone();
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = accept_peer_path(
                        state,
                        hub_host,
                        data_port,
                        request_id,
                        connection_id,
                        data_token,
                        purpose,
                    )
                    .await
                    {
                        warn!(error = %e, "accept peer path failed");
                    }
                });
            }
            ControlMessage::Error { code, message, .. } => {
                warn!(%code, %message, "hub error");
            }
            _ => {}
        }
    }
    handle.abort();
    Ok(())
}

async fn accept_peer_path(
    state: AppState,
    hub_host: String,
    data_port: u16,
    request_id: String,
    connection_id: String,
    data_token: String,
    purpose: PeerPathPurpose,
) -> anyhow::Result<()> {
    let mut data = TcpStream::connect(format!("{hub_host}:{data_port}")).await?;
    let hs = encode_data_handshake(&connection_id, &data_token);
    data.write_all(&hs).await?;
    match purpose {
        PeerPathPurpose::Data => {
            complete_hub_tunnel(&state, &request_id, data).await?;
        }
        PeerPathPurpose::Mgmt => {
            // Keep relay leg alive for management mesh.
            let (a, b) = tokio::io::duplex(8);
            let _ = copy_bidirectional(data, a).await;
            drop(b);
        }
    }
    Ok(())
}

async fn execute_local_mgmt(
    api_port: u16,
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body_b64: Option<String>,
) -> anyhow::Result<(u16, Vec<(String, String)>, Vec<u8>)> {
    let body = match body_b64 {
        Some(b64) => Some(base64::engine::general_purpose::STANDARD.decode(b64)?),
        None => None,
    };
    execute_local_mgmt_bytes(api_port, method, path, headers, body).await
}

/// Localhost management API call (used by Hub MGMT_FORWARD and overlay mesh).
pub async fn execute_local_mgmt_bytes(
    api_port: u16,
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
) -> anyhow::Result<(u16, Vec<(String, String)>, Vec<u8>)> {
    // API is localhost-only (not a public management port).
    let url = format!("http://127.0.0.1:{api_port}{path}");
    let client = reqwest::Client::new();
    let mut builder = client.request(
        reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
        &url,
    );
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host") || k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        builder = builder.header(k, v);
    }
    if let Some(body) = body {
        builder = builder.body(body);
    }
    let resp = builder.send().await?;
    let status = resp.status().as_u16();
    let mut resp_headers = Vec::new();
    for (k, v) in resp.headers().iter() {
        if let Ok(val) = v.to_str() {
            resp_headers.push((k.as_str().to_string(), val.to_string()));
        }
    }
    let body = resp.bytes().await?.to_vec();
    Ok((status, resp_headers, body))
}

/// Upsert a service row from overlay / Hub registration.
pub async fn upsert_registered_service(
    db: &sqlx::SqlitePool,
    edge_id: &str,
    service_id: &str,
    name: &str,
    protocol: &str,
    local_addr: &str,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        r#"INSERT INTO services (service_id, name, node_id, protocol, local_addr, enabled, created_at, updated_at)
           VALUES (?, ?, ?, ?, ?, 1, ?, ?)
           ON CONFLICT(service_id) DO UPDATE SET
           name=excluded.name, node_id=excluded.node_id, protocol=excluded.protocol,
           local_addr=excluded.local_addr, updated_at=excluded.updated_at"#,
    )
    .bind(service_id)
    .bind(name)
    .bind(edge_id)
    .bind(protocol)
    .bind(local_addr)
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;
    Ok(())
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "p2p-server".into())
}
