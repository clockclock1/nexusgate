use thiserror::Error;

/// Shared error type used across crates.
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("authorization failed: {0}")]
    Forbidden(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("connection closed: {0}")]
    Closed(String),

    #[error("invalid token")]
    InvalidToken,

    #[error("node offline: {0}")]
    NodeOffline(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Config(e.to_string())
    }
}

impl Error {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    pub fn auth(msg: impl Into<String>) -> Self {
        Self::Auth(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let e = Error::auth("bad credentials");
        assert!(e.to_string().contains("bad credentials"));
    }
}
