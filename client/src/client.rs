use crate::config::EdgeConfig;
use tracing::{error, info, warn};
use tokio::time::{sleep, Duration};

/// Edge runtime: management prefers internal mesh; Hub remains for visitor tunnels.
pub async fn run_edge(cfg: EdgeConfig) -> anyhow::Result<()> {
    if cfg
        .hub_host
        .as_ref()
        .map(|h| h.trim().is_empty())
        .unwrap_or(true)
    {
        anyhow::bail!("hub_host is required: visitor tunnels still use Admin Hub data plane");
    }

    info!(
        node_id = %cfg.node_id,
        hub = ?cfg.hub_host,
        services = cfg.services.len(),
        "starting edge"
    );

    // Join internal management mesh; publish local services there.
    if cfg.overlay.enabled {
        let mut o = cfg.overlay.clone();
        o.role = p2p_overlay::OverlayRole::Node;
        if o.node_id.trim().is_empty() {
            o.node_id = cfg.node_id.clone();
        }
        if o.bootstrap.is_empty() {
            if let Some(host) = cfg.hub_host.as_ref() {
                o.bootstrap.push(format!("{host}:{}", o.port));
            }
        }
        match p2p_overlay::start_overlay(o).await {
            Ok(mesh) => {
                info!(vip = %mesh.vip(), "management mesh ready (edge)");
                for svc in &cfg.services {
                    match mesh
                        .publish_register_service(
                            &svc.service_id,
                            &svc.name,
                            &svc.protocol,
                            &svc.local_addr,
                        )
                        .await
                    {
                        Ok(()) => info!(
                            service = %svc.service_id,
                            "registered service on management mesh"
                        ),
                        Err(e) => warn!(
                            error = %e,
                            service = %svc.service_id,
                            "mesh service register failed (Hub register still runs)"
                        ),
                    }
                }
                // Keep mesh alive alongside Hub client.
                tokio::spawn(async move {
                    let _mesh = mesh;
                    std::future::pending::<()>().await;
                });
            }
            Err(e) => {
                warn!(error = %e, "management mesh failed; Hub-only management fallback");
            }
        }
    }

    // Hub: visitor tunnels + transitional control (register still sent as backup).
    loop {
        crate::hub_client::run_hub_client(cfg.clone()).await;
        warn!("hub client exited unexpectedly, restarting...");
        sleep(Duration::from_secs(3)).await;
    }
}

#[allow(dead_code)]
fn _unused_error_log(e: anyhow::Error) {
    error!(error = %e, "edge error");
}
