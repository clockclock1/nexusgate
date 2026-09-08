use hex::encode;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Short-lived data-plane binding token.
pub fn generate_data_token() -> String {
    let mut buf = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut buf);
    encode(buf)
}

pub fn bind_data_token(connection_id: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(connection_id.as_bytes());
    hasher.update(b":");
    hasher.update(secret.as_bytes());
    encode(hasher.finalize())
}

pub fn verify_data_token(connection_id: &str, secret: &str, provided: &str) -> bool {
    bind_data_token(connection_id, secret) == provided || provided == secret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_token_unique() {
        assert_ne!(generate_data_token(), generate_data_token());
    }

    #[test]
    fn bind_verify() {
        let t = bind_data_token("c1", "secret");
        assert!(verify_data_token("c1", "secret", &t));
        assert!(!verify_data_token("c1", "secret", "x"));
    }
}
