//! Resource‑oriented auction mechanisms.
//!
//! This module extends the basic auction system with resource‑specific features:
//! - Resource types (CPU, memory, bandwidth, storage, GPU, etc.)
//! - Combinatorial auctions for resource bundles
//! - Double auctions for matching supply and demand
//! - Budget and constraint‑aware bidding

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{AuctionError, Result};
use crate::types::{AuctionConfig, AuctionId, AuctionItem, AuctionType, Bid, BidderId};

/// Resource type for auction items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// CPU cores (count).
    CpuCores,
    /// Memory in megabytes.
    MemoryMb,
    /// Disk storage in gigabytes.
    StorageGb,
    /// Network bandwidth in megabits per second.
    BandwidthMbps,
    /// GPU memory in megabytes.
    GpuMemoryMb,
    /// GPU compute units.
    GpuComputeUnits,
    /// Energy in watt‑hours.
    EnergyWh,
    /// Time slot (duration in seconds).
    TimeSlot,
    /// Custom resource type.
    Custom(&'static str),
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::CpuCores => write!(f, "cpu_cores"),
            ResourceType::MemoryMb => write!(f, "memory_mb"),
            ResourceType::StorageGb => write!(f, "storage_gb"),
            ResourceType::BandwidthMbps => write!(f, "bandwidth_mbps"),
            ResourceType::GpuMemoryMb => write!(f, "gpu_memory_mb"),
            ResourceType::GpuComputeUnits => write!(f, "gpu_compute_units"),
            ResourceType::EnergyWh => write!(f, "energy_wh"),
            ResourceType::TimeSlot => write!(f, "time_slot"),
            ResourceType::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// A resource unit with type and quantity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUnit {
    /// Resource type.
    pub resource_type: ResourceType,
    /// Quantity (units depend on resource type).
    pub quantity: f64,
    /// Quality or performance level (0.0 to 1.0).
    pub quality: Option<f64>,
    /// Location constraint (optional).
    pub location: Option<String>,
    /// Time window (start, end) as Unix timestamps.
    pub time_window: Option<(i64, i64)>,
}

impl ResourceUnit {
    /// Create a new resource unit.
    pub fn new(resource_type: ResourceType, quantity: f64) -> Self {
        Self {
            resource_type,
            quantity,
            quality: None,
            location: None,
            time_window: None,
        }
    }

    /// Create a resource unit with quality.
    pub fn with_quality(resource_type: ResourceType, quantity: f64, quality: f64) -> Self {
        Self {
            resource_type,
            quantity,
            quality: Some(quality),
            location: None,
            time_window: None,
        }
    }
}

/// A bundle of resources that can be auctioned together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBundle {
    /// Bundle ID.
    pub id: String,
    /// Description.
    pub description: String,
    /// Resources in this bundle.
    pub resources: Vec<ResourceUnit>,
    /// Whether the bundle is indivisible (all‑or‑nothing).
    pub indivisible: bool,
    /// Complementary resources (must be allocated together).
    pub complements: Vec<String>,
    /// Substitutes (alternative bundle IDs).
    pub substitutes: Vec<String>,
}

impl ResourceBundle {
    /// Create a new resource bundle.
    pub fn new(id: &str, description: &str, resources: Vec<ResourceUnit>) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            resources,
            indivisible: true,
            complements: Vec::new(),
            substitutes: Vec::new(),
        }
    }

    /// Total quantity of a specific resource type in the bundle.
    pub fn total_of(&self, resource_type: ResourceType) -> f64 {
        self.resources
            .iter()
            .filter(|r| r.resource_type == resource_type)
            .map(|r| r.quantity)
            .sum()
    }
}

/// Bidder constraints (budget, preferences, limits).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidderConstraints {
    /// Maximum total budget.
    pub max_budget: f64,
    /// Maximum quantity per resource type.
    pub max_quantities: HashMap<ResourceType, f64>,
    /// Preferred resource types (weighted).
    pub preferences: HashMap<ResourceType, f64>,
    /// Required resource types (must have at least some amount).
    pub required: HashSet<ResourceType>,
    /// Time constraints (earliest start, latest finish).
    pub time_constraints: Option<(i64, i64)>,
}

