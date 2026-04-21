//! Password hashing and verification.

use anyhow::{Result, anyhow};
use bcrypt::{hash, verify, DEFAULT_COST};
use tracing::warn;

/// Password hasher.
pub struct PasswordHasher;

impl PasswordHasher {
    /// Hash a password.
    pub fn hash(password: &str) -> Result<String> {
        hash(password, DEFAULT_COST)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))
    }

    /// Verify a password against a hash.
    pub fn verify(password: &str, hash: &str) -> Result<bool> {
        verify(password, hash)
            .map_err(|e| {
                warn!("Password verification error: {}", e);
                anyhow!("Password verification failed: {}", e)
            })
    }

    /// Check if password matches hash.
    pub fn check(password: &str, hash: &str) -> bool {
        Self::verify(password, hash).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let password = "secure_password_123";

        // Hash password
        let hash = PasswordHasher::hash(password).unwrap();
        assert!(!hash.is_empty());
        assert_ne!(hash, password);

        // Verify correct password
        assert!(PasswordHasher::verify(password, &hash).unwrap());

        // Verify incorrect password
        assert!(!PasswordHasher::verify("wrong_password", &hash).unwrap());

        // Check method
        assert!(PasswordHasher::check(password, &hash));
        assert!(!PasswordHasher::check("wrong", &hash));
    }

    #[test]
    fn test_hash_uniqueness() {
        let password = "password";

        // Hash twice - should be different (due to salt)
        let hash1 = PasswordHasher::hash(password).unwrap();
        let hash2 = PasswordHasher::hash(password).unwrap();

        assert_ne!(hash1, hash2);

        // Both should verify
        assert!(PasswordHasher::verify(password, &hash1).unwrap());
        assert!(PasswordHasher::verify(password, &hash2).unwrap());
    }
}
