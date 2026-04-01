//! Auction engine with different auction types.

use crate::error::{AuctionError, Result};
use crate::types::{
    AuctionConfig, AuctionId, AuctionItem, AuctionResult, AuctionState, AuctionType, Bid,
    BidderId,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Auction engine that manages multiple auctions.
pub struct AuctionEngine {
    auctions: RwLock<HashMap<AuctionId, Arc<Auction>>>,
    bidders: RwLock<HashMap<BidderId, crate::types::BidderInfo>>,
}

impl AuctionEngine {
    /// Create a new auction engine.
    pub fn new() -> Self {
        Self {
            auctions: RwLock::new(HashMap::new()),
            bidders: RwLock::new(HashMap::new()),
        }
    }

    /// Register a bidder.
    pub async fn register_bidder(&self, bidder_info: crate::types::BidderInfo) -> Result<()> {
        let mut bidders = self.bidders.write().await;
        let bidder_id = bidder_info.id;
        
        if bidders.contains_key(&bidder_id) {
            return Err(AuctionError::Internal(format!(
                "Bidder {} already registered",
                bidder_id
            )));
        }
        
        bidders.insert(bidder_id, bidder_info);
        info!("Registered bidder {}", bidder_id);
        Ok(())
    }

    /// Unregister a bidder.
    pub async fn unregister_bidder(&self, bidder_id: BidderId) -> Result<()> {
        let mut bidders = self.bidders.write().await;
        
        if bidders.remove(&bidder_id).is_none() {
            return Err(AuctionError::BidderNotRegistered(bidder_id));
        }
        
        info!("Unregistered bidder {}", bidder_id);
        Ok(())
    }

    /// Create a new auction.
    pub async fn create_auction(
        &self,
        auction_id: &str,
        item: AuctionItem,
        config: AuctionConfig,
    ) -> Result<()> {
        let mut auctions = self.auctions.write().await;
        
        if auctions.contains_key(auction_id) {
            return Err(AuctionError::AuctionAlreadyExists(auction_id.to_string()));
        }
        
        let auction = Auction::new(auction_id, item, config)?;
        auctions.insert(auction_id.to_string(), Arc::new(auction));
        
        info!("Created auction {}", auction_id);
        Ok(())
    }

    /// Start an auction.
    pub async fn start_auction(&self, auction_id: &str) -> Result<()> {
        let auction = self.get_auction(auction_id).await?;
        auction.start().await?;
        info!("Started auction {}", auction_id);
        Ok(())
    }

    /// Place a bid in an auction.
    pub async fn place_bid(&self, auction_id: &str, bidder_id: BidderId, amount: f64) -> Result<()> {
        let auction = self.get_auction(auction_id).await?;
        auction.place_bid(bidder_id, amount).await?;
        debug!("Bid placed in auction {} by bidder {}: {}", auction_id, bidder_id, amount);
        Ok(())
    }

    /// Close an auction (stop accepting bids).
    pub async fn close_auction(&self, auction_id: &str) -> Result<()> {
        let auction = self.get_auction(auction_id).await?;
        auction.close().await?;
        info!("Closed auction {}", auction_id);
        Ok(())
    }

    /// Evaluate an auction and determine the winner.
    pub async fn evaluate_auction(&self, auction_id: &str) -> Result<AuctionResult> {
        let auction = self.get_auction(auction_id).await?;
        let result = auction.evaluate().await?;
        info!("Evaluated auction {}: winner={:?}", auction_id, result.winner);
        Ok(result)
    }

    /// Get an auction by ID.
    async fn get_auction(&self, auction_id: &str) -> Result<Arc<Auction>> {
        let auctions = self.auctions.read().await;
        auctions
            .get(auction_id)
            .cloned()
            .ok_or_else(|| AuctionError::AuctionNotFound(auction_id.to_string()))
    }

    /// Get all auctions.
    pub async fn get_all_auctions(&self) -> Vec<Arc<Auction>> {
        let auctions = self.auctions.read().await;
        auctions.values().cloned().collect()
    }

    /// Get auctions by state.
    pub async fn get_auctions_by_state(&self, state: AuctionState) -> Vec<Arc<Auction>> {
        let auctions = self.auctions.read().await;
        auctions
            .values()
            .filter(|auction| auction.get_state().await == state)
            .cloned()
            .collect()
    }

    /// Cancel an auction.
    pub async fn cancel_auction(&self, auction_id: &str, reason: &str) -> Result<()> {
        let auction = self.get_auction(auction_id).await?;
        auction.cancel(reason).await?;
        info!("Cancelled auction {}: {}", auction_id, reason);
        Ok(())
    }

    /// Remove an auction.
    pub async fn remove_auction(&self, auction_id: &str) -> Result<()> {
        let mut auctions = self.auctions.write().await;
        
        if auctions.remove(auction_id).is_none() {
            return Err(AuctionError::AuctionNotFound(auction_id.to_string()));
        }
        
        info!("Removed auction {}", auction_id);
        Ok(())
    }
}

impl Default for AuctionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual auction instance.
pub struct Auction {
    id: String,
    item: AuctionItem,
    config: AuctionConfig,
    state: RwLock<AuctionState>,
    bids: RwLock<Vec<Bid>>,
    bidders: RwLock<HashSet<BidderId>>,
    created_at: std::time::SystemTime,
    started_at: RwLock<Option<std::time::SystemTime>>,
    closed_at: RwLock<Option<std::time::SystemTime>>,
}

impl Auction {
    /// Create a new auction.
    pub fn new(id: &str, item: AuctionItem, config: AuctionConfig) -> Result<Self> {
        Ok(Self {
            id: id.to_string(),
            item,
            config,
            state: RwLock::new(AuctionState::Creating),
            bids: RwLock::new(Vec::new()),
            bidders: RwLock::new(HashSet::new()),
            created_at: std::time::SystemTime::now(),
            started_at: RwLock::new(None),
            closed_at: RwLock::new(None),
        })
    }

    /// Get auction ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get auction state.
    pub async fn get_state(&self) -> AuctionState {
        *self.state.read().await
    }

    /// Get auction item.
    pub fn item(&self) -> &AuctionItem {
        &self.item
    }

    /// Get auction configuration.
    pub fn config(&self) -> &AuctionConfig {
        &self.config
    }

    /// Get all bids.
    pub async fn get_bids(&self) -> Vec<Bid> {
        self.bids.read().await.clone()
    }

    /// Start the auction.
    pub async fn start(&self) -> Result<()> {
        let mut state = self.state.write().await;
        
        if *state != AuctionState::Creating {
            return Err(AuctionError::InvalidAuctionState(
                self.id.clone(),
                AuctionState::Creating.to_string(),
                state.to_string(),
            ));
        }
        
        *state = AuctionState::Open;
        *self.started_at.write().await = Some(std::time::SystemTime::now());
        Ok(())
    }

    /// Place a bid in the auction.
    pub async fn place_bid(&self, bidder_id: BidderId, amount: f64) -> Result<()> {
        // Check auction state
        let state = self.state.read().await;
        if *state != AuctionState::Open {
            return Err(AuctionError::InvalidAuctionState(
                self.id.clone(),
                AuctionState::Open.to_string(),
                state.to_string(),
            ));
        }
        
        // Check if bidder is allowed
        if !self.config.allowed_bidders.is_empty()
            && !self.config.allowed_bidders.contains(&bidder_id)
        {
            return Err(AuctionError::BidderNotAllowed(
                bidder_id,
                "Bidder not in allowed list".to_string(),
            ));
        }
        
        // Check minimum bid increment
        if let Some(min_increment) = self.config.min_bid_increment {
            if let Some(highest_bid) = self.get_highest_bid().await {
                if amount < highest_bid + min_increment {
                    return Err(AuctionError::InvalidBid(format!(
                        "Bid must be at least {} higher than current highest bid {}",
                        min_increment, highest_bid
                    )));
                }
            }
        }
        
        // Check starting price
        if let Some(starting_price) = self.config.starting_price {
            if amount < starting_price {
                return Err(AuctionError::InvalidBid(format!(
                    "Bid must be at least starting price {}",
                    starting_price
                )));
            }
        }
        
        // Check reserve price
        if let Some(reserve_price) = self.item.reserve_price {
            if amount < reserve_price {
                return Err(AuctionError::ReservePriceNotMet(reserve_price));
            }
        }
        
        // Create and store the bid
        let bid = Bid::new(bidder_id, &self.id, amount);
        
        let mut bids = self.bids.write().await;
        bids.push(bid);
        
        let mut bidders = self.bidders.write().await;
        bidders.insert(bidder_id);
        
        Ok(())
    }

    /// Close the auction (stop accepting bids).
    pub async fn close(&self) -> Result<()> {
        let mut state = self.state.write().await;
        
        if *state != AuctionState::Open {
            return Err(AuctionError::InvalidAuctionState(
                self.id.clone(),
                AuctionState::Open.to_string(),
                state.to_string(),
            ));
        }
        
        *state = AuctionState::Closed;
        *self.closed_at.write().await = Some(std::time::SystemTime::now());
        Ok(())
    }

    /// Evaluate the auction and determine the winner.
    pub async fn evaluate(&self) -> Result<AuctionResult> {
        // Check auction state
        let mut state = self.state.write().await;
        if *state != AuctionState::Closed && *state != AuctionState::Open {
            return Err(AuctionError::InvalidAuctionState(
                self.id.clone(),
                format!("{} or {}", AuctionState::Closed, AuctionState::Open),
                state.to_string(),
            ));
        }
        
        *state = AuctionState::Evaluating;
        
        // Get all bids
        let bids = self.bids.read().await.clone();
        
        if bids.is_empty() {
            *state = AuctionState::Failed;
            return Ok(AuctionResult::failure(&self.id, "No bids received", bids));
        }
        
        // Find the highest bid
        let highest_bid = bids.iter().max_by(|a, b| {
            a.amount
                .partial_cmp(&b.amount)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let highest_bid = match highest_bid {
            Some(bid) => bid,
            None => {
                *state = AuctionState::Failed;
                return Ok(AuctionResult::failure(&self.id, "No valid bids", bids));
            }
        };
        
        // Check reserve price
        if let Some(reserve_price) = self.item.reserve_price {
            if highest_bid.amount < reserve_price {
                *state = AuctionState::Failed;
                return Ok(AuctionResult::failure(
                    &self.id,
                    &format!("Reserve price {} not met", reserve_price),
                    bids,
                ));
            }
        }
        
        // Find second highest bid for Vickrey auctions
        let second_highest_bid = if self.config.auction_type == AuctionType::Vickrey {
            let mut sorted_bids: Vec<&Bid> = bids.iter().collect();
            sorted_bids.sort_by(|a, b| {
                b.amount
                    .partial_cmp(&a.amount)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            
            if sorted_bids.len() >= 2 {
                Some(sorted_bids[1].amount)
            } else {
                Some(0.0) // Only one bidder pays the reserve price
            }
        } else {
            None
        };
        
        // Determine winning bid amount
        let winning_bid_amount = match self.config.auction_type {
            AuctionType::Vickrey => second_highest_bid.unwrap_or(0.0),
            _ => highest_bid.amount,
        };
        
        // Create result
        let result = AuctionResult {
            auction_id: self.id.clone(),
            winner: Some(highest_bid.bidder_id),
            winning_bid: Some(winning_bid_amount),
            second_highest_bid,
            all_bids: bids,
            timestamp: std::time::SystemTime::now(),
            successful: true,
            failure_reason: None,
        };
        
        *state = AuctionState::Completed;
        Ok(result)
    }

    /// Cancel the auction.
    pub async fn cancel(&self, reason: &str) -> Result<()> {
        let mut state = self.state.write().await;
        *state = AuctionState::Cancelled;
        
        // Store cancellation reason somewhere if needed
        debug!("Auction {} cancelled: {}", self.id, reason);
        Ok(())
    }

    /// Get the highest bid amount.
    async fn get_highest_bid(&self) -> Option<f64> {
        let bids = self.bids.read().await;
        bids.iter().map(|bid| bid.amount).max_by(|a, b| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Get the number of bidders.
    pub async fn bidder_count(&self) -> usize {
        self.bidders.read().await.len()
    }

    /// Get the number of bids.
    pub async fn bid_count(&self) -> usize {
        self.bids.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auction_creation() {
        let item = AuctionItem::new("task-1", "Sample task", "task");
        let config = AuctionConfig::default();
        
        let auction = Auction::new("test-auction", item, config).unwrap();
        assert_eq!(auction.id(), "test-auction");
        assert_eq!(auction.get_state().await, AuctionState::Creating);
    }

    #[tokio::test]
    async fn test_auction_start_close() {
        let item = AuctionItem::new("task-1", "Sample task", "task");
        let config = AuctionConfig::default();
        
        let auction = Auction::new("test-auction", item, config).unwrap();
        
        auction.start().await.unwrap();
        assert_eq!(auction.get_state().await, AuctionState::Open);
        
        auction.close().await.unwrap();
        assert_eq!(auction.get_state().await, AuctionState::Closed);
    }

    #[tokio::test]
    async fn test_place_bid() {
        let item = AuctionItem::new("task-1", "Sample task", "task");
        let config = AuctionConfig::default();
        
        let auction = Auction::new("test-auction", item, config).unwrap();
        auction.start().await.unwrap();
        
        auction.place_bid(1, 100.0).await.unwrap();
        assert_eq!(auction.bid_count().await, 1);
        assert_eq!(auction.bidder_count().await, 1);
    }

    #[tokio::test]
    async fn test_auction_engine() {
        let engine = AuctionEngine::new();
        
        // Register a bidder
        let bidder_info = crate::types::BidderInfo::new(1, "Test Bidder");
        engine.register_bidder(bidder_info).await.unwrap();
        
        // Create an auction
        let item = AuctionItem::new("task-1", "Sample task", "task");
        let config = AuctionConfig::default();
        
        engine.create_auction("test-auction", item, config).await.unwrap();
        
        // Start the auction
        engine.start_auction("test-auction").await.unwrap();
        
        // Place a bid
        engine.place_bid("test-auction", 1, 100.0).await.unwrap();
        
        // Close and evaluate
        engine.close_auction("test-auction").await.unwrap();
        let result = engine.evaluate_auction("test-auction").await.unwrap();
        
        assert!(result.successful);
        assert_eq!(result.winner, Some(1));
        assert_eq!(result.winning_bid, Some(100.0));
    }
}