impl Default for BidderConstraints {
    fn default() -> Self {
        Self {
            max_budget: f64::INFINITY,
            max_quantities: HashMap::new(),
            preferences: HashMap::new(),
            required: HashSet::new(),
            time_constraints: None,
        }
    }
}

/// Combinatorial auction that supports bidding on bundles.
pub struct CombinatorialAuction {
    /// Auction ID.
    id: AuctionId,
    /// Bundles being auctioned.
    bundles: Vec<ResourceBundle>,
    /// Bids: map from bidder ID to list of bundle bids (bundle ID -> amount).
    bids: RwLock<HashMap<BidderId, HashMap<String, f64>>>,
    /// Constraints per bidder.
    constraints: HashMap<BidderId, BidderConstraints>,
    /// Auction configuration.
    config: AuctionConfig,
}

impl CombinatorialAuction {
    /// Create a new combinatorial auction.
    pub fn new(
        id: &str,
        bundles: Vec<ResourceBundle>,
        config: AuctionConfig,
    ) -> Result<Self> {
        if config.auction_type != AuctionType::Combinatorial {
            return Err(AuctionError::InvalidAuctionType(
                "Combinatorial auction requires AuctionType::Combinatorial".to_string(),
            ));
        }

        Ok(Self {
            id: id.to_string(),
            bundles,
            bids: RwLock::new(HashMap::new()),
            constraints: HashMap::new(),
            config,
        })
    }

    /// Register a bidder with constraints.
    pub fn register_bidder(
        &mut self,
        bidder_id: BidderId,
        constraints: BidderConstraints,
    ) -> Result<()> {
        if self.constraints.contains_key(&bidder_id) {
            return Err(AuctionError::BidderAlreadyRegistered(bidder_id));
        }
        self.constraints.insert(bidder_id, constraints);
        Ok(())
    }

    /// Place a bid on a specific bundle.
    pub async fn place_bundle_bid(
        &self,
        bidder_id: BidderId,
        bundle_id: &str,
        amount: f64,
    ) -> Result<()> {
        // Check if bundle exists
        if !self.bundles.iter().any(|b| b.id == bundle_id) {
            return Err(AuctionError::InvalidBid(format!(
                "Bundle {} does not exist",
                bundle_id
            )));
        }

        // Check bidder constraints
        if let Some(constraints) = self.constraints.get(&bidder_id) {
            if amount > constraints.max_budget {
                return Err(AuctionError::InvalidBid(format!(
                    "Bid amount {} exceeds max budget {}",
                    amount, constraints.max_budget
                )));
            }
        }

        let mut bids = self.bids.write().await;
        let bidder_bids = bids.entry(bidder_id).or_insert_with(HashMap::new);
        bidder_bids.insert(bundle_id.to_string(), amount);

        Ok(())
    }

    /// Evaluate the combinatorial auction using a greedy algorithm.
    pub async fn evaluate(&self) -> Result<CombinatorialAuctionResult> {
        let bids = self.bids.read().await;

        if bids.is_empty() {
            return Ok(CombinatorialAuctionResult {
                auction_id: self.id.clone(),
                winners: Vec::new(),
                total_revenue: 0.0,
                allocation: HashMap::new(),
                unsatisfied_bidders: bids.keys().copied().collect(),
            });
        }

        // Simple greedy winner determination: highest bid per bundle wins
        let mut bundle_winners: HashMap<String, (BidderId, f64)> = HashMap::new();
        let mut unsatisfied = HashSet::new();

        for (bidder_id, bidder_bids) in bids.iter() {
            for (bundle_id, amount) in bidder_bids {
                let entry = bundle_winners.entry(bundle_id.clone()).or_insert((*bidder_id, *amount));
                if *amount > entry.1 {
                    *entry = (*bidder_id, *amount);
                }
            }
        }

        // Build result
        let mut winners = Vec::new();
        let mut total_revenue = 0.0;
        let mut allocation = HashMap::new();

        for (bundle_id, (winner_id, amount)) in bundle_winners {
            winners.push(winner_id);
            total_revenue += amount;
            allocation.insert(bundle_id, winner_id);
        }

        // Find unsatisfied bidders (those who didn't win any bundle)
        for bidder_id in bids.keys() {
            if !winners.contains(bidder_id) {
                unsatisfied.insert(*bidder_id);
            }
        }

        Ok(CombinatorialAuctionResult {
            auction_id: self.id.clone(),
            winners,
            total_revenue,
            allocation,
            unsatisfied_bidders: unsatisfied,
        })
    }

