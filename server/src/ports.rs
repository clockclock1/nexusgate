//! Bind visitor + data ports only after Admin Hub pushes `ApplyPorts`.

use crate::data_plane;
use crate::gateway;
use crate::state::AppState;
use p2p_protocol::{ControlMessage, PortBinding};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use tokio::task::JoinHandle;

/// Tell the Hub which mappings exist. Hub assigns missing `data_port` and pushes `ApplyPorts`.
pub async fn report_mappings(state: &AppState) -> anyhow::Result<()> {
    let rows = sqlx::query("SELECT * FROM routes")
        .fetch_all(&state.db)
        .await?;
    let mut mappings = Vec::with_capacity(rows.len());
    for r in rows {
        let service_id: String = r.get("service_id");
        let local_addr: String =
            sqlx::query_scalar("SELECT local_addr FROM services WHERE service_id = ?")
                .bind(&service_id)
                .fetch_optional(&state.db)
                .await?
                .unwrap_or_default();
        mappings.push(PortBinding {
            route_id: r.get("route_id"),
            node_id: r.get("node_id"),
            visitor_port: r.get::<i64, _>("public_port") as u16,
            data_port: r
                .try_get::<Option<i64>, _>("data_port")
                .ok()
                .flatten()
                .map(|p| p as u16),
            protocol: r.get("protocol"),
            local_addr,
            enabled: r.get::<i64, _>("enabled") == 1,
        });
    }
    state.hub_send(ControlMessage::ReportMappings { mappings })?;
    Ok(())
}

/// Persist issued data ports and open/close listeners to match the plan.
pub async fn apply_port_plan(state: &AppState, bindings: Vec<PortBinding>) -> anyhow::Result<()> {
    for b in &bindings {
        if let Some(port) = b.data_port {
            sqlx::query("UPDATE routes SET data_port = ? WHERE route_id = ?")
                .bind(port as i64)
                .bind(&b.route_id)
                .execute(&state.db)
                .await?;
        }
    }
    crate::db::load_routes_into(&state.db, &state.routes).await?;

    if !state
        .cleanup_started
        .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        let s1 = state.clone();
        tokio::spawn(async move {
            gateway::cleanup_hub_tunnels(s1).await;
        });
        let s2 = state.clone();
        tokio::spawn(async move {
            data_plane::cleanup_pending(s2).await;
        });
    }

    let listen = state.config.read().listen.clone();
    let mut desired: HashSet<String> = HashSet::new();
    for b in bindings.iter().filter(|b| b.enabled) {
        desired.insert(format!("v:{}", b.visitor_port));
        if let Some(p) = b.data_port {
            desired.insert(format!("d:{p}"));
        }
    }

    let mut guard = state.listeners.lock().await;
    let stale: Vec<String> = guard
        .keys()
        .filter(|k| !desired.contains(*k))
        .cloned()
        .collect();
    for key in stale {
        if let Some(handle) = guard.remove(&key) {
            handle.abort();
            tracing::info!(%key, "stopped listener");
        }
    }

    let mut seen_v = HashSet::new();
    let mut seen_d = HashSet::new();
    for b in bindings.into_iter().filter(|b| b.enabled) {
        if seen_v.insert(b.visitor_port) {
            let key = format!("v:{}", b.visitor_port);
            if !guard.contains_key(&key) {
                let addr: SocketAddr = format!("{listen}:{}", b.visitor_port).parse()?;
                let s = state.clone();
                let handle = tokio::spawn(async move {
                    if let Err(e) = gateway::run_tcp_gateway(s, addr).await {
                        tracing::error!(%addr, error = %e, "visitor listener exited");
                    }
                });
                tracing::info!(%addr, route = %b.route_id, "visitor port bound");
                guard.insert(key, handle);
            }
        }
        if let Some(port) = b.data_port {
            if seen_d.insert(port) {
                let key = format!("d:{port}");
                if !guard.contains_key(&key) {
                    let addr: SocketAddr = format!("{listen}:{port}").parse()?;
                    let s = state.clone();
                    let handle = tokio::spawn(async move {
                        if let Err(e) = data_plane::run_data_plane(s, addr).await {
                            tracing::error!(%addr, error = %e, "data listener exited");
                        }
                    });
                    tracing::info!(%addr, route = %b.route_id, "data port bound");
                    guard.insert(key, handle);
                }
            }
        }
    }
    tracing::info!(listeners = guard.len(), "port plan applied");
    Ok(())
}

pub type ListenerMap = HashMap<String, JoinHandle<()>>;
