use crate::api::auth::AppError;
use crate::state::AppState;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use std::time::Duration;

pub async fn ws_dashboard(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| push_loop(socket, state, "dashboard"))
}

pub async fn ws_logs(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| push_loop(socket, state, "logs"))
}

pub async fn ws_connections(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| push_loop(socket, state, "connections"))
}

pub async fn ws_traffic(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| push_loop(socket, state, "traffic"))
}

async fn push_loop(socket: WebSocket, state: AppState, kind: &'static str) {
    let (mut sender, mut receiver) = socket.split();
    let mut events = state.event_tx.subscribe();
    let mut tick = tokio::time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let payload = match kind {
                    "dashboard" => {
                        let snap = state.metrics.snapshot();
                        serde_json::json!({
                            "type": "metrics",
                            "payload": {
                                "nodes_online": snap.nodes_online,
                                "connections_active": snap.connections_active,
                                "traffic_rx_bytes": snap.rx_bytes,
                                "traffic_tx_bytes": snap.tx_bytes,
                                "p2p_rate": snap.p2p_rate,
                                "relay_rate": snap.relay_rate,
                            }
                        })
                    }
                    "traffic" => {
                        serde_json::json!({
                            "type": "traffic",
                            "payload": state.metrics.history().last()
                        })
                    }
                    "connections" => {
                        let items: Vec<_> = state.connections.iter().map(|c| c.value().clone()).collect();
                        serde_json::json!({ "type": "connections", "payload": items })
                    }
                    "logs" => {
                        let logs = state.logs.read().iter().rev().take(20).cloned().collect::<Vec<_>>();
                        serde_json::json!({ "type": "logs", "payload": logs })
                    }
                    _ => serde_json::json!({ "type": "ping" }),
                };
                if sender
                    .send(Message::Text(payload.to_string()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            evt = events.recv() => {
                if let Ok(v) = evt {
                    if sender.send(Message::Text(v.to_string())).await.is_err() {
                        break;
                    }
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(p))) => {
                        let _ = sender.send(Message::Pong(p)).await;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[allow(dead_code)]
fn _err() -> AppError {
    AppError::internal("unused")
}
