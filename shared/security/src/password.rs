use p2p_common::{Error, Result};

pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| Error::internal(format!("bcrypt hash: {e}")))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let h = hash_password("admin123").unwrap();
        assert!(verify_password("admin123", &h));
        assert!(!verify_password("wrong", &h));
    }
}
