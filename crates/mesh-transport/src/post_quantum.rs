//! Post‑quantum cryptography support for mesh transport.
//!
//! This module provides quantum‑resistant algorithms for key exchange (Kyber),
//! digital signatures (Dilithium, Falcon), and hybrid encryption.
//! It is gated behind the `post‑quantum` feature.

use pqcrypto_kyber::{
    kyber1024,
    kyber1024::*,
};
use pqcrypto_dilithium::dilithium5::*;
use pqcrypto_falcon::falcon1024::*;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Errors specific to post‑quantum cryptography.
#[derive(Error, Debug)]
pub enum PostQuantumError {
    #[error("Key generation failed")]
    KeyGen,
    #[error("Encapsulation failed")]
    Encaps,
    #[error("Decapsulation failed")]
    Decaps,
    #[error("Signature generation failed")]
    Sign,
    #[error("Signature verification failed")]
    Verify,
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Invalid ciphertext length")]
    InvalidCiphertextLength,
    #[error("Serialization error")]
    Serialization(#[from] bincode::Error),
}

impl From<PostQuantumError> for crate::security::SecurityError {
    fn from(err: PostQuantumError) -> Self {
        crate::security::SecurityError::PostQuantum(err.to_string())
    }
}

/// A Kyber key pair for key encapsulation.
pub struct KyberKeyPair {
    public_key: PublicKey,
    secret_key: SecretKey,
}

impl KyberKeyPair {
    /// Generate a new Kyber‑1024 key pair.
    pub fn generate() -> Result<Self, PostQuantumError> {
        let (public_key, secret_key) = keypair().map_err(|_| PostQuantumError::KeyGen)?;
        Ok(Self { public_key, secret_key })
    }

    /// Get the public key bytes.
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.as_bytes().to_vec()
    }

    /// Encapsulate a shared secret to this public key.
    /// Returns (ciphertext, shared_secret).
    pub fn encapsulate(&self) -> Result<(Vec<u8>, Vec<u8>), PostQuantumError> {
        let (ciphertext, shared_secret) = encapsulate(&self.public_key)
            .map_err(|_| PostQuantumError::Encaps)?;
        Ok((ciphertext.as_bytes().to_vec(), shared_secret.as_bytes().to_vec()))
    }

    /// Decapsulate a ciphertext using the secret key.
    pub fn decapsulate(&self, ciphertext: &[u8]) -> Result<Vec<u8>, PostQuantumError> {
        let ct = Ciphertext::from_bytes(ciphertext)
            .map_err(|_| PostQuantumError::InvalidCiphertextLength)?;
        let shared_secret = decapsulate(&ct, &self.secret_key)
            .map_err(|_| PostQuantumError::Decaps)?;
        Ok(shared_secret.as_bytes().to_vec())
    }
}

/// A Dilithium key pair for digital signatures.
pub struct DilithiumKeyPair {
    public_key: PublicKey,
    secret_key: SecretKey,
}

impl DilithiumKeyPair {
    /// Generate a new Dilithium‑5 key pair.
    pub fn generate() -> Result<Self, PostQuantumError> {
        let (public_key, secret_key) = keypair().map_err(|_| PostQuantumError::KeyGen)?;
        Ok(Self { public_key, secret_key })
    }

    /// Get the public key bytes.
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.as_bytes().to_vec()
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, PostQuantumError> {
        let signature = sign(message, &self.secret_key)
            .map_err(|_| PostQuantumError::Sign)?;
        Ok(signature.as_bytes().to_vec())
    }

    /// Verify a signature.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), PostQuantumError> {
        let sig = Signature::from_bytes(signature)
            .map_err(|_| PostQuantumError::Verify)?;
        verify(&sig, message, &self.public_key)
            .map_err(|_| PostQuantumError::Verify)
    }
}

/// A Falcon key pair for compact signatures.
pub struct FalconKeyPair {
    public_key: PublicKey,
    secret_key: SecretKey,
}

