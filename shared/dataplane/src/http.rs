//! HTTP gateway helpers (host-based routing scaffold).

use p2p_common::Result;

/// Extract Host header value from a raw HTTP request prefix (best-effort).
pub fn extract_host(request_prefix: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(request_prefix).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Host:") {
            return Some(rest.trim().split(':').next()?.to_string());
        }
        if let Some(rest) = line.strip_prefix("host:") {
            return Some(rest.trim().split(':').next()?.to_string());
        }
    }
    None
}

/// Minimal HTTP 502 response body.
pub fn http_bad_gateway(reason: &str) -> Vec<u8> {
    let body = format!("Bad Gateway: {reason}");
    format!(
        "HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// Placeholder for future HTTP-aware proxy features.
pub async fn handle_http_passthrough() -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_host() {
        let req = b"GET / HTTP/1.1\r\nHost: example.com:8080\r\n\r\n";
        assert_eq!(extract_host(req).as_deref(), Some("example.com"));
    }
}