    /// Get all bundles.
    pub fn bundles(&self) -> &[ResourceBundle] {
        &self.bundles
    }
}

/// Result of a combinatorial auction.
#[derive(Debug, Clone, Serialize)]
pub struct CombinatorialAuctionResult {
    /// Auction ID.
    pub auction_id: AuctionId,
    /// Winning bidder IDs.
    pub winners: Vec<BidderId>,
    /// Total revenue collected.
    pub total_revenue: f64,
    /// Allocation mapping bundle ID to winner ID.
    pub allocation: HashMap<String, BidderId>,
    /// Bidders who did not win any bundle.
    pub unsatisfied_bidders: HashSet<BidderId>,
}

/// Double auction for matching buyers and sellers.
pub struct DoubleAuction {
    /// Auction ID.
    id: AuctionId,
    /// Buy bids (bidder ID -> price per unit, quantity).
    buy_bids: RwLock<Vec<(BidderId, f64, f64)>>,
    /// Sell bids (bidder ID -> price per unit, quantity).
    sell_bids: RwLock<Vec<(BidderId, f64, f64)>>,
    /// Resource type being traded.
    resource_type: ResourceType,
    /// Unit of quantity.
    unit: String,
    /// Clearing price (set after evaluation).
    clearing_price: RwLock<Option<f64>>,
}

impl DoubleAuction {
    /// Create a new double auction.
    pub fn new(id: &str, resource_type: ResourceType, unit: &str) -> Self {
        Self {
            id: id.to_string(),
            buy_bids: RwLock::new(Vec::new()),
            sell_bids: RwLock::new(Vec::new()),
            resource_type,
            unit: unit.to_string(),
            clearing_price: RwLock::new(None),
        }
    }

    /// Submit a buy bid (demand).
    pub async fn submit_buy_bid(&self, bidder_id: BidderId, price: f64, quantity: f64) -> Result<()> {
        if price <= 0.0 || quantity <= 0.0 {
            return Err(AuctionError::InvalidBid(
                "Price and quantity must be positive".to_string(),
            ));
        }

        let mut bids = self.buy_bids.write().await;
        bids.push((bidder_id, price, quantity));
        // Sort by price descending (highest buy price first)
        bids.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(())
    }

    /// Submit a sell bid (supply).
    pub async fn submit_sell_bid(&self, bidder_id: BidderId, price: f64, quantity: f64) -> Result<()> {
        if price <= 0.0 || quantity <= 0.0 {
            return Err(AuctionError::InvalidBid(
                "Price and quantity must be positive".to_string(),
            ));
        }

        let mut bids = self.sell_bids.write().await;
        bids.push((bidder_id, price, quantity));
        // Sort by price ascending (lowest sell price first)
        bids.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(())
    }

