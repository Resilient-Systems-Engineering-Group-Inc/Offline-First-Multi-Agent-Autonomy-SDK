//! Auction mechanisms for resource allocation in multi-agent systems.
//!
//! This crate provides various auction mechanisms for distributing tasks and resources
//! among agents in a decentralized manner.
//!
//! # Supported Auction Types
//! - English auction (ascending price, open cry)
//! - Dutch auction (descending price)
//! - First-price sealed-bid
//! - Vickrey auction (second-price sealed-bid)
//! - Combinatorial auctions
//! - Double auctions
//!
//! # Example
//! ```
//! use auction_mechanism::{AuctionEngine, AuctionItem, AuctionConfig, AuctionType};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = AuctionEngine::new();
//!
//!     // Create an auction
//!     let item = AuctionItem::new("task-1", "Sample task", "task");
//!     let config = AuctionConfig {
//!         auction_type: AuctionType::English,
//!         duration_seconds: Some(300),
//!         ..Default::default()
//!     };
//!
//!     engine.create_auction("auction-1", item, config).await?;
//!     engine.start_auction("auction-1").await?;
//!
//!     // Place bids...
//!     // engine.place_bid("auction-1", 1, 100.0).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod engine;
pub mod error;
pub mod types;
pub mod resource_auction;

// Re-export commonly used types
pub use engine::AuctionEngine;
pub use error::{AuctionError, Result};
pub use types::{
    AuctionConfig, AuctionId, AuctionItem, AuctionResult, AuctionState, AuctionType, Bid,
    BidderId, BidderInfo,
};
pub use resource_auction::{
    ResourceType, ResourceUnit, ResourceBundle, BidderConstraints,
    CombinatorialAuction, CombinatorialAuctionResult,
    DoubleAuction, DoubleAuctionResult, Trade,
    utils,
};

/// Current version of the auction mechanism crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the auction system.
pub fn init() {
    // Any initialization logic would go here
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn test_basic_auction_flow() {
        let engine = AuctionEngine::new();
        
        // Create auction
        let item = AuctionItem::new("task-1", "Test task", "task");
        let config = AuctionConfig::default();
        
        engine.create_auction("test-auction", item, config).await.unwrap();
        
        // Start auction
        engine.start_auction("test-auction").await.unwrap();
        
        // Get auctions
        let auctions = engine.get_all_auctions().await;
        assert_eq!(auctions.len(), 1);
        assert_eq!(auctions[0].id(), "test-auction");
    }
}