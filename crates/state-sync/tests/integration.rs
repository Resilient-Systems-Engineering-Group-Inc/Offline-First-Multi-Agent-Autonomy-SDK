//! Integration tests for state synchronization.

use state_sync::{DefaultStateSync, StateSync};
use common::types::AgentId;
use serde_json::json;

#[tokio::test]
async fn test_two_agents_sync() {
    let mut sync1 = DefaultStateSync::new(AgentId(1));
    let mut sync2 = DefaultStateSync::new(AgentId(2));

    // Agent 1 sets a value
    sync1.set_value("counter", json!(42)).expect("set failed");
    let delta1 = sync1.generate_delta().expect("generate delta failed");
    assert!(!delta1.is_empty());

    // Apply delta to agent 2
    sync2.apply_delta(&delta1).expect("apply delta failed");
    let value2 = sync2.get_value::<i64>("counter").expect("get failed");
    assert_eq!(value2, 42);

    // Agent 2 updates the value
    sync2.set_value("counter", json!(100)).expect("set failed");
    let delta2 = sync2.generate_delta().expect("generate delta failed");
    assert!(!delta2.is_empty());

    // Apply back to agent 1
    sync1.apply_delta(&delta2).expect("apply delta failed");
    let value1 = sync1.get_value::<i64>("counter").expect("get failed");
    assert_eq!(value1, 100);

    // Both should have the same value
    assert_eq!(value1, value2);
}

#[tokio::test]
async fn test_concurrent_updates_converge() {
    let mut sync_a = DefaultStateSync::new(AgentId(1));
    let mut sync_b = DefaultStateSync::new(AgentId(2));

    // Simulate concurrent updates: A sets "x" = 1, B sets "x" = 2
    sync_a.set_value("x", json!(1)).unwrap();
    sync_b.set_value("x", json!(2)).unwrap();

    // Generate deltas
    let delta_a = sync_a.generate_delta().unwrap();
    let delta_b = sync_b.generate_delta().unwrap();

    // Apply deltas in opposite order
    sync_b.apply_delta(&delta_a).unwrap();
    sync_a.apply_delta(&delta_b).unwrap();

    // CRDT should converge to the same value (last writer wins? depends on CRDT)
    // For AWMap, the value with the highest vector clock wins.
    // Since we don't know which one wins, we just ensure they are equal.
    let val_a = sync_a.get_value::<i64>("x").unwrap();
    let val_b = sync_b.get_value::<i64>("x").unwrap();
    assert_eq!(val_a, val_b);
}

#[tokio::test]
async fn test_multiple_keys() {
    let mut sync = DefaultStateSync::new(AgentId(1));
    sync.set_value("a", json!("hello")).unwrap();
    sync.set_value("b", json!(true)).unwrap();
    sync.set_value("c", json!([1,2,3])).unwrap();

    let delta = sync.generate_delta().unwrap();
    let mut sync2 = DefaultStateSync::new(AgentId(2));
    sync2.apply_delta(&delta).unwrap();

    assert_eq!(sync2.get_value::<String>("a").unwrap(), "hello");
    assert_eq!(sync2.get_value::<bool>("b").unwrap(), true);
    assert_eq!(sync2.get_value::<Vec<i32>>("c").unwrap(), vec![1,2,3]);
}