    /// Evaluate the double auction using standard clearing price mechanism.
    pub async fn evaluate(&self) -> Result<DoubleAuctionResult> {
        let buy_bids = self.buy_bids.read().await.clone();
        let sell_bids = self.sell_bids.read().await.clone();

        if buy_bids.is_empty() || sell_bids.is_empty() {
            return Ok(DoubleAuctionResult {
                auction_id: self.id.clone(),
                clearing_price: None,
                trades: Vec::new(),
                total_quantity: 0.0,
                total_value: 0.0,
                unmatched_buyers: buy_bids.iter().map(|(id, _, _)| *id).collect(),
                unmatched_sellers: sell_bids.iter().map(|(id, _, _)| *id).collect(),
            });
        }

        // Aggregate demand and supply curves
        let mut demand_curve = Vec::new();
        let mut cumulative_qty = 0.0;
        for (bidder_id, price, qty) in &buy_bids {
            cumulative_qty += qty;
            demand_curve.push((*price, cumulative_qty, *bidder_id));
        }

        let mut supply_curve = Vec::new();
        cumulative_qty = 0.0;
        for (bidder_id, price, qty) in &sell_bids {
            cumulative_qty += qty;
            supply_curve.push((*price, cumulative_qty, *bidder_id));
        }

        // Find intersection (clearing price)
        let mut clearing_price = None;
        let mut clearing_quantity = 0.0;

        // Simple algorithm: find price where demand >= supply
        for (buy_price, buy_qty, _) in &demand_curve {
            for (sell_price, sell_qty, _) in &supply_curve {
                if buy_price >= sell_price && buy_qty >= sell_qty {
                    clearing_price = Some((buy_price + sell_price) / 2.0);
                    clearing_quantity = sell_qty.min(*buy_qty);
                    break;
                }
            }
            if clearing_price.is_some() {
                break;
            }
        }

        let clearing_price = match clearing_price {
            Some(p) => p,
            None => {
                // No intersection
                return Ok(DoubleAuctionResult {
                    auction_id: self.id.clone(),
                    clearing_price: None,
                    trades: Vec::new(),
                    total_quantity: 0.0,
                    total_value: 0.0,
                    unmatched_buyers: buy_bids.iter().map(|(id, _, _)| *id).collect(),
                    unmatched_sellers: sell_bids.iter().map(|(id, _, _)| *id).collect(),
                });
            }
        };

        // Determine trades
        let mut trades = Vec::new();
        let mut total_qty = 0.0;
        let mut total_value = 0.0;

        // Match buyers and sellers at clearing price
        let mut buy_idx = 0;
        let mut sell_idx = 0;
        let mut buy_remaining = if !buy_bids.is_empty() { buy_bids[0].2 } else { 0.0 };
        let mut sell_remaining = if !sell_bids.is_empty() { sell_bids[0].2 } else { 0.0 };

        while buy_idx < buy_bids.len() && sell_idx < sell_bids.len() {
            let (buyer_id, buy_price, _) = buy_bids[buy_idx];
            let (seller_id, sell_price, _) = sell_bids[sell_idx];

            if buy_price < clearing_price || sell_price > clearing_price {
                // Cannot trade at this price
                break;
            }

            let trade_qty = buy_remaining.min(sell_remaining);
            if trade_qty > 0.0 {
                trades.push(Trade {
                    buyer_id,
                    seller_id,
                    quantity: trade_qty,
                    price: clearing_price,
                });
                total_qty += trade_qty;
                total_value += trade_qty * clearing_price;
            }

            buy_remaining -= trade_qty;
            sell_remaining -= trade_qty;

            if buy_remaining <= 1e-9 {
                buy_idx += 1;
                if buy_idx < buy_bids.len() {
                    buy_remaining = buy_bids[buy_idx].2;
                }
            }
            if sell_remaining <= 1e-9 {
                sell_idx += 1;
                if sell_idx < sell_bids.len() {
                    sell_remaining = sell_bids[sell_idx].2;
                }
            }
        }

        // Store clearing price
        *self.clearing_price.write().await = Some(clearing_price);

        Ok(DoubleAuctionResult {
            auction_id: self.id.clone(),
            clearing_price: Some(clearing_price),
            trades,
            total_quantity: total_qty,
            total_value,
            unmatched_buyers: buy_bids[buy_idx..].iter().map(|(id, _, _)| *id).collect(),
            unmatched_sellers: sell_bids[sell_idx..].iter().map(|(id, _, _)| *id).collect(),
        })
    }
}

/// A trade in a double auction.
#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    /// Buyer ID.
    pub buyer_id: BidderId,
    /// Seller ID.
    pub seller_id: BidderId,
    /// Quantity traded.
    pub quantity: f64,
    /// Price per unit.
    pub price: f64,
}

