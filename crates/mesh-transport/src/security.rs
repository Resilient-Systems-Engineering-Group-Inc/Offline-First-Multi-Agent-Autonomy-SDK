//! Cryptographic security for mesh transport (signing, verification, encryption).

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand_core::OsRng;
use serde::{Serialize, Deserialize};
use serde_bytes::Bytes;
use thiserror::Error;
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use chacha20poly1305::aead::{Aead, NewAead};
use x25519_dalek::{PublicKey, StaticSecret, SharedSecret};
use std::convert::TryInto;

/// Errors that can occur during security operations.
#[derive(Error, Debug)]
pub enum SecurityError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Key generation error")]
    KeyGeneration,
    #[error("Encryption error")]
    Encryption,
    #[error("Decryption error")]
    Decryption,
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Invalid nonce length")]
    InvalidNonceLength,
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

// --- Encryption ---

/// Generate a random 256‑bit key for symmetric encryption.
pub fn generate_symmetric_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

/// Encrypt a message with ChaCha20‑Poly1305.
pub fn encrypt(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>, SecurityError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(nonce);
    cipher.encrypt(nonce, plaintext)
        .map_err(|_| SecurityError::Encryption)
}

/// Decrypt a message with ChaCha20‑Poly1305.
pub fn decrypt(key: &[u8; 32], nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>, SecurityError> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext)
        .map_err(|_| SecurityError::Decryption)
}

/// Generate a random nonce.
pub fn generate_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

// --- Diffie‑Hellman key exchange ---

/// A Diffie‑Hellman key pair (X25519).
pub struct DhKeyPair {
    secret: StaticSecret,
    public: PublicKey,
}

impl DhKeyPair {
    pub fn generate() -> Self {
        let secret = StaticSecret::new(OsRng);
        let public = PublicKey::from(&secret);
        Self { secret, public }
    }

    pub fn public_key(&self) -> PublicKey {
        self.public
    }

    /// Compute shared secret with another public key.
    pub fn diffie_hellman(&self, other_public: &PublicKey) -> SharedSecret {
        self.secret.diffie_hellman(other_public)
    }
}

/// An encrypted message that includes the nonce and optional sender's ephemeral public key.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EncryptedMessage {
    /// Nonce used for encryption.
    #[serde(with = "serde_bytes")]
    pub nonce: Vec<u8>,
    /// Ephemeral public key for key exchange (if using ECDH).
    #[serde(with = "serde_bytes")]
    pub ephemeral_public_key: Option<Vec<u8>>,
    /// The ciphertext.
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
}

impl EncryptedMessage {
    /// Encrypt a plaintext with a symmetric key.
    pub fn encrypt_with_key(plaintext: &[u8], key: &[u8; 32]) -> Result<Self, SecurityError> {
        let nonce = generate_nonce();
        let ciphertext = encrypt(key, &nonce, plaintext)?;
        Ok(Self {
            nonce: nonce.to_vec(),
            ephemeral_public_key: None,
            ciphertext,
        })
    }

    /// Decrypt with a symmetric key.
    pub fn decrypt_with_key(&self, key: &[u8; 32]) -> Result<Vec<u8>, SecurityError> {
        let nonce: [u8; 12] = self.nonce.as_slice().try_into()
            .map_err(|_| SecurityError::InvalidNonceLength)?;
        decrypt(key, &nonce, &self.ciphertext)
    }

    /// Encrypt using a Diffie‑Hellman key exchange (hybrid encryption).
    /// The sender uses their ephemeral key pair and the recipient's static public key.
    pub fn encrypt_hybrid(
        plaintext: &[u8],
        recipient_public: &PublicKey,
    ) -> Result<(Self, DhKeyPair), SecurityError> {
        let ephemeral = DhKeyPair::generate();
        let shared_secret = ephemeral.diffie_hellman(recipient_public);
        // Derive a symmetric key from the shared secret (simple hash for demonstration)
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(shared_secret.as_bytes());
        let symmetric_key: [u8; 32] = hasher.finalize().into();

        let nonce = generate_nonce();
        let ciphertext = encrypt(&symmetric_key, &nonce, plaintext)?;

        let message = Self {
            nonce: nonce.to_vec(),
            ephemeral_public_key: Some(ephemeral.public_key().as_bytes().to_vec()),
            ciphertext,
        };
        Ok((message, ephemeral))
    }

