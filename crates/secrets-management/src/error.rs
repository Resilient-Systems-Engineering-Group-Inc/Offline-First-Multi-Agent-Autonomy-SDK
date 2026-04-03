//! Error types for secrets management.

use thiserror::Error;

/// Result alias for secrets operations.
pub type Result<T> = std::result::Result<T, SecretsError>;

/// Main error type for secrets management.
#[derive(Error, Debug)]
pub enum SecretsError {
    /// Secret not found.
    #[error("secret not found: {0}")]
    NotFound(String),

    /// Secret already exists.
    #[error("secret already exists: {0}")]
    AlreadyExists(String),

    /// Access denied due to policy.
    #[error("access denied to secret {0}: {1}")]
    AccessDenied(String, String),

    /// Encryption/decryption error.
    #[error("crypto error: {0}")]
    Crypto(String),

    /// Invalid secret format.
    #[error("invalid secret format: {0}")]
    InvalidFormat(String),

    /// Backend-specific error.
    #[error("backend error: {0}")]
    Backend(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Network error.
    #[error("network error: {0}")]
    Network(String),

    /// Timeout error.
    #[error("operation timed out: {0}")]
    Timeout(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// Key rotation error.
    #[error("key rotation error: {0}")]
    Rotation(String),

    /// Transport error.
    #[error("transport error: {0}")]
    Transport(String),

    /// Unknown error.
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl From<ring::error::Unspecified> for SecretsError {
    fn from(err: ring::error::Unspecified) -> Self {
        SecretsError::Crypto(format!("ring error: {:?}", err))
    }
}

impl From<aes_gcm::Error> for SecretsError {
    fn from(err: aes_gcm::Error) -> Self {
        SecretsError::Crypto(format!("AES-GCM error: {:?}", err))
    }
}

impl From<chacha20poly1305::Error> for SecretsError {
    fn from(err: chacha20poly1305::Error) -> Self {
        SecretsError::Crypto(format!("ChaCha20-Poly1305 error: {:?}", err))
    }
}