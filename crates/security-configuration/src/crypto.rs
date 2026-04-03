//! Cryptographic utilities for security configuration.
//!
//! This module provides cryptographic operations that can be used by security policies,
//! such as key generation, signing, encryption, and hash verification.
//!
//! Requires the `crypto` feature.

use crate::error::{Result, SecurityConfigError};
use base64::{engine::general_purpose, Engine as _};
use ring::rand::SystemRandom;
use ring::signature::{self, Ed25519KeyPair, KeyPair};
use std::sync::Arc;

/// A cryptographic key pair (Ed25519 by default).
pub struct CryptoKeyPair {
    key_pair: Arc<Ed25519KeyPair>,
    rng: SystemRandom,
}

impl CryptoKeyPair {
    /// Generates a new Ed25519 key pair.
    pub fn generate() -> Result<Self> {
        let rng = SystemRandom::new();
        let pkcs8_bytes = signature::Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?;
        let key_pair = signature::Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?;

        Ok(Self {
            key_pair: Arc::new(key_pair),
            rng,
        })
    }

    /// Returns the public key as bytes.
    pub fn public_key(&self) -> Vec<u8> {
        self.key_pair.public_key().as_ref().to_vec()
    }

    /// Returns the public key as a base64‑encoded string.
    pub fn public_key_base64(&self) -> String {
        general_purpose::STANDARD.encode(self.public_key())
    }

    /// Signs a message and returns the signature as bytes.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.key_pair.sign(message).as_ref().to_vec()
    }

    /// Signs a message and returns the signature as a base64‑encoded string.
    pub fn sign_base64(&self, message: &[u8]) -> String {
        general_purpose::STANDARD.encode(self.sign(message))
    }

    /// Verifies a signature against a message using the given public key.
    pub fn verify(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<()> {
        let peer_public_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key);
        peer_public_key
            .verify(message, signature)
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))
    }
}

/// Symmetric encryption using AES‑GCM (via ring).
pub struct SymmetricCrypto;

impl SymmetricCrypto {
    /// Encrypts plaintext with a key and nonce.
    ///
    /// The key must be 32 bytes (AES‑256). The nonce must be 12 bytes.
    /// Returns the ciphertext (without the nonce).
    pub fn encrypt(key: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        use ring::aead;

        if key.len() != 32 {
            return Err(SecurityConfigError::Crypto(
                "Key must be 32 bytes for AES‑256‑GCM".to_string(),
            ));
        }
        if nonce.len() != 12 {
            return Err(SecurityConfigError::Crypto(
                "Nonce must be 12 bytes".to_string(),
            ));
        }

        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?;
        let nonce = aead::Nonce::try_assume_unique_for_key(nonce)
            .map_err(|_| SecurityConfigError::Crypto("Invalid nonce".to_string()))?;
        let sealing_key = aead::LessSafeKey::new(unbound_key);

        let mut in_out = plaintext.to_vec();
        sealing_key
            .seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut in_out)
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?;

        Ok(in_out)
    }

    /// Decrypts ciphertext with a key and nonce.
    pub fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        use ring::aead;

        if key.len() != 32 {
            return Err(SecurityConfigError::Crypto(
                "Key must be 32 bytes for AES‑256‑GCM".to_string(),
            ));
        }
        if nonce.len() != 12 {
            return Err(SecurityConfigError::Crypto(
                "Nonce must be 12 bytes".to_string(),
            ));
        }

        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?;
        let nonce = aead::Nonce::try_assume_unique_for_key(nonce)
            .map_err(|_| SecurityConfigError::Crypto("Invalid nonce".to_string()))?;
        let opening_key = aead::LessSafeKey::new(unbound_key);

        let mut in_out = ciphertext.to_vec();
        let plaintext_len = opening_key
            .open_in_place(nonce, aead::Aad::empty(), &mut in_out)
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?
            .len();

        in_out.truncate(plaintext_len);
        Ok(in_out)
    }
}

/// Hash functions.
pub struct Hasher;

impl Hasher {
    /// Computes SHA‑256 hash of the input.
    pub fn sha256(data: &[u8]) -> Vec<u8> {
        use ring::digest;
        digest::digest(&digest::SHA256, data).as_ref().to_vec()
    }

    /// Computes SHA‑256 hash and returns it as a hex string.
    pub fn sha256_hex(data: &[u8]) -> String {
        hex::encode(Self::sha256(data))
    }

    /// Computes SHA‑256 hash and returns it as a base64 string.
    pub fn sha256_base64(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(Self::sha256(data))
    }
}

/// Password‑based key derivation (requires argon2 feature).
#[cfg(feature = "argon2")]
pub struct PasswordKdf;

#[cfg(feature = "argon2")]
impl PasswordKdf {
    /// Derives a key from a password and salt using Argon2id.
    pub fn derive_key(
        password: &[u8],
        salt: &[u8],
        iterations: u32,
        memory_kib: u32,
        parallelism: u32,
        output_len: usize,
    ) -> Result<Vec<u8>> {
        use argon2::{Algorithm, Argon2, Params, Version};

        let params = Params::new(memory_kib, iterations, parallelism, Some(output_len))
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut output = vec![0u8; output_len];
        argon2
            .hash_password_into(password, salt, &mut output)
            .map_err(|e| SecurityConfigError::Crypto(e.to_string()))?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_pair_sign_verify() {
        let key_pair = CryptoKeyPair::generate().unwrap();
        let message = b"hello world";
        let signature = key_pair.sign(message);
        assert!(CryptoKeyPair::verify(
            &key_pair.public_key(),
            message,
            &signature
        )
        .is_ok());
    }

    #[test]
    fn test_symmetric_encryption() {
        let key = b"0123456789abcdef0123456789abcdef"; // 32 bytes
        let nonce = b"nonce1234567"; // 12 bytes
        let plaintext = b"secret message";
        let ciphertext = SymmetricCrypto::encrypt(key, nonce, plaintext).unwrap();
        let decrypted = SymmetricCrypto::decrypt(key, nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_hasher() {
        let data = b"test";
        let hash = Hasher::sha256(data);
        assert_eq!(hash.len(), 32);
        let hex = Hasher::sha256_hex(data);
        assert_eq!(hex.len(), 64);
        let b64 = Hasher::sha256_base64(data);
        assert!(!b64.is_empty());
    }

    #[cfg(feature = "argon2")]
    #[test]
    fn test_password_kdf() {
        let password = b"password";
        let salt = b"salt1234";
        let key = PasswordKdf::derive_key(password, salt, 3, 4096, 1, 32).unwrap();
        assert_eq!(key.len(), 32);
    }
}