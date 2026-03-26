//! Cryptographic security for mesh transport (signing, verification, encryption).

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand_core::OsRng;
use serde::{Serialize, Deserialize};
use serde_bytes::Bytes;
use thiserror::Error;

/// Errors that can occur during security operations.
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Key generation error")]
    KeyGeneration,
}

/// A key pair for signing messages.
pub struct KeyPair {
    signing_key: SigningKey,
}

impl KeyPair {
    /// Generate a new random key pair.
    pub fn generate() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self { signing_key }
    }

    /// Get the verifying (public) key.
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing_key.sign(message)
    }

    /// Sign a serializable message.
    pub fn sign_serializable<T: Serialize>(&self, message: &T) -> Result<Signature, SecurityError> {
        let bytes = bincode::serialize(message)?;
        Ok(self.sign(&bytes))
    }
}

/// A signed message that can be sent over the network.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedMessage {
    /// The serialized message payload.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    /// The signature of the payload.
    pub signature: Vec<u8>,
    /// The public key of the sender (optional, can be derived from context).
    #[serde(with = "serde_bytes")]
    pub public_key: Vec<u8>,
}

impl SignedMessage {
    /// Create a new signed message.
    pub fn new(payload: Vec<u8>, key_pair: &KeyPair) -> Self {
        let signature = key_pair.sign(&payload).to_bytes().to_vec();
        let public_key = key_pair.verifying_key().to_bytes().to_vec();
        Self {
            payload,
            signature,
            public_key,
        }
    }

    /// Verify the signature of this message.
    pub fn verify(&self) -> Result<(), SecurityError> {
        let verifying_key = VerifyingKey::from_bytes(
            self.public_key.as_slice().try_into()
                .map_err(|_| SecurityError::InvalidSignature)?
        ).map_err(|_| SecurityError::InvalidSignature)?;
        let signature = Signature::from_bytes(
            self.signature.as_slice().try_into()
                .map_err(|_| SecurityError::InvalidSignature)?
        ).map_err(|_| SecurityError::InvalidSignature)?;
        verifying_key.verify(&self.payload, &signature)
            .map_err(|_| SecurityError::InvalidSignature)
    }

    /// Deserialize the payload into a typed value.
    pub fn deserialize_payload<T: for<'de> Deserialize<'de>>(&self) -> Result<T, SecurityError> {
        bincode::deserialize(&self.payload).map_err(Into::into)
    }
    /// Serialize the signed message to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, SecurityError> {
        bincode::serialize(self).map_err(Into::into)
    }

    /// Deserialize bytes into a signed message.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SecurityError> {
        bincode::deserialize(bytes).map_err(Into::into)
    }
}

/// Helper to sign and verify messages using a given key pair.
pub struct SecurityManager {
    key_pair: KeyPair,
}

impl SecurityManager {
    pub fn new(key_pair: KeyPair) -> Self {
        Self { key_pair }
    }

    pub fn generate() -> Self {
        Self::new(KeyPair::generate())
    }

    pub fn key_pair(&self) -> &KeyPair {
        &self.key_pair
    }

    pub fn sign(&self, payload: Vec<u8>) -> SignedMessage {
        SignedMessage::new(payload, &self.key_pair)
    }

    pub fn verify(&self, signed: &SignedMessage) -> Result<(), SecurityError> {
        signed.verify()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_pair_sign_verify() {
        let key_pair = KeyPair::generate();
        let message = b"Hello, world!";
        let signature = key_pair.sign(message);
        let verifying_key = key_pair.verifying_key();
        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_signed_message() {
        let key_pair = KeyPair::generate();
        let payload = vec![1, 2, 3, 4];
        let signed = SignedMessage::new(payload.clone(), &key_pair);
        assert!(signed.verify().is_ok());
        assert_eq!(signed.payload, payload);
    }

    #[test]
    fn test_security_manager() {
        let manager = SecurityManager::generate();
        let payload = vec![5, 6, 7];
        let signed = manager.sign(payload);
        assert!(manager.verify(&signed).is_ok());
    }
}