/// Result of a double auction.
#[derive(Debug, Clone, Serialize)]
pub struct DoubleAuctionResult {
    /// Auction ID.
    pub auction_id: AuctionId,
    /// Market clearing price (if any).
    pub clearing_price: Option<f64>,
    /// Executed trades.
    pub trades: Vec<Trade>,
    /// Total quantity traded.
    pub total_quantity: f64,
    /// Total monetary value exchanged.
    pub total_value: f64,
    /// Buyers who could not be matched.
    pub unmatched_buyers: Vec<BidderId>,
    /// Sellers who could not be matched.
    pub unmatched_sellers: Vec<BidderId>,
}

/// Utility functions for resource auctions.
pub mod utils {
    use super::*;

    /// Convert resource bundle to auction item.
    pub fn bundle_to_auction_item(bundle: &ResourceBundle) -> AuctionItem {
        AuctionItem::new(&bundle.id, &bundle.description, "resource_bundle")
    }

    /// Check if a bid satisfies constraints.
    pub fn check_constraints(
        bid_amount: f64,
        bundle: &ResourceBundle,
        constraints: &BidderConstraints,
    ) -> bool {
        if bid_amount > constraints.max_budget {
            return false;
        }

        for resource in &bundle.resources {
            if let Some(max_qty) = constraints.max_quantities.get(&resource.resource_type) {
                if resource.quantity > *max_qty {
                    return false;
                }
            }
        }

        true
    }

    /// Compute efficiency of an allocation (total value / max possible).
    pub fn allocation_efficiency(
        winners: &[BidderId],
        bids: &HashMap<BidderId, HashMap<String, f64>>,
    ) -> f64 {
        let mut total_value = 0.0;
        let mut max_possible = 0.0;

        for (bidder_id, bidder_bids) in bids {
            let max_bid = bidder_bids.values().copied().fold(0.0, f64::max);
            max_possible += max_bid;
            if winners.contains(bidder_id) {
                // Assume winner gets their highest bid
                total_value += max_bid;
            }
        }

        if max_possible == 0.0 {
            1.0
        } else {
            total_value / max_possible
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_combinatorial_auction() {
        let bundle1 = ResourceBundle::new(
            "bundle-1",
            "CPU + Memory",
            vec![
                ResourceUnit::new(ResourceType::CpuCores, 4.0),
                ResourceUnit::new(ResourceType::MemoryMb, 8192.0),
            ],
        );
        let bundle2 = ResourceBundle::new(
            "bundle-2",
            "Storage",
            vec![ResourceUnit::new(ResourceType::StorageGb, 500.0)],
        );

        let config = AuctionConfig {
            auction_type: AuctionType::Combinatorial,
            ..Default::default()
        };

        let mut auction = CombinatorialAuction::new("test", vec![bundle1, bundle2], config).unwrap();

        // Register bidders
        auction
            .register_bidder(
                1,
                BidderConstraints {
                    max_budget: 200.0,
                    ..Default::default()
                },
            )
            .unwrap();
        auction
            .register_bidder(
                2,
                BidderConstraints {
                    max_budget: 150.0,
                    ..Default::default()
                },
            )
            .unwrap();

        // Place bids
        auction.place_bundle_bid(1, "bundle-1", 100.0).await.unwrap();
        auction.place_bundle_bid(1, "bundle-2", 50.0).await.unwrap();
        auction.place_bundle_bid(2, "bundle-1", 120.0).await.unwrap();

        let result = auction.evaluate().await.unwrap();
        assert_eq!(result.winners.len(), 2); // Both bidders win something
        assert!(result.total_revenue > 0.0);
    }

    #[tokio::test]
    async fn test_double_auction() {
        let auction = DoubleAuction::new("test-double", ResourceType::CpuCores, "cores");

        // Buy bids (demand)
        auction.submit_buy_bid(1, 10.0, 5.0).await.unwrap();
        auction.submit_buy_bid(2, 8.0, 3.0).await.unwrap();

        // Sell bids (supply)
        auction.submit_sell_bid(3, 6.0, 4.0).await.unwrap();
        auction.submit_sell_bid(4, 7.0, 3.0).await.unwrap();

        let result = auction.evaluate().await.unwrap();
        assert!(result.clearing_price.is_some());
        assert!(!result.trades.is_empty());
        assert!(result.total_quantity > 0.0);
    }
}