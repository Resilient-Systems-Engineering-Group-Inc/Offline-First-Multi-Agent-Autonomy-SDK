//! Blockchain integration for decentralized consensus.
//!
//! Provides:
//! - Smart contract interactions
//! - Decentralized task assignment
//! - Consensus mechanisms
//! - Transaction management

pub mod consensus;
pub mod contracts;
pub mod transactions;

use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub use consensus::*;
pub use contracts::*;
pub use transactions::*;

/// Blockchain configuration.
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    pub rpc_url: String,
    pub chain_id: u64,
    pub contract_address: Option<Address>,
    pub private_key: Option<String>,
    pub confirmation_blocks: u64,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 1,
            contract_address: None,
            private_key: None,
            confirmation_blocks: 12,
        }
    }
}

/// Blockchain manager.
pub struct BlockchainManager {
    config: BlockchainConfig,
    client: RwLock<Option<Arc<Provider<Http>>>>,
    contract: RwLock<Option<Arc<SDKConsensusContract>>>,
}

impl BlockchainManager {
    /// Create new blockchain manager.
    pub fn new(config: BlockchainConfig) -> Self {
        Self {
            config,
            client: RwLock::new(None),
            contract: RwLock::new(None),
        }
    }

    /// Connect to blockchain.
    pub async fn connect(&self) -> Result<()> {
        let provider = Provider::<Http>::try_from(&self.config.rpc_url)?;
        let client = Arc::new(provider);

        *self.client.write().await = Some(client);
        info!("Connected to blockchain");

        // Load contract if address provided
        if let Some(addr) = self.config.contract_address {
            self.load_contract(addr).await?;
        }

        Ok(())
    }

    /// Load smart contract.
    pub async fn load_contract(&self, address: Address) -> Result<()> {
        let client = self.client.read().await.clone();
        
        if let Some(client) = client {
            let contract = Arc::new(SDKConsensusContract::new(address, client));
            *self.contract.write().await = Some(contract);
            
            info!("Loaded contract at {:?}", address);
        }

        Ok(())
    }

    /// Register agent on blockchain.
    pub async fn register_agent(&self, agent_id: &str, capabilities: Vec<String>) -> Result<TxHash> {
        let contract = self.contract.read().await.clone();
        
        if let Some(contract) = contract {
            let agent_bytes = hex::decode(agent_id)?;
            let caps_bytes: Vec<Vec<u8>> = capabilities.iter().map(|s| s.as_bytes().to_vec()).collect();
            
            let pending_tx = contract.register_agent(agent_bytes.into(), caps_bytes);
            let receipt = pending_tx.await?;
            
            info!("Agent registered: {:?}", receipt);
            Ok(receipt.transaction_hash)
        } else {
            Err(anyhow::anyhow!("Contract not loaded"))
        }
    }

    /// Submit task to blockchain.
    pub async fn submit_task(&self, task_id: &str, requirements: serde_json::Value) -> Result<TxHash> {
        let contract = self.contract.read().await.clone();
        
        if let Some(contract) = contract {
            let task_bytes = hex::decode(task_id)?;
            let req_bytes = serde_json::to_vec(&requirements)?;
            
            let pending_tx = contract.submit_task(task_bytes.into(), req_bytes.into());
            let receipt = pending_tx.await?;
            
            info!("Task submitted: {:?}", receipt);
            Ok(receipt.transaction_hash)
        } else {
            Err(anyhow::anyhow!("Contract not loaded"))
        }
    }

    /// Get consensus for task.
    pub async fn get_consensus(&self, task_id: &str) -> Result<Vec<String>> {
        let contract = self.contract.read().await.clone();
        
        if let Some(contract) = contract {
            let task_bytes = hex::decode(task_id)?;
            let agents = contract.get_consensus(task_bytes.into()).await?;
            
            let agent_ids: Vec<String> = agents.iter()
                .map(|bytes| hex::encode(bytes.0.as_slice()))
                .collect();
            
            Ok(agent_ids)
        } else {
            Err(anyhow::anyhow!("Contract not loaded"))
        }
    }

    /// Finalize task assignment.
    pub async fn finalize_assignment(&self, task_id: &str, agent_id: &str) -> Result<TxHash> {
        let contract = self.contract.read().await.clone();
        
        if let Some(contract) = contract {
            let task_bytes = hex::decode(task_id)?;
            let agent_bytes = hex::decode(agent_id)?;
            
            let pending_tx = contract.finalize_assignment(task_bytes.into(), agent_bytes.into());
            let receipt = pending_tx.await?;
            
            info!("Assignment finalized: {:?}", receipt);
            Ok(receipt.transaction_hash)
        } else {
            Err(anyhow::anyhow!("Contract not loaded"))
        }
    }

    /// Get blockchain statistics.
    pub async fn get_stats(&self) -> Result<BlockchainStats> {
        let client = self.client.read().await.clone();
        
        if let Some(client) = client {
            let block_number = client.get_block_number().await?;
            
            Ok(BlockchainStats {
                chain_id: self.config.chain_id,
                current_block: block_number.as_u64(),
                contract_address: self.config.contract_address,
                confirmation_blocks: self.config.confirmation_blocks,
            })
        } else {
            Err(anyhow::anyhow!("Not connected"))
        }
    }
}

/// Blockchain statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlockchainStats {
    pub chain_id: u64,
    pub current_block: u64,
    pub contract_address: Option<Address>,
    pub confirmation_blocks: u64,
}

/// Transaction hash type alias.
pub type TxHash = H256;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blockchain_manager() {
        let config = BlockchainConfig {
            rpc_url: "http://localhost:8545".to_string(),
            chain_id: 31337, // Local testnet
            contract_address: None,
            private_key: None,
            confirmation_blocks: 1,
        };

        let manager = BlockchainManager::new(config);
        
        // Would connect to real blockchain in production
        // manager.connect().await.unwrap();
        
        assert!(true); // Placeholder
    }
}
