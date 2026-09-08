use hex::encode;
use p2p_common::NodeId;
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Generate a random node authentication token (hex).
pub fn generate_node_token() -> String {
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    encode(buf)
}

/// Hash a token for storage (never store plaintext tokens when avoidable).
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    encode(hasher.finalize())
}

pub fn verify_node_token(node_id: &NodeId, provided: &str, expected_hash: &str) -> bool {
    let _ = node_id;
    hash_token(provided) == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_hash_verify() {
        let tok = generate_node_token();
        let h = hash_token(&tok);
        assert!(verify_node_token(&NodeId::new("n1"), &tok, &h));
        assert!(!verify_node_token(&NodeId::new("n1"), "bad", &h));
    }
}
