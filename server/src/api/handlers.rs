use crate::api::auth::AppError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;

pub async fn dashboard(State(state): State<AppState>) -> Json<Value> {
    let snap = state.metrics.snapshot();
    let nodes_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM nodes")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let services_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM services")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    let routes_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM routes")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    Json(json!({
        "nodes_total": nodes_total,
        "nodes_online": snap.nodes_online,
        "connections_active": snap.connections_active,
        "traffic_rx_bytes": snap.rx_bytes,
        "traffic_tx_bytes": snap.tx_bytes,
        "p2p_rate": snap.p2p_rate,
        "relay_rate": snap.relay_rate,
        "services_total": services_total,
        "routes_total": routes_total,
    }))
}

pub async fn traffic(State(state): State<AppState>) -> Json<Value> {
    let points: Vec<Value> = state
        .metrics
        .history()
        .into_iter()
        .map(|s| {
            json!({
                "timestamp": chrono::DateTime::from_timestamp(s.ts, 0)
                    .map(|t| t.to_rfc3339())
                    .unwrap_or_default(),
                "rx_bytes": s.rx_bytes,
                "tx_bytes": s.tx_bytes,
            })
        })
        .collect();
    Json(json!({ "interval": "hourly", "points": points }))
}

pub async fn connections(State(state): State<AppState>) -> Json<Value> {
    let items: Vec<Value> = state
        .connections
        .iter()
        .map(|e| {
            let c = e.value();
            json!({
                "conn_id": c.conn_id,
                "node_id": c.node_id,
                "mode": c.mode,
                "status": c.status,
                "protocol": c.protocol,
                "local_addr": c.local_addr,
                "remote_addr": c.remote_addr,
                "rx_bytes": c.rx_bytes,
                "tx_bytes": c.tx_bytes,
                "started_at": c.started_at.to_rfc3339(),
            })
        })
        .collect();
    Json(json!(items))
}

pub async fn logs(State(state): State<AppState>) -> Json<Value> {
    let logs = state.logs.read().clone();
    Json(json!(logs.into_iter().rev().take(200).collect::<Vec<_>>()))
}

pub async fn topology(State(state): State<AppState>) -> Json<Value> {
    let mut nodes = vec![json!({
        "id": "server",
        "name": "Super Node",
        "status": "online",
    })];
    let mut edges = vec![];
    for n in state.online.iter() {
        nodes.push(json!({
            "id": n.node_id,
            "name": n.name,
            "status": "online",
            "nat_type": n.nat_type,
        }));
        edges.push(json!({
            "source": "server",
            "target": n.node_id,
            "mode": "relay",
        }));
    }
    Json(json!({ "nodes": nodes, "edges": edges }))
}

pub async fn p2p_stats(State(state): State<AppState>) -> Json<Value> {
    let snap = state.metrics.snapshot();
    Json(json!({
        "p2p_connections": snap.p2p_connections,
        "relay_connections": snap.relay_connections,
        "p2p_success_rate": snap.p2p_rate,
        "avg_latency_ms": null,
        "hole_punch_success": 0,
        "hole_punch_fail": 0,
    }))
}

pub async fn server_info(State(state): State<AppState>) -> Json<Value> {
    let cfg = state.config.read().clone();
    Json(json!({
        "status": "online",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": state.started_at.elapsed().as_secs(),
        "cpu": 0.0,
        "memory": 0.0,
        "listen_addr": cfg.listen,
        "ports": [
            { "name": "control", "port": cfg.control_port, "protocol": "tcp", "status": "listening" },
            { "name": "data", "port": cfg.data_port, "protocol": "tcp", "status": "listening" },
            { "name": "gateway", "port": cfg.gateway_port, "protocol": "tcp", "status": "listening" },
            { "name": "api", "port": cfg.api_port, "protocol": "tcp", "status": "listening" },
        ],
        "node_count": state.online.len(),
        "connection_count": state.connections.len(),
        "hostname": hostname(),
    }))
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "p2p-server".into())
}

