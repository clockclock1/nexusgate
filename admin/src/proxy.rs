use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
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
    // consumed by admin proxy for routing
    "x-nexus-server",
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

fn server_id_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-nexus-server")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn server_id_from_uri(uri: &Uri) -> Option<String> {
    uri.query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or("");
            if k == "server" && !v.is_empty() {
                Some(urlencoding_decode(v))
            } else {
                None
            }
        })
    })
}

fn urlencoding_decode(s: &str) -> String {
    // minimal decode for common cases; ids are usually alphanumeric
    percent_encoding_decode(s)
}

fn percent_encoding_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn strip_server_query(path_and_query: &str) -> String {
    let Some((path, query)) = path_and_query.split_once('?') else {
        return path_and_query.to_string();
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("server=") && *p != "server")
        .collect();
    if kept.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{}", kept.join("&"))
    }
}

/// Reverse-proxy any `/api/*` HTTP request to the selected Super Node.
/// Prefer Hub management forward when that server is dialed into the Admin Hub;
/// otherwise fall back to HTTP `api_upstream`.
pub async fn proxy_http(State(state): State<AppState>, req: Request) -> Response {
    let method = req.method().clone();
    let server_id = server_id_from_headers(req.headers()).or_else(|| server_id_from_uri(req.uri()));
    let entry = match state.registry.resolve_entry(server_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("server select error: {e}")).into_response();
        }
    };

    let headers = filter_request_headers(req.headers());
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let path_and_query = strip_server_query(path_and_query);

    let body_bytes = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            tracing::warn!(error = %e, "proxy read body failed");
            return (StatusCode::BAD_GATEWAY, "bad gateway").into_response();
        }
    };

    // Hub path: server peer online on management mesh
    if let Some(hub) = &state.hub {
        if hub.is_server_online(&entry.id) {
            let hdrs: Vec<(String, String)> = headers
                .iter()
                .filter_map(|(k, v)| {
                    v.to_str()
                        .ok()
                        .map(|val| (k.as_str().to_string(), val.to_string()))
                })
                .collect();
            match hub
                .mgmt_forward(
                    &entry.id,
                    method.as_str(),
                    &path_and_query,
                    hdrs,
                    if body_bytes.is_empty() {
                        None
                    } else {
                        Some(body_bytes.to_vec())
                    },
                    std::time::Duration::from_secs(30),
                )
                .await
            {
                Ok(p2p_protocol::ControlMessage::MgmtForwardResult {
                    status,
                    headers: resp_hdrs,
                    body_b64,
                    error,
                    ..
                }) => {
                    if let Some(err) = error {
                        return (StatusCode::BAD_GATEWAY, format!("hub mgmt error: {err}"))
                            .into_response();
                    }
                    let bytes = body_b64
                        .and_then(|b| {
                            use base64::Engine;
                            base64::engine::general_purpose::STANDARD.decode(b).ok()
                        })
                        .unwrap_or_default();
                    let mut response = Response::new(Body::from(bytes));
                    *response.status_mut() =
                        StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
                    let mut out = HeaderMap::new();
                    for (k, v) in resp_hdrs {
                        if let (Ok(name), Ok(val)) = (
                            HeaderName::from_bytes(k.as_bytes()),
                            HeaderValue::from_str(&v),
                        ) {
                            out.append(name, val);
                        }
                    }
                    *response.headers_mut() = out;
                    return response;
                }
                Ok(_) => {
                    return (StatusCode::BAD_GATEWAY, "unexpected hub response").into_response();
                }
                Err(e) => {
                    tracing::warn!(error = %e, server = %entry.id, "hub mgmt forward failed, trying HTTP");
                }
            }
        }
    }

    let http_upstream = crate::config::http_base(&entry.api_upstream);
    let url = format!("{http_upstream}{path_and_query}");

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
    let server_id = server_id_from_uri(&uri);
    let (_, ws_upstream) = match state.registry.resolve(server_id.as_deref()) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("server select error: {e}")).into_response();
        }
    };
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/ws");
    let path_and_query = strip_server_query(path_and_query);
    let upstream = format!("{ws_upstream}{path_and_query}");
    ws.on_upgrade(move |socket| bridge_ws(socket, upstream))
        .into_response()
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
