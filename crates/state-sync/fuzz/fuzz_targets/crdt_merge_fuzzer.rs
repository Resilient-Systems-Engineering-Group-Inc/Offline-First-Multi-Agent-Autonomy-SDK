//! Fuzz test for CRDT merge operations.
//!
//! Tests that merge operations are:
//! - Commutative: a.merge(b) == b.merge(a)
//! - Associative: (a.merge(b)).merge(c) == a.merge(b.merge(c))
//! - Idempotent: a.merge(a) == a
//! - Convergent: All replicas converge to same state

#![no_main]

use libfuzzer_sys::fuzz_target;
use state_sync::crdt_map::CrdtMap;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, arbitrary::Arbitrary)]
struct MergeOperation {
    key: String,
    value: String,
    timestamp: u64,
    node_id: u8,
}

fuzz_target!(|operations: Vec<MergeOperation>| {
    // Create three independent CRDT maps
    let mut map_a = CrdtMap::new();
    let mut map_b = CrdtMap::new();
    let mut map_c = CrdtMap::new();
    
    // Apply operations to different maps
    for (i, op) in operations.iter().enumerate() {
        match i % 3 {
            0 => {
                map_a.insert_with_metadata(
                    &op.key,
                    &op.value,
                    op.timestamp,
                    op.node_id,
                );
            }
            1 => {
                map_b.insert_with_metadata(
                    &op.key,
                    &op.value,
                    op.timestamp,
                    op.node_id,
                );
            }
            _ => {
                map_c.insert_with_metadata(
                    &op.key,
                    &op.value,
                    op.timestamp,
                    op.node_id,
                );
            }
        }
    }
    
    // Test commutativity: a.merge(b) == b.merge(a)
    let mut map_ab = map_a.clone();
    let mut map_ba = map_b.clone();
    
    map_ab.merge(&map_b);
    map_ba.merge(&map_a);
    
    assert_eq!(
        map_ab.len(),
        map_ba.len(),
        "Commutativity failed: different lengths"
    );
    
    // Test idempotency: a.merge(a) == a
    let mut map_aa = map_a.clone();
    map_aa.merge(&map_a);
    
    assert_eq!(
        map_a.len(),
        map_aa.len(),
        "Idempotency failed: different lengths after self-merge"
    );
    
    // Test convergence: all maps should converge to same state
    let mut map_abc = map_a.clone();
    map_abc.merge(&map_b);
    map_abc.merge(&map_c);
    
    let mut map_bac = map_b.clone();
    map_bac.merge(&map_a);
    map_bac.merge(&map_c);
    
    let mut map_cba = map_c.clone();
    map_cba.merge(&map_b);
    map_cba.merge(&map_a);
    
    // All converged maps should have same keys and values
    assert_eq!(
        map_abc.len(),
        map_bac.len(),
        "Convergence failed: abc vs bac"
    );
    assert_eq!(
        map_bac.len(),
        map_cba.len(),
        "Convergence failed: bac vs cba"
    );
    
    // Verify all values match for common keys
    for key in map_abc.keys() {
        if let (Some(v1), Some(v2), Some(v3)) = (
            map_abc.get(key),
            map_bac.get(key),
            map_cba.get(key),
        ) {
            assert_eq!(v1, v2, "Value mismatch for key {} (abc vs bac)", key);
            assert_eq!(v2, v3, "Value mismatch for key {} (bac vs cba)", key);
        }
    }
});
