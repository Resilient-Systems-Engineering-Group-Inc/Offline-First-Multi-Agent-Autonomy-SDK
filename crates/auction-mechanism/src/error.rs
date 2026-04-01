//! Error types for auction mechanisms.

use thiserror::Error;

/// Errors that can occur in auction operations.
#[derive(Error, Debug)]
pub enum AuctionError {
    /// Auction not found.
    #[error("Auction {0} not found")]
    AuctionNotFound(String),

    /// Auction already exists.
    #[error("Auction {0} already exists")]
    AuctionAlreadyExists(String),

    /// Auction is not in the correct state for the operation.
    #[error("Auction {0} is not in the correct state: expected {1}, got {2}")]
    InvalidAuctionState(String, String, String),

    /// Bid is invalid.
    #[error("Invalid bid: {0}")]
    InvalidBid(String),

    /// Bidder is not registered.
    #[error("Bidder {0} is not registered")]
    BidderNotRegistered(u64),

    /// Bidder is not allowed to bid.
    #[error("Bidder {0} is not allowed to bid: {1}")]
    BidderNotAllowed(u64, String),

    /// Auction has no bidders.
    #[error("Auction has no bidders")]
    NoBidders,

    /// Auction has no valid bids.
    #[error("Auction has no valid bids")]
    NoValidBids,

    /// Reserve price not met.
    #[error("Reserve price not met: {0}")]
    ReservePriceNotMet(f64),

    /// Timeout during auction.
    #[error("Auction timeout: {0}")]
    Timeout(String),

    /// Invalid auction configuration.
    #[error("Invalid auction configuration: {0}")]
    InvalidConfig(String),

    /// Combinatorial auction error.
    #[error("Combinatorial auction error: {0}")]
    CombinatorialError(String),

    /// Payment/clearing error.
    #[error("Payment/clearing error: {0}")]
    PaymentError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Transport error.
    #[error("Transport error: {0}")]
    TransportError(#[from] mesh_transport::Error),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for auction operations.
pub type Result<T> = std::result::Result<T, AuctionError>;