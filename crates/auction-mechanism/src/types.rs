//! Auction data types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique auction ID.
pub type AuctionId = String;

/// Bidder ID (agent ID).
pub type BidderId = u64;

/// Auction item (resource, task, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionItem {
    /// Item ID.
    pub id: String,
    /// Item description.
    pub description: String,
    /// Item type.
    pub item_type: String,
    /// Item properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Quantity available.
    pub quantity: u32,
    /// Minimum bid (reserve price).
    pub reserve_price: Option<f64>,
}

impl AuctionItem {
    /// Create a new auction item.
    pub fn new(id: &str, description: &str, item_type: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            item_type: item_type.to_string(),
            properties: HashMap::new(),
            quantity: 1,
            reserve_price: None,
        }
    }

    /// Create a new auction item with reserve price.
    pub fn with_reserve_price(id: &str, description: &str, item_type: &str, reserve_price: f64) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            item_type: item_type.to_string(),
            properties: HashMap::new(),
            quantity: 1,
            reserve_price: Some(reserve_price),
        }
    }
}

/// Bid in an auction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bid {
    /// Bid ID.
    pub id: String,
    /// Bidder ID.
    pub bidder_id: BidderId,
    /// Auction ID.
    pub auction_id: AuctionId,
    /// Bid amount.
    pub amount: f64,
    /// Bid timestamp.
    pub timestamp: std::time::SystemTime,
    /// Bid metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Bid {
    /// Create a new bid.
    pub fn new(bidder_id: BidderId, auction_id: &str, amount: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            bidder_id,
            auction_id: auction_id.to_string(),
            amount,
            timestamp: std::time::SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create a new bid with metadata.
    pub fn with_metadata(
        bidder_id: BidderId,
        auction_id: &str,
        amount: f64,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            bidder_id,
            auction_id: auction_id.to_string(),
            amount,
            timestamp: std::time::SystemTime::now(),
            metadata,
        }
    }
}

/// Auction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionResult {
    /// Auction ID.
    pub auction_id: AuctionId,
    /// Winning bidder ID (if any).
    pub winner: Option<BidderId>,
    /// Winning bid amount (if any).
    pub winning_bid: Option<f64>,
    /// Second-highest bid amount (for Vickrey auctions).
    pub second_highest_bid: Option<f64>,
    /// All bids received.
    pub all_bids: Vec<Bid>,
    /// Result timestamp.
    pub timestamp: std::time::SystemTime,
    /// Whether the auction was successful.
    pub successful: bool,
    /// Failure reason if unsuccessful.
    pub failure_reason: Option<String>,
}

impl AuctionResult {
    /// Create a successful auction result.
    pub fn success(
        auction_id: &str,
        winner: BidderId,
        winning_bid: f64,
        all_bids: Vec<Bid>,
    ) -> Self {
        Self {
            auction_id: auction_id.to_string(),
            winner: Some(winner),
            winning_bid: Some(winning_bid),
            second_highest_bid: None,
            all_bids,
            timestamp: std::time::SystemTime::now(),
            successful: true,
            failure_reason: None,
        }
    }

    /// Create a failed auction result.
    pub fn failure(auction_id: &str, failure_reason: &str, all_bids: Vec<Bid>) -> Self {
        Self {
            auction_id: auction_id.to_string(),
            winner: None,
            winning_bid: None,
            second_highest_bid: None,
            all_bids,
            timestamp: std::time::SystemTime::now(),
            successful: false,
            failure_reason: Some(failure_reason.to_string()),
        }
    }
}

/// Auction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuctionState {
    /// Auction is being created.
    Creating,
    /// Auction is open for bidding.
    Open,
    /// Auction is closed (no more bids accepted).
    Closed,
    /// Auction is being evaluated.
    Evaluating,
    /// Auction is completed with a result.
    Completed,
    /// Auction is cancelled.
    Cancelled,
    /// Auction has failed.
    Failed,
}

impl std::fmt::Display for AuctionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuctionState::Creating => write!(f, "creating"),
            AuctionState::Open => write!(f, "open"),
            AuctionState::Closed => write!(f, "closed"),
            AuctionState::Evaluating => write!(f, "evaluating"),
            AuctionState::Completed => write!(f, "completed"),
            AuctionState::Cancelled => write!(f, "cancelled"),
            AuctionState::Failed => write!(f, "failed"),
        }
    }
}

