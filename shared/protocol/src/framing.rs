use crate::ControlMessage;
use bytes::{Buf, BufMut, BytesMut};
use p2p_common::Error;
use std::io;

/// Maximum control message payload size (16 MiB).
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Encode a control message as `u32 BE length + UTF-8 JSON`.
pub fn encode_message(msg: &ControlMessage) -> Result<Vec<u8>, Error> {
    let json = serde_json::to_vec(msg)?;
    if json.len() > MAX_FRAME_SIZE {
        return Err(Error::protocol(format!(
            "frame too large: {} bytes",
            json.len()
        )));
    }
    let mut out = Vec::with_capacity(4 + json.len());
    out.put_u32(json.len() as u32);
    out.extend_from_slice(&json);
    Ok(out)
}

/// Decode a complete frame from a buffer. Returns `None` if incomplete.
pub fn try_decode_message(buf: &mut BytesMut) -> Result<Option<ControlMessage>, Error> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let mut length_bytes = [0u8; 4];
    length_bytes.copy_from_slice(&buf[..4]);
    let len = u32::from_be_bytes(length_bytes) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(Error::protocol(format!("frame length {len} exceeds max")));
    }
    if buf.len() < 4 + len {
        return Ok(None);
    }
    buf.advance(4);
    let payload = buf.split_to(len);
    let msg: ControlMessage = serde_json::from_slice(&payload)?;
    Ok(Some(msg))
}

/// Decode exactly one message from a reader-owned byte slice (full frame).
pub fn decode_frame(frame: &[u8]) -> Result<ControlMessage, Error> {
    if frame.len() < 4 {
        return Err(Error::protocol("frame too short"));
    }
    let len = u32::from_be_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    if frame.len() != 4 + len {
        return Err(Error::protocol(format!(
            "frame size mismatch: have {}, expected {}",
            frame.len(),
            4 + len
        )));
    }
    Ok(serde_json::from_slice(&frame[4..])?)
}

/// Data-plane handshake line: `DATA {connection_id} {data_token}\n`
pub fn encode_data_handshake(connection_id: &str, data_token: &str) -> Vec<u8> {
    format!("DATA {connection_id} {data_token}\n").into_bytes()
}

/// Parse data handshake line.
pub fn parse_data_handshake(line: &str) -> Result<(String, String), Error> {
    let line = line.trim();
    let mut parts = line.split_whitespace();
    let tag = parts
        .next()
        .ok_or_else(|| Error::protocol("empty data handshake"))?;
    if tag != "DATA" {
        return Err(Error::protocol(format!("expected DATA, got {tag}")));
    }
    let connection_id = parts
        .next()
        .ok_or_else(|| Error::protocol("missing connection_id"))?
        .to_string();
    let data_token = parts
        .next()
        .ok_or_else(|| Error::protocol("missing data_token"))?
        .to_string();
    Ok((connection_id, data_token))
}

/// Read a single line ending with `\n` from async-compatible buffer logic helper.
pub fn extract_line(buf: &mut BytesMut) -> Option<String> {
    if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
        let line = buf.split_to(pos + 1);
        let s = String::from_utf8_lossy(&line).to_string();
        Some(s)
    } else {
        None
    }
}

/// Helper used by sync tests / simple readers.
pub fn read_exact_frame_sync(mut reader: impl io::Read) -> Result<ControlMessage, Error> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(Error::protocol("frame too large"));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ControlMessage;

    #[test]
    fn encode_decode_roundtrip() {
        let msg = ControlMessage::Heartbeat {
            seq: 42,
            rtt_ms: Some(12),
        };
        let encoded = encode_message(&msg).unwrap();
        let mut buf = BytesMut::from(&encoded[..]);
        let decoded = try_decode_message(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.message_type(), "HEARTBEAT");
        assert!(buf.is_empty());
    }

    #[test]
    fn incomplete_frame() {
        let msg = ControlMessage::Hello {
            version: "1".into(),
            features: vec![],
        };
        let encoded = encode_message(&msg).unwrap();
        let mut buf = BytesMut::from(&encoded[..encoded.len() - 2]);
        assert!(try_decode_message(&mut buf).unwrap().is_none());
    }

    #[test]
    fn data_handshake_roundtrip() {
        let bytes = encode_data_handshake("cid-1", "tok-abc");
        let line = String::from_utf8(bytes).unwrap();
        let (cid, tok) = parse_data_handshake(&line).unwrap();
        assert_eq!(cid, "cid-1");
        assert_eq!(tok, "tok-abc");
    }
}
