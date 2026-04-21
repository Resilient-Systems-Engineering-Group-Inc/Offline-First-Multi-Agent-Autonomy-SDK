//! Fuzz test for delta serialization and deserialization.
//!
//! Ensures that deltas can be serialized/deserialized without errors
//! and that round-trip serialization preserves data.

#![no_main]

use libfuzzer_sys::fuzz_target;
use state_sync::delta::Delta;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, arbitrary::Arbitrary)]
struct DeltaEntry {
    key: String,
    value: String,
    operation: DeltaOperation,
    timestamp: u64,
    vector_clock: HashMap<u8, u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, arbitrary::Arbitrary)]
enum DeltaOperation {
    Put,
    Delete,
    Update,
}

fuzz_target!(|entries: Vec<DeltaEntry>| {
    // Create a delta from fuzzed entries
    let mut delta = Delta::new();
    
    for entry in &entries {
        match entry.operation {
            DeltaOperation::Put | DeltaOperation::Update => {
                delta.insert(
                    entry.key.clone(),
                    entry.value.clone(),
                    entry.timestamp,
                    entry.vector_clock.clone(),
                );
            }
            DeltaOperation::Delete => {
                delta.remove(&entry.key, entry.timestamp, entry.vector_clock.clone());
            }
        }
    }
    
    // Test serialization round-trip
    let serialized = bincode::serialize(&delta).expect("Failed to serialize delta");
    let deserialized: Delta = bincode::deserialize(&serialized)
        .expect("Failed to deserialize delta");
    
    // Verify deserialized delta matches original
    assert_eq!(
        delta.len(),
        deserialized.len(),
        "Delta length mismatch after round-trip"
    );
    
    // Verify all entries are preserved
    for (key, value) in delta.entries() {
        assert!(
            deserialized.contains_key(key),
            "Key {} missing after round-trip",
            key
        );
        
        if let Some(deser_value) = deserialized.get(key) {
            assert_eq!(
                value,
                deser_value,
                "Value mismatch for key {} after round-trip",
                key
            );
        }
    }
    
    // Test delta compression
    let delta_1 = delta.clone();
    let delta_2 = Delta::new(); // Empty delta
    
    let compressed = delta_1.compress(&delta_2);
    assert!(
        compressed.len() <= delta_1.len(),
        "Compression increased size"
    );
    
    // Test delta application
    let mut base_map = state_sync::crdt_map::CrdtMap::new();
    
    // Apply delta to base map
    base_map.apply_delta(&delta).expect("Failed to apply delta");
    
    // Verify map contains all delta entries
    for (key, expected_value) in delta.entries() {
        if let Some(actual_value) = base_map.get(key) {
            assert_eq!(
                expected_value,
                actual_value,
                "Applied delta value mismatch for key {}",
                key
            );
        }
    }
});