impl FalconKeyPair {
    /// Generate a new Falcon‑1024 key pair.
    pub fn generate() -> Result<Self, PostQuantumError> {
        let (public_key, secret_key) = keypair().map_err(|_| PostQuantumError::KeyGen)?;
        Ok(Self { public_key, secret_key })
    }

    /// Get the public key bytes.
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.as_bytes().to_vec()
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, PostQuantumError> {
        let signature = sign(message, &self.secret_key)
            .map_err(|_| PostQuantumError::Sign)?;
        Ok(signature.as_bytes().to_vec())
    }

    /// Verify a signature.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), PostQuantumError> {
        let sig = Signature::from_bytes(signature)
            .map_err(|_| PostQuantumError::Verify)?;
        verify(&sig, message, &self.public_key)
            .map_err(|_| PostQuantumError::Verify)
    }
}

/// Hybrid encryption using Kyber KEM and ChaCha20‑Poly1305.
pub mod hybrid {
    use super::*;
    use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
    use chacha20poly1305::aead::{Aead, NewAead};
    use rand_core::OsRng;

    /// Encrypt a plaintext for a recipient's Kyber public key.
    pub fn encrypt(
        plaintext: &[u8],
        recipient_public_key: &[u8],
    ) -> Result<(Vec<u8>, Vec<u8>), PostQuantumError> {
        // Parse public key
        let pk = PublicKey::from_bytes(recipient_public_key)
            .map_err(|_| PostQuantumError::InvalidKeyLength)?;
        // Encapsulate a shared secret
        let (ciphertext_kem, shared_secret) = encapsulate(&pk)
            .map_err(|_| PostQuantumError::Encaps)?;
        // Derive symmetric key from shared secret (SHA‑256)
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(shared_secret.as_bytes());
        let symmetric_key: [u8; 32] = hasher.finalize().into();
        // Encrypt plaintext with ChaCha20‑Poly1305
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&symmetric_key));
        let nonce = generate_nonce();
        let ciphertext_aead = cipher.encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| PostQuantumError::Encaps)?;
        // Return KEM ciphertext + AEAD ciphertext + nonce
        let mut combined = Vec::new();
        combined.extend_from_slice(ciphertext_kem.as_bytes());
        combined.extend_from_slice(&nonce);
        combined.extend_from_slice(&ciphertext_aead);
        Ok((combined, shared_secret.as_bytes().to_vec()))
    }

    /// Decrypt a ciphertext using a Kyber secret key.
    pub fn decrypt(
        ciphertext: &[u8],
        secret_key: &[u8],
    ) -> Result<Vec<u8>, PostQuantumError> {
        // Ciphertext layout: [KEM ciphertext (1568 bytes)][nonce (12 bytes)][AEAD ciphertext]
        if ciphertext.len() < 1568 + 12 {
            return Err(PostQuantumError::InvalidCiphertextLength);
        }
        let (kem_ct_bytes, rest) = ciphertext.split_at(1568);
        let (nonce_bytes, aead_ct) = rest.split_at(12);
        // Decapsulate shared secret
        let sk = SecretKey::from_bytes(secret_key)
            .map_err(|_| PostQuantumError::InvalidKeyLength)?;
        let ct = Ciphertext::from_bytes(kem_ct_bytes)
            .map_err(|_| PostQuantumError::InvalidCiphertextLength)?;
        let shared_secret = decapsulate(&ct, &sk)
            .map_err(|_| PostQuantumError::Decaps)?;
        // Derive symmetric key
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(shared_secret.as_bytes());
        let symmetric_key: [u8; 32] = hasher.finalize().into();
        // Decrypt AEAD
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&symmetric_key));
        let plaintext = cipher.decrypt(Nonce::from_slice(nonce_bytes), aead_ct)
            .map_err(|_| PostQuantumError::Decaps)?;
        Ok(plaintext)
    }

    fn generate_nonce() -> [u8; 12] {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
}

/// A signed message using Dilithium.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DilithiumSignedMessage {
    /// The serialized message payload.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    /// The Dilithium signature.
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// The public key of the sender.
    #[serde(with = "serde_bytes")]
    pub public_key: Vec<u8>,
}