pub async fn server_config(State(state): State<AppState>) -> Json<Value> {
    let cfg = state.config.read().clone();
    Json(json!({
        "listen_addr": cfg.listen,
        "api_port": cfg.api_port,
        "control_port": cfg.control_port,
        "data_port": cfg.data_port,
        "max_nodes": 10000,
        "heartbeat_interval_secs": 15,
        "enable_tls": false,
        "log_level": "info",
    }))
}

#[derive(Deserialize)]
pub struct ServerConfigUpdate {
    pub listen_addr: Option<String>,
    pub api_port: Option<u16>,
    pub control_port: Option<u16>,
    pub data_port: Option<u16>,
    #[allow(dead_code)]
    pub log_level: Option<String>,
}

pub async fn update_server_config(
    State(state): State<AppState>,
    Json(req): Json<ServerConfigUpdate>,
) -> Json<Value> {
    {
        let mut cfg = state.config.write();
        if let Some(v) = req.listen_addr {
            cfg.listen = v;
        }
        if let Some(v) = req.api_port {
            cfg.api_port = v;
        }
        if let Some(v) = req.control_port {
            cfg.control_port = v;
        }
        if let Some(v) = req.data_port {
            cfg.data_port = v;
        }
    }
    server_config(State(state)).await
}

pub async fn server_restart() -> Json<Value> {
    Json(json!({ "ok": true, "message": "restart scheduled (apply on next boot)" }))
}

#[derive(Deserialize)]
pub struct CreateNodeReq {
    pub name: String,
    pub node_id: Option<String>,
}

pub async fn list_nodes(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let rows = crate::db::list_nodes(&state.db)
        .await
        .map_err(AppError::internal)?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let id: String = r.get("node_id");
            let online = state.online.contains_key(&id);
            let (cpu, memory, uptime, conns, version, public_ip, nat) = if let Some(n) = state.online.get(&id)
            {
                (
                    Some(0.0_f64),
                    Some(0.0_f64),
                    Some(n.connected_at.elapsed().as_secs()),
                    Some(
                        state
                            .connections
                            .iter()
                            .filter(|c| c.node_id == id)
                            .count() as u64,
                    ),
                    n.version.clone(),
                    n.public_ip.clone(),
                    n.nat_type.clone(),
                )
            } else {
                (
                    None,
                    None,
                    None,
                    None,
                    r.get::<String, _>("version"),
                    r.try_get("public_ip").ok(),
                    r.try_get("nat_type").ok(),
                )
            };
            json!({
                "node_id": id,
                "name": r.get::<String, _>("name"),
                "status": if r.get::<i64, _>("enabled") != 1 {
                    "disabled"
                } else if online {
                    "online"
                } else {
                    "offline"
                },
                "public_ip": public_ip,
                "nat_type": nat,
                "version": version,
                "cpu": cpu,
                "memory": memory,
                "uptime_secs": uptime,
                "connections": conns,
                "created_at": r.get::<String, _>("created_at"),
            })
        })
        .collect();
    Ok(Json(json!(items)))
}

pub async fn create_node(
    State(state): State<AppState>,
    Json(req): Json<CreateNodeReq>,
) -> Result<Json<Value>, AppError> {
    let (node_id, token) = crate::db::create_node(&state.db, &req.name, req.node_id)
        .await
        .map_err(AppError::internal)?;
    let _ = crate::db::audit(&state.db, "admin", "node.create", &node_id).await;
    Ok(Json(json!({
        "node_id": node_id,
        "name": req.name,
        "status": "offline",
        "version": "0.1.0",
        "token": token,
    })))
}

pub async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let rows = list_nodes(State(state)).await?.0;
    let arr = rows.as_array().cloned().unwrap_or_default();
    arr.into_iter()
        .find(|n| n.get("node_id").and_then(|v| v.as_str()) == Some(id.as_str()))
        .map(Json)
        .ok_or_else(|| AppError::not_found("node not found"))
}

#[derive(Deserialize)]
pub struct UpdateNodeReq {
    pub name: Option<String>,
    pub enabled: Option<bool>,
}

