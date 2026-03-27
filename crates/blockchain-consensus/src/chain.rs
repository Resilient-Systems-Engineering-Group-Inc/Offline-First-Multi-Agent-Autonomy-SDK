//! Blockchain structure and validation.

use crate::block::{Block, Transaction};
use crate::error::{Error, Result};
use std::collections::HashMap;

/// A simple blockchain.
pub struct Blockchain {
    /// The chain of blocks.
    blocks: Vec<Block>,
    /// Pending transactions (mempool).
    pending_transactions: Vec<Transaction>,
    /// Map from public key to stake amount.
    stakes: HashMap<Vec<u8>, u64>,
}

impl Blockchain {
    /// Create a new blockchain with a genesis block.
    pub fn new() -> Result<Self> {
        let genesis = Block::new(
            0,
            vec![0; 32], // zero previous hash
            0,
            Vec::new(),
            0,
            vec![0; 32], // zero validator
        );
        let mut chain = Self {
            blocks: vec![genesis],
            pending_transactions: Vec::new(),
            stakes: HashMap::new(),
        };
        // Initialize with some default stakes (for demo)
        chain.stakes.insert(vec![0; 32], 1000);
        Ok(chain)
    }

    /// Add a new block to the chain after validation.
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        // Validate block
        self.validate_block(&block)?;
        self.blocks.push(block);
        // Remove transactions from pending
        // (simplified: we just clear all pending)
        self.pending_transactions.clear();
        Ok(())
    }

    /// Validate a block (consistency, signatures, etc.)
    fn validate_block(&self, block: &Block) -> Result<()> {
        // Check previous hash matches last block
        let last_block = self.blocks.last().ok_or_else(|| Error::InvalidBlock("No genesis".to_string()))?;
        if block.previous_hash != last_block.hash {
            return Err(Error::InvalidBlock("Previous hash mismatch".to_string()));
        }
        // Check index
        if block.index != last_block.index + 1 {
            return Err(Error::InvalidBlock("Index out of order".to_string()));
        }
        // Verify block signature
        if !block.verify_signature() {
            return Err(Error::InvalidBlock("Invalid signature".to_string()));
        }
        // Verify each transaction
        for tx in &block.transactions {
            if !tx.verify() {
                return Err(Error::InvalidTransaction("Transaction verification failed".to_string()));
            }
        }
        // Additional PoS validation would go here
        Ok(())
    }

    /// Add a transaction to the mempool.
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        if !transaction.verify() {
            return Err(Error::InvalidTransaction("Invalid signature".to_string()));
        }
        self.pending_transactions.push(transaction);
        Ok(())
    }

    /// Get the last block.
    pub fn last_block(&self) -> &Block {
        self.blocks.last().unwrap()
    }

    /// Get the length of the chain.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Check if the chain is empty (only genesis).
    pub fn is_empty(&self) -> bool {
        self.blocks.len() <= 1
    }

    /// Get stake for a validator.
    pub fn stake(&self, validator: &[u8]) -> u64 {
        self.stakes.get(validator).cloned().unwrap_or(0)
    }

    /// Set stake for a validator.
    pub fn set_stake(&mut self, validator: Vec<u8>, amount: u64) {
        self.stakes.insert(validator, amount);
    }
}