impl DilithiumSignedMessage {
    /// Create a new signed message.
    pub fn new(payload: Vec<u8>, key_pair: &DilithiumKeyPair) -> Result<Self, PostQuantumError> {
        let signature = key_pair.sign(&payload)?;
        let public_key = key_pair.public_key_bytes();
        Ok(Self {
            payload,
            signature,
            public_key,
        })
    }

    /// Verify the signature.
    pub fn verify(&self) -> Result<(), PostQuantumError> {
        let pk = PublicKey::from_bytes(&self.public_key)
            .map_err(|_| PostQuantumError::InvalidKeyLength)?;
        let sig = Signature::from_bytes(&self.signature)
            .map_err(|_| PostQuantumError::Verify)?;
        verify(&sig, &self.payload, &pk)
            .map_err(|_| PostQuantumError::Verify)
    }
}

/// Integration with the existing `SecurityManager`.
/// This struct can be used as a drop‑in replacement for classical cryptography.
pub struct PostQuantumSecurityManager {
    kyber_keypair: KyberKeyPair,
    dilithium_keypair: DilithiumKeyPair,
}

impl PostQuantumSecurityManager {
    /// Generate a new manager with fresh key pairs.
    pub fn generate() -> Result<Self, PostQuantumError> {
        let kyber_keypair = KyberKeyPair::generate()?;
        let dilithium_keypair = DilithiumKeyPair::generate()?;
        Ok(Self {
            kyber_keypair,
            dilithium_keypair,
        })
    }

    /// Get the Kyber public key bytes.
    pub fn kyber_public_key(&self) -> Vec<u8> {
        self.kyber_keypair.public_key_bytes()
    }

    /// Get the Dilithium public key bytes.
    pub fn dilithium_public_key(&self) -> Vec<u8> {
        self.dilithium_keypair.public_key_bytes()
    }

    /// Sign a payload with Dilithium.
    pub fn sign(&self, payload: Vec<u8>) -> Result<DilithiumSignedMessage, PostQuantumError> {
        DilithiumSignedMessage::new(payload, &self.dilithium_keypair)
    }

    /// Verify a Dilithium signed message.
    pub fn verify(&self, signed: &DilithiumSignedMessage) -> Result<(), PostQuantumError> {
        signed.verify()
    }

    /// Encrypt a plaintext for a recipient's Kyber public key.
    pub fn encrypt_for(
        &self,
        plaintext: &[u8],
        recipient_kyber_public: &[u8],
    ) -> Result<Vec<u8>, PostQuantumError> {
        hybrid::encrypt(plaintext, recipient_kyber_public).map(|(ciphertext, _)| ciphertext)
    }

    /// Decrypt a ciphertext that was encrypted for us.
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, PostQuantumError> {
        hybrid::decrypt(ciphertext, &self.kyber_keypair.secret_key.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kyber_keypair() {
        let kp = KyberKeyPair::generate().unwrap();
        let (ct, ss1) = kp.encapsulate().unwrap();
        let ss2 = kp.decapsulate(&ct).unwrap();
        assert_eq!(ss1, ss2);
    }

    #[test]
    fn test_dilithium_sign_verify() {
        let kp = DilithiumKeyPair::generate().unwrap();
        let message = b"Hello, quantum world!";
        let sig = kp.sign(message).unwrap();
        kp.verify(message, &sig).unwrap();
    }

    #[test]
    fn test_hybrid_encryption() {
        let sender_kp = KyberKeyPair::generate().unwrap();
        let recipient_kp = KyberKeyPair::generate().unwrap();
        let plaintext = b"Secret message";
        let (ciphertext, _) = hybrid::encrypt(plaintext, &recipient_kp.public_key_bytes()).unwrap();
        let decrypted = hybrid::decrypt(&ciphertext, &recipient_kp.secret_key.as_bytes()).unwrap();
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_security_manager() {
        let manager = PostQuantumSecurityManager::generate().unwrap();
        let message = b"Test payload";
        let signed = manager.sign(message.to_vec()).unwrap();
        manager.verify(&signed).unwrap();
    }
}