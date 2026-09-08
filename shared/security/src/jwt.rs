use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use p2p_common::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Clone)]
pub struct JwtManager {
    secret: String,
    ttl_secs: i64,
}

impl JwtManager {
    pub fn new(secret: impl Into<String>, ttl_secs: i64) -> Self {
        Self {
            secret: secret.into(),
            ttl_secs,
        }
    }

    pub fn issue(&self, user_id: &str, role: &str) -> Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            role: role.to_string(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(self.ttl_secs)).timestamp(),
        };
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| Error::internal(format!("jwt encode: {e}")))
    }

    pub fn verify(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map(|d| d.claims)
        .map_err(|_| Error::InvalidToken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_roundtrip() {
        let m = JwtManager::new("test-secret", 3600);
        let t = m.issue("u1", "admin").unwrap();
        let c = m.verify(&t).unwrap();
        assert_eq!(c.sub, "u1");
        assert_eq!(c.role, "admin");
    }
}