/// Auction type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuctionType {
    /// English auction (ascending price, open cry).
    English,
    /// Dutch auction (descending price).
    Dutch,
    /// First-price sealed-bid.
    FirstPriceSealedBid,
    /// Second-price sealed-bid (Vickrey).
    Vickrey,
    /// Combinatorial auction.
    Combinatorial,
    /// Double auction (buyers and sellers).
    Double,
}

impl std::fmt::Display for AuctionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuctionType::English => write!(f, "english"),
            AuctionType::Dutch => write!(f, "dutch"),
            AuctionType::FirstPriceSealedBid => write!(f, "first_price_sealed_bid"),
            AuctionType::Vickrey => write!(f, "vickrey"),
            AuctionType::Combinatorial => write!(f, "combinatorial"),
            AuctionType::Double => write!(f, "double"),
        }
    }
}

/// Auction configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionConfig {
    /// Auction type.
    pub auction_type: AuctionType,
    /// Auction duration in seconds (if time-limited).
    pub duration_seconds: Option<u64>,
    /// Minimum bid increment.
    pub min_bid_increment: Option<f64>,
    /// Starting price.
    pub starting_price: Option<f64>,
    /// Reserve price (minimum acceptable price).
    pub reserve_price: Option<f64>,
    /// Whether bids are visible to other bidders.
    pub bids_visible: bool,
    /// Allowed bidders (if empty, all bidders are allowed).
    pub allowed_bidders: Vec<BidderId>,
    /// Maximum number of bids per bidder.
    pub max_bids_per_bidder: Option<u32>,
}

impl Default for AuctionConfig {
    fn default() -> Self {
        Self {
            auction_type: AuctionType::English,
            duration_seconds: Some(300), // 5 minutes
            min_bid_increment: Some(1.0),
            starting_price: Some(0.0),
            reserve_price: None,
            bids_visible: true,
            allowed_bidders: Vec::new(),
            max_bids_per_bidder: None,
        }
    }
}

/// Bidder information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidderInfo {
    /// Bidder ID.
    pub id: BidderId,
    /// Bidder name.
    pub name: String,
    /// Bidder reputation score.
    pub reputation: f64,
    /// Maximum bid amount for this bidder.
    pub max_bid_amount: Option<f64>,
    /// Whether the bidder is active.
    pub active: bool,
}

impl BidderInfo {
    /// Create a new bidder info.
    pub fn new(id: BidderId, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            reputation: 1.0,
            max_bid_amount: None,
            active: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auction_item_creation() {
        let item = AuctionItem::new("task-1", "Sample task", "task");
        assert_eq!(item.id, "task-1");
        assert_eq!(item.description, "Sample task");
        assert_eq!(item.item_type, "task");
        assert_eq!(item.quantity, 1);
        assert!(item.reserve_price.is_none());
    }

    #[test]
    fn test_bid_creation() {
        let bid = Bid::new(1, "auction-1", 100.0);
        assert_eq!(bid.bidder_id, 1);
        assert_eq!(bid.auction_id, "auction-1");
        assert_eq!(bid.amount, 100.0);
        assert!(!bid.id.is_empty());
    }

    #[test]
    fn test_auction_result_success() {
        let bids = vec![Bid::new(1, "auction-1", 100.0)];
        let result = AuctionResult::success("auction-1", 1, 100.0, bids);
        
        assert_eq!(result.auction_id, "auction-1");
        assert_eq!(result.winner, Some(1));
        assert_eq!(result.winning_bid, Some(100.0));
        assert!(result.successful);
        assert!(result.failure_reason.is_none());
    }

    #[test]
    fn test_auction_config_default() {
        let config = AuctionConfig::default();
        assert_eq!(config.auction_type, AuctionType::English);
        assert_eq!(config.duration_seconds, Some(300));
        assert_eq!(config.min_bid_increment, Some(1.0));
        assert_eq!(config.starting_price, Some(0.0));
        assert!(config.reserve_price.is_none());
        assert!(config.bids_visible);
        assert!(config.allowed_bidders.is_empty());
    }
}