//! Outbound connection from server node to Admin Hub (control + tunnel relay).
//! Server does NOT listen on a control plane; Hub is the only management channel.

use crate::data_plane::{self, register_pending};
use crate::state::AppState;
use base64::Engine;
use p2p_common::{PeerPathPurpose, PeerRole};
use p2p_control::{ControlSession, HeartbeatConfig};
use p2p_dataplane::copy_bidirectional;
use p2p_protocol::{encode_data_handshake, ControlMessage};
use p2p_transport::tune_tcp;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{info, warn};

/// How long the server waits for an edge direct dial before falling back to Hub.
const DIRECT_WAIT: Duration = Duration::from_secs(4);

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
    let hub_data_port = cfg.hub_data_port;
    let server_id = cfg
        .hub_server_id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(hostname);
    let token = cfg.hub_token.clone().unwrap_or_default();
    if token.is_empty() {
        anyhow::bail!("hub_token empty");
    }
    let data_endpoint = cfg.advertised_data_endpoint();

    info!(%control_addr, %server_id, ?data_endpoint, "connecting to admin hub");
    let stream = TcpStream::connect(&control_addr).await?;
    tune_tcp(&stream, cfg.tcp_nodelay, cfg.tcp_buffer_bytes);
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
            data_endpoint: data_endpoint.clone(),
        })
        .await?;

    let api_port = cfg.api_port;
    let hub_host = hub_host.to_string();
    let (tx, mut rx, handle) = session.into_channels(256);
    *state.hub_tx.write() = Some(tx.clone());

    if let Err(e) = crate::ports::report_mappings(state).await {
        warn!(error = %e, "report mappings failed");
    }

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
            ControlMessage::ApplyPorts { bindings } => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::ports::apply_port_plan(&state, bindings).await {
                        warn!(error = %e, "apply port plan failed");
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
                data_endpoint,
                ..
            } => {
                info!(
                    %request_id,
                    %connection_id,
                    %peer_node_id,
                    ?path,
                    ?purpose,
                    ?local_addr,
                    ?data_endpoint,
                    "hub peer path offer"
                );
                let hub_host = hub_host.clone();
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = accept_peer_path(
                        state,
                        hub_host,
                        hub_data_port,
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
    hub_data_port: u16,
    request_id: String,
    connection_id: String,
    data_token: String,
    purpose: PeerPathPurpose,
) -> anyhow::Result<()> {
    match purpose {
        PeerPathPurpose::Data => {
            // Move visitor stream into pending so edge can dial data_plane directly.
            let Some((_, tunnel)) = state.hub_tunnels.remove(&request_id) else {
                anyhow::bail!("no pending tunnel for {request_id}");
            };
            let public = tunnel
                .public
                .lock()
                .await
                .take()
                .ok_or_else(|| anyhow::anyhow!("public stream already taken"))?;

            register_pending(
                &state,
                connection_id.clone(),
                data_token.clone(),
                tunnel.node_id.clone(),
                tunnel.local_addr.clone(),
                tunnel.protocol.clone(),
                public,
            );

            // Wait for direct dial; if still pending after timeout → Hub relay.
            tokio::time::sleep(DIRECT_WAIT).await;
            if state.pending.contains_key(&connection_id) {
                info!(%connection_id, "direct dial timeout; falling back to hub-relay");
                let mut data = TcpStream::connect(format!("{hub_host}:{hub_data_port}")).await?;
                let cfg = state.config.read().clone();
                tune_tcp(&data, cfg.tcp_nodelay, cfg.tcp_buffer_bytes);
                let hs = encode_data_handshake(&connection_id, &data_token);
                data.write_all(&hs).await?;
                data_plane::bridge_hub_relay(
                    &state,
                    &connection_id,
                    &request_id,
                    data,
                    &tunnel.protocol,
                    &tunnel.node_id,
                    &tunnel.local_addr,
                )
                .await?;
            } else {
                info!(%connection_id, "direct path consumed by edge (or cleaned up)");
            }
        }
        PeerPathPurpose::Mgmt => {
            let mut data = TcpStream::connect(format!("{hub_host}:{hub_data_port}")).await?;
            let cfg = state.config.read().clone();
            tune_tcp(&data, cfg.tcp_nodelay, cfg.tcp_buffer_bytes);
            let hs = encode_data_handshake(&connection_id, &data_token);
            data.write_all(&hs).await?;
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