pub async fn update_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNodeReq>,
) -> Result<Json<Value>, AppError> {
    if let Some(name) = req.name {
        sqlx::query("UPDATE nodes SET name = ? WHERE node_id = ?")
            .bind(name)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;
    }
    if let Some(enabled) = req.enabled {
        sqlx::query("UPDATE nodes SET enabled = ? WHERE node_id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;
    }
    get_node(State(state), Path(id)).await
}

pub async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    sqlx::query("DELETE FROM nodes WHERE node_id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn regen_token(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let token = p2p_security::generate_node_token();
    let hash = p2p_security::hash_token(&token);
    let r = sqlx::query("UPDATE nodes SET token_hash = ? WHERE node_id = ?")
        .bind(&hash)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;
    if r.rows_affected() == 0 {
        return Err(AppError::not_found("node not found"));
    }
    Ok(Json(json!({ "node_id": id, "token": token })))
}

pub async fn node_services(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let q = Query(HashMap::from([("node_id".into(), id)]));
    list_services(State(state), q).await
}

pub async fn node_routes(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let q = Query(HashMap::from([("node_id".into(), id)]));
    list_routes(State(state), q).await
}

pub async fn node_connections(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let items: Vec<Value> = state
        .connections
        .iter()
        .filter(|c| c.node_id == id)
        .map(|c| {
            json!({
                "conn_id": c.conn_id,
                "node_id": c.node_id,
                "mode": c.mode,
                "status": c.status,
                "protocol": c.protocol,
                "local_addr": c.local_addr,
                "remote_addr": c.remote_addr,
                "started_at": c.started_at.to_rfc3339(),
            })
        })
        .collect();
    Json(json!(items))
}

pub async fn node_traffic(
    State(state): State<AppState>,
    Path(_id): Path<String>,
) -> Json<Value> {
    traffic(State(state)).await
}

pub async fn node_logs(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let logs: Vec<_> = state
        .logs
        .read()
        .iter()
        .rev()
        .filter(|l| l.node_id.as_deref() == Some(id.as_str()))
        .take(100)
        .cloned()
        .collect();
    Json(json!(logs))
}

pub async fn list_services(
    State(state): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    let mut sql = "SELECT * FROM services WHERE 1=1".to_string();
    if q.get("node_id").is_some() {
        sql.push_str(" AND node_id = ?");
    }
    let mut query = sqlx::query(&sql);
    if let Some(nid) = q.get("node_id") {
        query = query.bind(nid);
    }
    let rows = query.fetch_all(&state.db).await.map_err(AppError::internal)?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let local: String = r.get("local_addr");
            let port = local
                .rsplit(':')
                .next()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(0);
            json!({
                "service_id": r.get::<String,_>("service_id"),
                "name": r.get::<String,_>("name"),
                "node_id": r.get::<String,_>("node_id"),
                "protocol": r.get::<String,_>("protocol"),
                "local_addr": local,
                "local_port": port,
                "enabled": r.get::<i64,_>("enabled") == 1,
                "description": r.try_get::<String,_>("description").ok(),
                "created_at": r.get::<String,_>("created_at"),
                "updated_at": r.get::<String,_>("updated_at"),
            })
        })
        .collect();
    Ok(Json(json!(items)))
}

#[derive(Deserialize)]
pub struct ServiceReq {
    pub name: String,
    pub node_id: String,
    pub protocol: String,
    pub local_addr: String,
    pub enabled: Option<bool>,
    pub description: Option<String>,
    pub service_id: Option<String>,
}

pub async fn create_service(
    State(state): State<AppState>,
    Json(req): Json<ServiceReq>,
) -> Result<Json<Value>, AppError> {
    let id = req
        .service_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO services (service_id, name, node_id, protocol, local_addr, enabled, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.node_id)
    .bind(&req.protocol)
    .bind(&req.local_addr)
    .bind(if req.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(&req.description)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;
    get_service(State(state), Path(id)).await
}

pub async fn get_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let list = list_services(State(state), Query(HashMap::new()))
        .await?
        .0;
    list.as_array()
        .into_iter()
        .flatten()
        .find(|s| s.get("service_id").and_then(|v| v.as_str()) == Some(id.as_str()))
        .cloned()
        .map(Json)
        .ok_or_else(|| AppError::not_found("service not found"))
}

