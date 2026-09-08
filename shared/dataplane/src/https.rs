//! HTTPS / TLS SNI extraction scaffold.

use p2p_common::{Error, Result};

/// Best-effort SNI extraction from a TLS ClientHello (no full TLS stack).
pub fn extract_sni(client_hello: &[u8]) -> Result<Option<String>> {
    // TLS record: type(1)=22 handshake, ver(2), len(2), handshake...
    if client_hello.len() < 5 {
        return Ok(None);
    }
    if client_hello[0] != 0x16 {
        return Err(Error::protocol("not a TLS handshake record"));
    }
    let record_len = u16::from_be_bytes([client_hello[3], client_hello[4]]) as usize;
    if client_hello.len() < 5 + record_len.min(client_hello.len().saturating_sub(5)) {
        // incomplete — caller should buffer more
        return Ok(None);
    }
    let hs = &client_hello[5..];
    if hs.is_empty() || hs[0] != 0x01 {
        return Ok(None); // not ClientHello
    }
    if hs.len() < 38 {
        return Ok(None);
    }
    // Skip: msg_type(1) + len(3) + version(2) + random(32) = 38
    let mut pos = 38;
    if hs.len() < pos + 1 {
        return Ok(None);
    }
    let session_id_len = hs[pos] as usize;
    pos += 1 + session_id_len;
    if hs.len() < pos + 2 {
        return Ok(None);
    }
    let cipher_len = u16::from_be_bytes([hs[pos], hs[pos + 1]]) as usize;
    pos += 2 + cipher_len;
    if hs.len() < pos + 1 {
        return Ok(None);
    }
    let comp_len = hs[pos] as usize;
    pos += 1 + comp_len;
    if hs.len() < pos + 2 {
        return Ok(None);
    }
    let ext_len = u16::from_be_bytes([hs[pos], hs[pos + 1]]) as usize;
    pos += 2;
    let ext_end = pos + ext_len;
    if hs.len() < ext_end {
        return Ok(None);
    }
    while pos + 4 <= ext_end {
        let typ = u16::from_be_bytes([hs[pos], hs[pos + 1]]);
        let len = u16::from_be_bytes([hs[pos + 2], hs[pos + 3]]) as usize;
        pos += 4;
        if pos + len > ext_end {
            break;
        }
        if typ == 0 {
            // server_name
            if len < 2 {
                break;
            }
            let mut p = pos + 2; // skip list length
            while p + 3 <= pos + len {
                let name_type = hs[p];
                let name_len = u16::from_be_bytes([hs[p + 1], hs[p + 2]]) as usize;
                p += 3;
                if p + name_len > pos + len {
                    break;
                }
                if name_type == 0 {
                    let name = String::from_utf8_lossy(&hs[p..p + name_len]).to_string();
                    return Ok(Some(name));
                }
                p += name_len;
            }
        }
        pos += len;
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_tls_rejected() {
        let r = extract_sni(b"GET / HTTP/1.1\r\n");
        assert!(r.is_err());
    }

    #[test]
    fn short_buffer() {
        assert!(extract_sni(&[0x16, 0x03, 0x01]).unwrap().is_none());
    }
}
