//! Resource auction demonstration.
//!
//! This example shows how to use combinatorial and double auctions
//! for resource allocation in a multi‑agent system.

use std::sync::Arc;
use std::time::Duration;

use auction_mechanism::{
    AuctionConfig, AuctionEngine, AuctionType,
    ResourceType, ResourceUnit, ResourceBundle, BidderConstraints,
    CombinatorialAuction, DoubleAuction,
};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Resource Auction Demo ===");

    // 1. Basic auction engine for simple tasks
    println!("1. Basic auction engine for task allocation...");
    let engine = AuctionEngine::new();

    // Register bidders
    let bidder1 = auction_mechanism::types::BidderInfo::new(1, "Agent-1");
    let bidder2 = auction_mechanism::types::BidderInfo::new(2, "Agent-2");
    engine.register_bidder(bidder1).await?;
    engine.register_bidder(bidder2).await?;

    // Create a task auction
    let item = auction_mechanism::types::AuctionItem::new(
        "task-1",
        "Process sensor data",
        "computation",
    );
    let config = AuctionConfig {
        auction_type: AuctionType::English,
        duration_seconds: Some(10),
        ..Default::default()
    };

    engine.create_auction("auction-1", item, config).await?;
    engine.start_auction("auction-1").await?;

    // Simulate bidding
    engine.place_bid("auction-1", 1, 50.0).await?;
    engine.place_bid("auction-1", 2, 55.0).await?;
    engine.place_bid("auction-1", 1, 60.0).await?;

    sleep(Duration::from_millis(100)).await;
    engine.close_auction("auction-1").await?;
    let result = engine.evaluate_auction("auction-1").await?;

    println!("   Auction result: winner = {:?}, winning bid = {:?}",
             result.winner, result.winning_bid);

    // 2. Combinatorial auction for resource bundles
    println!("2. Combinatorial auction for resource bundles...");
    let bundle1 = ResourceBundle::new(
        "bundle-cpu-mem",
        "4 CPU cores + 8 GB RAM",
        vec![
            ResourceUnit::new(ResourceType::CpuCores, 4.0),
            ResourceUnit::new(ResourceType::MemoryMb, 8192.0),
        ],
    );
    let bundle2 = ResourceBundle::new(
        "bundle-gpu",
        "GPU with 8 GB memory",
        vec![
            ResourceUnit::new(ResourceType::GpuMemoryMb, 8192.0),
            ResourceUnit::new(ResourceType::GpuComputeUnits, 1024.0),
        ],
    );

    let config = AuctionConfig {
        auction_type: AuctionType::Combinatorial,
        duration_seconds: Some(30),
        ..Default::default()
    };

    let mut comb_auction = CombinatorialAuction::new(
        "combinatorial-1",
        vec![bundle1, bundle2],
        config,
    )?;

    // Register bidders with constraints
    comb_auction.register_bidder(
        10,
        BidderConstraints {
            max_budget: 500.0,
            max_quantities: vec![
                (ResourceType::CpuCores, 8.0),
                (ResourceType::MemoryMb, 16384.0),
            ].into_iter().collect(),
            ..Default::default()
        },
    )?;
    comb_auction.register_bidder(
        11,
        BidderConstraints {
            max_budget: 300.0,
            max_quantities: vec![
                (ResourceType::GpuMemoryMb, 4096.0),
            ].into_iter().collect(),
            ..Default::default()
        },
    )?;

    // Place bids
    comb_auction.place_bundle_bid(10, "bundle-cpu-mem", 200.0).await?;
    comb_auction.place_bundle_bid(10, "bundle-gpu", 150.0).await?;
    comb_auction.place_bundle_bid(11, "bundle-gpu", 180.0).await?;

    let comb_result = comb_auction.evaluate().await?;
    println!("   Combinatorial auction result:");
    println!("     Winners: {:?}", comb_result.winners);
    println!("     Total revenue: {:.2}", comb_result.total_revenue);
    println!("     Allocation: {:?}", comb_result.allocation);
    println!("     Unsatisfied bidders: {:?}", comb_result.unsatisfied_bidders);

    // 3. Double auction for CPU cores market
    println!("3. Double auction for CPU cores market...");
    let double_auction = DoubleAuction::new("cpu-market", ResourceType::CpuCores, "cores");

    // Buyers (demand)
    double_auction.submit_buy_bid(100, 12.5, 10.0).await?; // 10 cores at $12.5 each
    double_auction.submit_buy_bid(101, 11.0, 5.0).await?;  // 5 cores at $11.0
    double_auction.submit_buy_bid(102, 10.0, 8.0).await?;  // 8 cores at $10.0

    // Sellers (supply)
    double_auction.submit_sell_bid(200, 8.0, 12.0).await?;  // 12 cores at $8.0
    double_auction.submit_sell_bid(201, 9.5, 6.0).await?;   // 6 cores at $9.5
    double_auction.submit_sell_bid(202, 10.5, 4.0).await?;  // 4 cores at $10.5

    let double_result = double_auction.evaluate().await?;
    println!("   Double auction result:");
    println!("     Clearing price: {:?}", double_result.clearing_price);
    println!("     Trades: {}", double_result.trades.len());
    for trade in &double_result.trades {
        println!("       Buyer {} ← Seller {}: {:.1} cores @ ${:.2}",
                 trade.buyer_id, trade.seller_id, trade.quantity, trade.price);
    }
    println!("     Total quantity traded: {:.1}", double_result.total_quantity);
    println!("     Total value: ${:.2}", double_result.total_value);
    println!("     Unmatched buyers: {:?}", double_result.unmatched_buyers);
    println!("     Unmatched sellers: {:?}", double_result.unmatched_sellers);

    // 4. Integration with resource monitoring (simulated)
    println!("4. Simulating integration with resource monitor...");
    let available_resources = vec![
        (ResourceType::CpuCores, 32.0),
        (ResourceType::MemoryMb, 65536.0),
        (ResourceType::StorageGb, 1000.0),
    ];

    println!("   Available resources:");
    for (res_type, qty) in &available_resources {
        println!("     {}: {:.1}", res_type, qty);
    }

    // Create auctions for each scarce resource
    println!("   Creating auctions for scarce resources...");
    for (res_type, total_qty) in &available_resources {
        if total_qty < &50.0 { // scarce if less than 50 units
            println!("     {} is scarce ({} units), starting auction", res_type, total_qty);
            // In a real system, we would create an auction here
        }
    }

    println!("=== Demo completed successfully ===");
    Ok(())
}