pub async fn update_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ServiceReq>,
) -> Result<Json<Value>, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "UPDATE services SET name=?, node_id=?, protocol=?, local_addr=?, enabled=?, description=?, updated_at=? WHERE service_id=?",
    )
    .bind(&req.name)
    .bind(&req.node_id)
    .bind(&req.protocol)
    .bind(&req.local_addr)
    .bind(if req.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(&req.description)
    .bind(&now)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;
    get_service(State(state), Path(id)).await
}

pub async fn delete_service(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    sqlx::query("DELETE FROM services WHERE service_id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn list_routes(
    State(state): State<AppState>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Value>, AppError> {
    let mut sql = "SELECT * FROM routes WHERE 1=1".to_string();
    if q.get("node_id").is_some() {
        sql.push_str(" AND node_id = ?");
    }
    let mut query = sqlx::query(&sql);
    if let Some(nid) = q.get("node_id") {
        query = query.bind(nid);
    }
    let rows = query.fetch_all(&state.db).await.map_err(AppError::internal)?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "route_id": r.get::<String,_>("route_id"),
                "name": r.get::<String,_>("name"),
                "public_port": r.get::<i64,_>("public_port"),
                "protocol": r.get::<String,_>("protocol"),
                "node_id": r.get::<String,_>("node_id"),
                "service_id": r.get::<String,_>("service_id"),
                "enabled": r.get::<i64,_>("enabled") == 1,
                "description": r.try_get::<String,_>("description").ok(),
                "created_at": r.get::<String,_>("created_at"),
            })
        })
        .collect();
    Ok(Json(json!(items)))
}

#[derive(Deserialize)]
pub struct RouteReq {
    pub name: String,
    pub public_port: u16,
    pub protocol: String,
    pub node_id: String,
    pub service_id: String,
    pub enabled: Option<bool>,
    pub description: Option<String>,
    pub route_id: Option<String>,
}

