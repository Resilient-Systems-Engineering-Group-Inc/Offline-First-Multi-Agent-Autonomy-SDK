//! Block and transaction definitions.

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use ed25519_dalek::{Signature, Signer, Verifier, SigningKey, VerifyingKey};

/// A transaction that can be included in a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique transaction ID.
    pub id: u64,
    /// Sender's public key (as bytes).
    pub sender: Vec<u8>,
    /// Recipient's public key (as bytes).
    pub recipient: Vec<u8>,
    /// Transaction payload (JSON).
    pub payload: serde_json::Value,
    /// Digital signature.
    pub signature: Vec<u8>,
}

impl Transaction {
    /// Sign the transaction with a signing key.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let data = self.signing_data();
        self.signature = signing_key.sign(&data).to_bytes().to_vec();
    }

    /// Verify the transaction's signature.
    pub fn verify(&self) -> bool {
        let Ok(verifying_key) = VerifyingKey::from_bytes(&self.sender[..32]) else {
            return false;
        };
        let Ok(signature) = Signature::from_bytes(&self.signature[..64]) else {
            return false;
        };
        verifying_key.verify(&self.signing_data(), &signature).is_ok()
    }

    /// Data that is signed (excluding signature).
    fn signing_data(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.id.to_le_bytes());
        hasher.update(&self.sender);
        hasher.update(&self.recipient);
        hasher.update(serde_json::to_vec(&self.payload).unwrap());
        hasher.finalize().to_vec()
    }
}

/// A block in the blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Block index (height).
    pub index: u64,
    /// Previous block hash.
    pub previous_hash: Vec<u8>,
    /// Timestamp (Unix seconds).
    pub timestamp: u64,
    /// List of transactions.
    pub transactions: Vec<Transaction>,
    /// Nonce for proof‑of‑stake (or proof‑of‑work).
    pub nonce: u64,
    /// Hash of this block (calculated after construction).
    pub hash: Vec<u8>,
    /// Validator's public key.
    pub validator: Vec<u8>,
    /// Signature of the block by the validator.
    pub signature: Vec<u8>,
}

impl Block {
    /// Create a new block (without hash and signature).
    pub fn new(
        index: u64,
        previous_hash: Vec<u8>,
        timestamp: u64,
        transactions: Vec<Transaction>,
        nonce: u64,
        validator: Vec<u8>,
    ) -> Self {
        let mut block = Self {
            index,
            previous_hash,
            timestamp,
            transactions,
            nonce,
            hash: Vec::new(),
            validator,
            signature: Vec::new(),
        };
        block.hash = block.compute_hash();
        block
    }

    /// Compute the hash of the block (excluding hash and signature fields).
    pub fn compute_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.index.to_le_bytes());
        hasher.update(&self.previous_hash);
        hasher.update(&self.timestamp.to_le_bytes());
        for tx in &self.transactions {
            hasher.update(&tx.id.to_le_bytes());
        }
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.validator);
        hasher.finalize().to_vec()
    }

    /// Sign the block with a validator's key.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let data = self.signing_data();
        self.signature = signing_key.sign(&data).to_bytes().to_vec();
    }

    /// Verify the block's signature.
    pub fn verify_signature(&self) -> bool {
        let Ok(verifying_key) = VerifyingKey::from_bytes(&self.validator[..32]) else {
            return false;
        };
        let Ok(signature) = Signature::from_bytes(&self.signature[..64]) else {
            return false;
        };
        verifying_key.verify(&self.signing_data(), &signature).is_ok()
    }

    /// Data that is signed (excluding signature).
    fn signing_data(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.index.to_le_bytes());
        hasher.update(&self.previous_hash);
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.validator);
        hasher.update(&self.hash);
        hasher.finalize().to_vec()
    }
}