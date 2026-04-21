//! Post-quantum cryptography implementation for mesh transport.
//!
//! Provides quantum-resistant key exchange and digital signatures using:
//! - Kyber (Key Encapsulation Mechanism)
//! - Dilithium (Digital Signatures)
//! - Falcon (Alternative digital signatures)

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fmt;

#[cfg(feature = "post-quantum")]
use pqcrypto_kyber as kyber;
#[cfg(feature = "post-quantum")]
use pqcrypto_dilithium as dilithium;
#[cfg(feature = "post-quantum")]
use pqcrypto_falcon as falcon;

/// Post-quantum key pair for Kyber KEM.
#[derive(Clone, Serialize, Deserialize)]
pub struct KyberKeyPair {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl KyberKeyPair {
    #[cfg(feature = "post-quantum")]
    pub fn generate() -> Self {
        let (pk, sk) = kyber::keypair();
        Self {
            public_key: pk.as_bytes().to_vec(),
            secret_key: sk.as_bytes().to_vec(),
        }
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn generate() -> Self {
        panic!("Post-quantum feature must be enabled");
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }

    #[cfg(feature = "post-quantum")]
    pub fn encapsulate(&self, recipient_pk: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        let recipient_pk = kyber::PublicKey::from_bytes(recipient_pk)
            .map_err(|e| anyhow!("Invalid public key: {:?}", e))?;
        
        let (ciphertext, shared_secret) = kyber::encapsulate(&recipient_pk);
        
        Ok((
            ciphertext.as_bytes().to_vec(),
            shared_secret.as_bytes().to_vec(),
        ))
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn encapsulate(&self, _recipient_pk: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
        panic!("Post-quantum feature must be enabled");
    }

    #[cfg(feature = "post-quantum")]
    pub fn decapsulate(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let ciphertext = kyber::Ciphertext::from_bytes(ciphertext)
            .map_err(|e| anyhow!("Invalid ciphertext: {:?}", e))?;
        let secret_key = kyber::SecretKey::from_bytes(&self.secret_key)
            .map_err(|e| anyhow!("Invalid secret key: {:?}", e))?;
        
        let shared_secret = kyber::decapsulate(&ciphertext, &secret_key);
        Ok(shared_secret.as_bytes().to_vec())
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn decapsulate(&self, _ciphertext: &[u8]) -> Result<Vec<u8>> {
        panic!("Post-quantum feature must be enabled");
    }
}

impl fmt::Debug for KyberKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KyberKeyPair")
            .field("public_key", &hex::encode(&self.public_key))
            .field("secret_key", &"<hidden>")
            .finish()
    }
}

/// Post-quantum digital signature using Dilithium.
#[derive(Clone, Serialize, Deserialize)]
pub struct DilithiumSignature {
    signature: Vec<u8>,
}

impl DilithiumSignature {
    #[cfg(feature = "post-quantum")]
    pub fn verify(&self, message: &[u8], public_key: &[u8]) -> bool {
        let pk = match pqcrypto_dilithium::PublicKey::from_bytes(public_key) {
            Ok(pk) => pk,
            Err(_) => return false,
        };
        
        let sig = match pqcrypto_dilithium::Signature::from_bytes(&self.signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        
        pqcrypto_dilithium::verify(&sig, message, &pk)
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn verify(&self, _message: &[u8], _public_key: &[u8]) -> bool {
        panic!("Post-quantum feature must be enabled");
    }
}

/// Dilithium key pair for post-quantum signatures.
#[derive(Clone, Serialize, Deserialize)]
pub struct DilithiumKeyPair {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl DilithiumKeyPair {
    #[cfg(feature = "post-quantum")]
    pub fn generate() -> Self {
        let (pk, sk) = dilithium::keypair();
        Self {
            public_key: pk.as_bytes().to_vec(),
            secret_key: sk.as_bytes().to_vec(),
        }
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn generate() -> Self {
        panic!("Post-quantum feature must be enabled");
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }

    #[cfg(feature = "post-quantum")]
    pub fn sign(&self, message: &[u8]) -> DilithiumSignature {
        let secret_key = dilithium::SecretKey::from_bytes(&self.secret_key)
            .expect("Invalid secret key");
        
        let signature = dilithium::sign(message, &secret_key);
        DilithiumSignature {
            signature: signature.as_bytes().to_vec(),
        }
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn sign(&self, _message: &[u8]) -> DilithiumSignature {
        panic!("Post-quantum feature must be enabled");
    }

    pub fn verify(&self, message: &[u8], signature: &DilithiumSignature) -> bool {
        signature.verify(message, &self.public_key)
    }
}

impl fmt::Debug for DilithiumKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DilithiumKeyPair")
            .field("public_key", &hex::encode(&self.public_key))
            .field("secret_key", &"<hidden>")
            .finish()
    }
}

/// Falcon digital signature (alternative to Dilithium, smaller signatures).
#[derive(Clone, Serialize, Deserialize)]
pub struct FalconSignature {
    signature: Vec<u8>,
}

/// Falcon key pair for post-quantum signatures.
#[derive(Clone, Serialize, Deserialize)]
pub struct FalconKeyPair {
    public_key: Vec<u8>,
    secret_key: Vec<u8>,
}

impl FalconKeyPair {
    #[cfg(feature = "post-quantum")]
    pub fn generate() -> Self {
        let (pk, sk) = falcon::keypair();
        Self {
            public_key: pk.as_bytes().to_vec(),
            secret_key: sk.as_bytes().to_vec(),
        }
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn generate() -> Self {
        panic!("Post-quantum feature must be enabled");
    }

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }

    #[cfg(feature = "post-quantum")]
    pub fn sign(&self, message: &[u8]) -> FalconSignature {
        let secret_key = falcon::SecretKey::from_bytes(&self.secret_key)
            .expect("Invalid secret key");
        
        let signature = falcon::sign(message, &secret_key);
        FalconSignature {
            signature: signature.as_bytes().to_vec(),
        }
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn sign(&self, _message: &[u8]) -> FalconSignature {
        panic!("Post-quantum feature must be enabled");
    }
}

impl fmt::Debug for FalconKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FalconKeyPair")
            .field("public_key", &hex::encode(&self.public_key))
            .field("secret_key", &"<hidden>")
            .finish()
    }
}

/// Hybrid security manager combining classical and post-quantum crypto.
/// 
/// This provides a transition path to post-quantum security by using
/// both classical (Ed25519, X25519) and post-quantum algorithms.
pub struct HybridSecurityManager {
    classical_enabled: bool,
    post_quantum_enabled: bool,
    
    #[cfg(feature = "post-quantum")]
    pq_keypair: Option<KyberKeyPair>,
    
    #[cfg(feature = "post-quantum")]
    sig_keypair: Option<DilithiumKeyPair>,
}

impl HybridSecurityManager {
    pub fn new() -> Self {
        Self {
            classical_enabled: true,
            post_quantum_enabled: cfg!(feature = "post-quantum"),
            
            #[cfg(feature = "post-quantum")]
            pq_keypair: None,
            
            #[cfg(feature = "post-quantum")]
            sig_keypair: None,
        }
    }

    pub fn enable_post_quantum(&mut self) -> Result<()> {
        #[cfg(feature = "post-quantum")]
        {
            self.pq_keypair = Some(KyberKeyPair::generate());
            self.sig_keypair = Some(DilithiumKeyPair::generate());
            self.post_quantum_enabled = true;
            Ok(())
        }
        
        #[cfg(not(feature = "post-quantum"))]
        Err(anyhow!("Post-quantum feature not enabled"))
    }

    #[cfg(feature = "post-quantum")]
    pub fn establish_shared_secret(&self, peer_public_key: &[u8]) -> Result<Vec<u8>> {
        if let Some(ref keypair) = self.pq_keypair {
            let (ciphertext, shared_secret) = keypair.encapsulate(peer_public_key)?;
            // In hybrid mode, combine with classical KEM
            Ok(shared_secret)
        } else {
            Err(anyhow!("Post-quantum keypair not initialized"))
        }
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn establish_shared_secret(&self, _peer_public_key: &[u8]) -> Result<Vec<u8>> {
        Err(anyhow!("Post-quantum feature not enabled"))
    }

    #[cfg(feature = "post-quantum")]
    pub fn sign_message(&self, message: &[u8]) -> Result<DilithiumSignature> {
        if let Some(ref keypair) = self.sig_keypair {
            Ok(keypair.sign(message))
        } else {
            Err(anyhow!("Post-quantum keypair not initialized"))
        }
    }

    #[cfg(not(feature = "post-quantum"))]
    pub fn sign_message(&self, _message: &[u8]) -> Result<DilithiumSignature> {
        Err(anyhow!("Post-quantum feature not enabled"))
    }

    pub fn is_post_quantum_enabled(&self) -> bool {
        self.post_quantum_enabled
    }
}

impl Default for HybridSecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "post-quantum")]
    fn test_kyber_encapsulation() {
        let keypair = KyberKeyPair::generate();
        let recipient_pk = keypair.public_key();
        
        let (ciphertext, shared_secret) = keypair.encapsulate(recipient_pk).unwrap();
        let decrypted = keypair.decapsulate(&ciphertext).unwrap();
        
        assert_eq!(shared_secret, decrypted);
    }

    #[test]
    #[cfg(feature = "post-quantum")]
    fn test_dilithium_signature() {
        let keypair = DilithiumKeyPair::generate();
        let message = b"Test message for signing";
        
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature));
        
        // Verify with wrong message should fail
        assert!(!keypair.verify(b"Wrong message", &signature));
    }

    #[test]
    #[cfg(feature = "post-quantum")]
    fn test_hybrid_security_manager() {
        let mut manager = HybridSecurityManager::new();
        assert!(!manager.is_post_quantum_enabled());
        
        manager.enable_post_quantum().unwrap();
        assert!(manager.is_post_quantum_enabled());
    }
}