pub async fn create_route(
    State(state): State<AppState>,
    Json(req): Json<RouteReq>,
) -> Result<Json<Value>, AppError> {
    let id = req
        .route_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO routes (route_id, name, public_port, protocol, node_id, service_id, enabled, description, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(req.public_port as i64)
    .bind(&req.protocol)
    .bind(&req.node_id)
    .bind(&req.service_id)
    .bind(if req.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(&req.description)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;
    crate::db::load_routes_into(&state.db, &state.routes)
        .await
        .map_err(AppError::internal)?;
    get_route(State(state), Path(id)).await
}

pub async fn get_route(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let list = list_routes(State(state), Query(HashMap::new()))
        .await?
        .0;
    list.as_array()
        .into_iter()
        .flatten()
        .find(|s| s.get("route_id").and_then(|v| v.as_str()) == Some(id.as_str()))
        .cloned()
        .map(Json)
        .ok_or_else(|| AppError::not_found("route not found"))
}

pub async fn update_route(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RouteReq>,
) -> Result<Json<Value>, AppError> {
    sqlx::query(
        "UPDATE routes SET name=?, public_port=?, protocol=?, node_id=?, service_id=?, enabled=?, description=? WHERE route_id=?",
    )
    .bind(&req.name)
    .bind(req.public_port as i64)
    .bind(&req.protocol)
    .bind(&req.node_id)
    .bind(&req.service_id)
    .bind(if req.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(&req.description)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;
    crate::db::load_routes_into(&state.db, &state.routes)
        .await
        .map_err(AppError::internal)?;
    get_route(State(state), Path(id)).await
}

pub async fn delete_route(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    sqlx::query("DELETE FROM routes WHERE route_id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;
    state.routes.remove(&id);
    Ok(Json(json!({ "ok": true })))
}

pub async fn get_settings(State(state): State<AppState>) -> Json<Value> {
    let cfg = state.config.read().clone();
    Json(json!({
        "server": {
            "listen_addr": cfg.listen,
            "api_port": cfg.api_port,
            "control_port": cfg.control_port,
            "data_port": cfg.data_port,
        },
        "security": {
            "enable_tls": false,
            "require_auth": true,
            "token_ttl_secs": cfg.jwt_ttl_secs,
            "allow_register": false,
        },
        "network": { "mtu": 1400, "keepalive_secs": 30, "dial_timeout_secs": 10 },
        "p2p": {
            "enable_hole_punch": cfg.enable_p2p,
            "stun_servers": ["stun:stun.l.google.com:19302"],
            "prefer_p2p": true,
            "fallback_relay": true,
        },
        "relay": {
            "enable": cfg.enable_relay,
            "max_bandwidth_mbps": 1000,
            "max_connections": cfg.max_connections,
        },
        "limits": {
            "max_nodes": 10000,
            "max_services_per_node": 100,
            "max_routes": 10000,
            "rate_limit_rps": 1000,
        },
        "logging": { "level": "info", "retention_days": 7, "enable_audit": true },
    }))
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(v): Json<Value>,
) -> Json<Value> {
    if let Some(server) = v.get("server") {
        let mut cfg = state.config.write();
        if let Some(p) = server.get("api_port").and_then(|x| x.as_u64()) {
            cfg.api_port = p as u16;
        }
        if let Some(p) = server.get("control_port").and_then(|x| x.as_u64()) {
            cfg.control_port = p as u16;
        }
        if let Some(p) = server.get("data_port").and_then(|x| x.as_u64()) {
            cfg.data_port = p as u16;
        }
    }
    get_settings(State(state)).await
}

pub async fn list_users(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let rows = sqlx::query("SELECT id, username, role, enabled, email, last_login, created_at FROM users")
        .fetch_all(&state.db)
        .await
        .map_err(AppError::internal)?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "user_id": r.get::<String,_>("id"),
                "username": r.get::<String,_>("username"),
                "role": r.get::<String,_>("role"),
                "enabled": r.get::<i64,_>("enabled") == 1,
                "email": r.try_get::<String,_>("email").ok(),
                "last_login": r.try_get::<String,_>("last_login").ok(),
                "created_at": r.get::<String,_>("created_at"),
            })
        })
        .collect();
    Ok(Json(json!(items)))
}

#[derive(Deserialize)]
pub struct UserReq {
    pub username: String,
    pub password: Option<String>,
    pub role: Option<String>,
    pub enabled: Option<bool>,
    pub email: Option<String>,
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<UserReq>,
) -> Result<Json<Value>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    let pass = req.password.unwrap_or_else(|| "changeme".into());
    let hash = p2p_security::hash_password(&pass).map_err(AppError::internal)?;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, enabled, email, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.username)
    .bind(&hash)
    .bind(req.role.unwrap_or_else(|| "viewer".into()))
    .bind(if req.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(&req.email)
    .bind(&now)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;
    Ok(Json(json!({
        "user_id": id,
        "username": req.username,
        "role": "viewer",
        "enabled": true,
        "created_at": now,
    })))
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UserReq>,
) -> Result<Json<Value>, AppError> {
    if let Some(pass) = req.password {
        let hash = p2p_security::hash_password(&pass).map_err(AppError::internal)?;
        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(hash)
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;
    }
    sqlx::query("UPDATE users SET username = ?, role = COALESCE(?, role), enabled = COALESCE(?, enabled), email = COALESCE(?, email) WHERE id = ?")
        .bind(&req.username)
        .bind(&req.role)
        .bind(req.enabled.map(|e| if e { 1 } else { 0 }))
        .bind(&req.email)
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;
    Ok(Json(json!({ "user_id": id, "username": req.username, "enabled": true, "role": req.role.unwrap_or_else(|| "viewer".into()) })))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;
    Ok(Json(json!({ "ok": true })))
}
