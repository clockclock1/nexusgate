use crate::config::ServerEntry;
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize)]
pub struct ServersResponse {
    pub servers: Vec<ServerEntry>,
    pub default_server: Option<String>,
}

pub async fn list_servers(State(state): State<AppState>) -> Json<ServersResponse> {
    Json(ServersResponse {
        servers: state.registry.list(),
        default_server: state.registry.default_server_id(),
    })
}

#[derive(Deserialize)]
pub struct UpsertServerReq {
    pub id: String,
    pub name: String,
    pub api_upstream: String,
    #[serde(default)]
    pub make_default: bool,
}

pub async fn upsert_server(
    State(state): State<AppState>,
    Json(req): Json<UpsertServerReq>,
) -> Result<Json<ServersResponse>, (StatusCode, Json<Value>)> {
    let entry = ServerEntry {
        id: req.id.trim().to_string(),
        name: if req.name.trim().is_empty() {
            req.id.trim().to_string()
        } else {
            req.name.trim().to_string()
        },
        api_upstream: req.api_upstream.trim().to_string(),
    };
    match state.registry.upsert(entry, req.make_default) {
        Ok(servers) => Ok(Json(ServersResponse {
            default_server: state.registry.default_server_id(),
            servers,
        })),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": e.to_string() })),
        )),
    }
}

pub async fn delete_server(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ServersResponse>, (StatusCode, Json<Value>)> {
    match state.registry.remove(&id) {
        Ok(servers) => Ok(Json(ServersResponse {
            default_server: state.registry.default_server_id(),
            servers,
        })),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": e.to_string() })),
        )),
    }
}

#[derive(Deserialize)]
pub struct DefaultReq {
    pub id: String,
}

pub async fn set_default_server(
    State(state): State<AppState>,
    Json(req): Json<DefaultReq>,
) -> Result<Json<ServersResponse>, (StatusCode, Json<Value>)> {
    match state.registry.set_default(&req.id) {
        Ok(servers) => Ok(Json(ServersResponse {
            default_server: state.registry.default_server_id(),
            servers,
        })),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": e.to_string() })),
        )),
    }
}

/// Probe one registered server via its `/api/health`.
pub async fn probe_server(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let (http_base, _) = match state.registry.resolve(Some(&id)) {
        Ok(v) => v,
        Err(e) => {
            return Json(json!({ "id": id, "ok": false, "error": e.to_string() }));
        }
    };
    let url = format!("{http_base}/api/health");
    match state.http.get(&url).send().await {
        Ok(resp) => Json(json!({
            "id": id,
            "ok": resp.status().is_success(),
            "status": resp.status().as_u16(),
        })),
        Err(e) => Json(json!({
            "id": id,
            "ok": false,
            "error": e.to_string(),
        })),
    }
}

pub async fn hub_status(State(state): State<AppState>) -> Json<Value> {
    let Some(hub) = &state.hub else {
        return Json(json!({ "enabled": false }));
    };
    let snap = state.registry.snapshot();
    let peers = hub.roster();
    let servers = peers.iter().filter(|p| matches!(p.role, p2p_common::PeerRole::Server)).count();
    let edges = peers.iter().filter(|p| matches!(p.role, p2p_common::PeerRole::Edge)).count();
    Json(json!({
        "enabled": true,
        "control_port": snap.hub_control_port,
        "data_port": snap.hub_data_port,
        "listen": snap.hub_listen,
        "peers_total": peers.len(),
        "servers_online": servers,
        "edges_online": edges,
    }))
}

pub async fn hub_peers(State(state): State<AppState>) -> Json<Value> {
    let Some(hub) = &state.hub else {
        return Json(json!({ "enabled": false, "peers": [] }));
    };
    Json(json!({
        "enabled": true,
        "peers": hub.roster(),
    }))
}