    /// Decrypt hybrid encryption using the recipient's static secret.
    pub fn decrypt_hybrid(
        &self,
        recipient_secret: &StaticSecret,
    ) -> Result<Vec<u8>, SecurityError> {
        let ephemeral_public = self.ephemeral_public_key.as_ref()
            .ok_or(SecurityError::Decryption)?;
        let ephemeral_public = PublicKey::from(ephemeral_public.as_slice().try_into()
            .map_err(|_| SecurityError::InvalidKeyLength)?);
        let shared_secret = recipient_secret.diffie_hellman(&ephemeral_public);
        let mut hasher = sha2::Sha256::new();
        hasher.update(shared_secret.as_bytes());
        let symmetric_key: [u8; 32] = hasher.finalize().into();

        let nonce: [u8; 12] = self.nonce.as_slice().try_into()
            .map_err(|_| SecurityError::InvalidNonceLength)?;
        decrypt(&symmetric_key, &nonce, &self.ciphertext)
    }
}

// --- Authentication ---

use common::types::AgentIdentity;

/// Authenticator verifies that a message is signed by a specific agent identity.
pub struct Authenticator;

impl Authenticator {
    /// Verify that a signed message is valid and matches the given agent identity.
    pub fn verify_signed_message(
        signed: &SignedMessage,
        identity: &AgentIdentity,
    ) -> Result<(), SecurityError> {
        // Check that the public key in the signed message matches the identity's public key.
        if signed.public_key != identity.public_key {
            return Err(SecurityError::InvalidSignature);
        }
        signed.verify()
    }

    /// Create a signed message that can be authenticated back to an identity.
    pub fn sign_message(
        payload: Vec<u8>,
        key_pair: &KeyPair,
        identity: &AgentIdentity,
    ) -> Result<SignedMessage, SecurityError> {
        // Ensure the key pair matches the identity (optional check)
        if key_pair.verifying_key().to_bytes() != identity.public_key {
            return Err(SecurityError::InvalidSignature);
        }
        Ok(SignedMessage::new(payload, key_pair))
    }
}

/// Access control list (ACL) entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlEntry {
    /// Agent ID allowed.
    pub agent_id: AgentId,
    /// Permissions: "read", "write", "admin", etc.
    pub permissions: Vec<String>,
    /// Resource pattern (e.g., "crdt/*").
    pub resource: String,
}

/// Simple ACL manager.
pub struct AccessControlList {
    entries: Vec<AccessControlEntry>,
}

impl AccessControlList {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, entry: AccessControlEntry) {
        self.entries.push(entry);
    }

    /// Check if an agent has permission for a given resource and action.
    pub fn check_permission(&self, agent_id: AgentId, resource: &str, action: &str) -> bool {
        self.entries.iter().any(|entry| {
            entry.agent_id == agent_id
                && (entry.resource == resource || entry.resource.ends_with("*"))
                && entry.permissions.contains(&action.to_string())
        })
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

    #[test]
    fn test_symmetric_encryption() {
        let key = generate_symmetric_key();
        let nonce = generate_nonce();
        let plaintext = b"secret message";
        let ciphertext = encrypt(&key, &nonce, plaintext).unwrap();
        let decrypted = decrypt(&key, &nonce, &ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypted_message() {
        let key = generate_symmetric_key();
        let plaintext = b"hello";
        let enc = EncryptedMessage::encrypt_with_key(plaintext, &key).unwrap();
        let decrypted = enc.decrypt_with_key(&key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_diffie_hellman() {
        let alice = DhKeyPair::generate();
        let bob = DhKeyPair::generate();
        let alice_shared = alice.diffie_hellman(&bob.public_key());
        let bob_shared = bob.diffie_hellman(&alice.public_key());
        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_hybrid_encryption() {
        let recipient = DhKeyPair::generate();
        let plaintext = b"hybrid secret";
        let (enc, sender_ephemeral) = EncryptedMessage::encrypt_hybrid(
            plaintext,
            &recipient.public_key(),
        ).unwrap();
        let decrypted = enc.decrypt_hybrid(&recipient.secret).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}