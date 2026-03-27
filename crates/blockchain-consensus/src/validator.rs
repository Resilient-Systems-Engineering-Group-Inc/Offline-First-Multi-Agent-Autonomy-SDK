//! Validator node that participates in consensus.

use crate::block::{Block, Transaction};
use crate::chain::Blockchain;
use crate::pos::ProofOfStake;
use crate::error::{Error, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;

/// A validator node that can propose and validate blocks.
pub struct Validator {
    /// Signing key (private).
    signing_key: SigningKey,
    /// Verifying key (public).
    verifying_key: VerifyingKey,
    /// Local copy of the blockchain.
    blockchain: Blockchain,
    /// Proof‑of‑stake engine.
    pos: ProofOfStake,
}

impl Validator {
    /// Create a new validator with a generated keypair.
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let blockchain = Blockchain::new().expect("Failed to create genesis block");
        let pos = ProofOfStake::new(100); // min stake 100
        Self {
            signing_key,
            verifying_key,
            blockchain,
            pos,
        }
    }

    /// Propose a new block containing pending transactions.
    pub fn propose_block(&mut self) -> Result<Block> {
        // Check if we are eligible to propose
        if !self.pos.can_propose(self.verifying_key.as_bytes()) {
            return Err(Error::Consensus("Not enough stake to propose".to_string()));
        }
        let last_block = self.blockchain.last_block();
        let transactions = self.blockchain.pending_transactions.clone();
        let block = Block::new(
            last_block.index + 1,
            last_block.hash.clone(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            transactions,
            0, // nonce (not used in PoS)
            self.verifying_key.as_bytes().to_vec(),
        );
        Ok(block)
    }

    /// Sign a block (as a validator).
    pub fn sign_block(&self, block: &mut Block) {
        block.sign(&self.signing_key);
    }

    /// Validate a received block and add to chain if valid.
    pub fn receive_block(&mut self, block: Block) -> Result<()> {
        // Verify signature
        if !block.verify_signature() {
            return Err(Error::InvalidBlock("Invalid signature".to_string()));
        }
        // Verify validator has enough stake
        if !self.pos.can_propose(&block.validator) {
            return Err(Error::Consensus("Validator insufficient stake".to_string()));
        }
        // Add to blockchain
        self.blockchain.add_block(block)
    }

    /// Add a transaction to the mempool.
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        self.blockchain.add_transaction(transaction)
    }

    /// Get the public key of this validator.
    pub fn public_key(&self) -> &[u8] {
        self.verifying_key.as_bytes()
    }

    /// Get the blockchain.
    pub fn blockchain(&self) -> &Blockchain {
        &self.blockchain
    }

    /// Get a mutable reference to the proof‑of‑stake engine.
    pub fn pos_mut(&mut self) -> &mut ProofOfStake {
        &mut self.pos
    }
}