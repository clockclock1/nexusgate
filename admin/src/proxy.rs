use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use futures::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use tokio_tungstenite::tungstenite::Message as TsMessage;

use crate::AppState;

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "host",
    "content-length",
];

fn filter_request_headers(src: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (k, v) in src.iter() {
        let name = k.as_str();
        if HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(name)) {
            continue;
        }
        out.append(k.clone(), v.clone());
    }
    out
}

fn filter_response_headers(src: &reqwest::header::HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (k, v) in src.iter() {
        let name = k.as_str();
        if HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(name)) {
            continue;
        }
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(k.as_str().as_bytes()),
            HeaderValue::from_bytes(v.as_bytes()),
        ) {
            out.append(name, val);
        }
    }
    out
}

/// Reverse-proxy any `/api/*` (and other non-ws) HTTP request to the management API.
pub async fn proxy_http(State(state): State<AppState>, req: Request) -> Response {
    let method = req.method().clone();
    let headers = filter_request_headers(req.headers());
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let url = format!("{}{}", state.http_upstream, path_and_query);

    let body_bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            tracing::warn!(error = %e, "proxy read body failed");
            return (StatusCode::BAD_GATEWAY, "bad gateway").into_response();
        }
    };

    let mut builder = state.http.request(
        Method::from_bytes(method.as_str().as_bytes()).unwrap_or(Method::GET),
        &url,
    );
    for (k, v) in headers.iter() {
        builder = builder.header(k, v);
    }

    let upstream = match builder.body(body_bytes).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, %url, "proxy upstream request failed");
            return (StatusCode::BAD_GATEWAY, format!("upstream error: {e}")).into_response();
        }
    };

    let status = StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let resp_headers = filter_response_headers(upstream.headers());
    let bytes = match upstream.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "proxy read upstream body failed");
            return (StatusCode::BAD_GATEWAY, "bad gateway").into_response();
        }
    };

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;
    response
}

pub async fn proxy_ws(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    uri: Uri,
) -> impl IntoResponse {
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/ws");
    let upstream = format!("{}{}", state.ws_upstream, path_and_query);
    ws.on_upgrade(move |socket| bridge_ws(socket, upstream))
}

async fn bridge_ws(client: WebSocket, upstream_url: String) {
    let (upstream, _) = match tokio_tungstenite::connect_async(&upstream_url).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, %upstream_url, "ws upstream connect failed");
            return;
        }
    };

    let (mut client_tx, mut client_rx) = client.split();
    let (mut up_tx, mut up_rx) = upstream.split();

    let client_to_up = async {
        while let Some(Ok(msg)) = client_rx.next().await {
            let mapped = match msg {
                AxumMessage::Text(t) => TsMessage::Text(t.into()),
                AxumMessage::Binary(b) => TsMessage::Binary(b.into()),
                AxumMessage::Ping(p) => TsMessage::Ping(p.into()),
                AxumMessage::Pong(p) => TsMessage::Pong(p.into()),
                AxumMessage::Close(_) => {
                    let _ = up_tx.close().await;
                    break;
                }
            };
            if up_tx.send(mapped).await.is_err() {
                break;
            }
        }
    };

    let up_to_client = async {
        while let Some(Ok(msg)) = up_rx.next().await {
            let mapped = match msg {
                TsMessage::Text(t) => AxumMessage::Text(t.to_string()),
                TsMessage::Binary(b) => AxumMessage::Binary(b.to_vec()),
                TsMessage::Ping(p) => AxumMessage::Ping(p.to_vec()),
                TsMessage::Pong(p) => AxumMessage::Pong(p.to_vec()),
                TsMessage::Close(_) => {
                    let _ = client_tx.send(AxumMessage::Close(None)).await;
                    break;
                }
                TsMessage::Frame(_) => continue,
            };
            if client_tx.send(mapped).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = client_to_up => {}
        _ = up_to_client => {}
    }
}
