//! CRDT‑based map (add‑wins map).

use common::types::{AgentId, VectorClock};
use crdts::{awmap, CmRDT, CvRDT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::delta::{Delta, Op};

/// A conflict‑free replicated map.
pub struct CrdtMap {
    inner: awmap::AWMap<AgentId, String, awmap::ValWrapper<serde_json::Value>>,
    vclock: VectorClock,
    op_log: Vec<Op>,
}

impl CrdtMap {
    /// Create a new empty map.
    pub fn new() -> Self {
        Self {
            inner: awmap::AWMap::new(),
            vclock: VectorClock::default(),
            op_log: Vec::new(),
        }
    }

    /// Insert or update a key with a JSON‑serializable value.
    pub fn set<V: Serialize>(&mut self, key: &str, value: V, author: AgentId) {
        let val = serde_json::to_value(value).expect("Serialization failed");
        let op = self.inner.update(
            key.to_string(),
            awmap::ValWrapper(val.clone()),
            author.0,
            awmap::AWMap::new(),
        );
        self.inner.apply(op);
        self.vclock.increment(author);

        // Record operation
        self.op_log.push(Op::Set {
            key: key.to_string(),
            value: val,
            author,
            seq: self.vclock.entries.get(&author).cloned().unwrap_or(0),
        });
    }

    /// Get the current value for a key, if any.
    pub fn get<V: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<V> {
        self.inner
            .get(key)
            .and_then(|vals| vals.iter().next())
            .map(|wrapped| serde_json::from_value(wrapped.0.clone()).ok())
            .flatten()
    }

    /// Delete a key.
    pub fn delete(&mut self, key: &str, author: AgentId) {
        let op = self.inner.rm(key.to_string(), author.0);
        self.inner.apply(op);
        self.vclock.increment(author);

        self.op_log.push(Op::Delete {
            key: key.to_string(),
            author,
            seq: self.vclock.entries.get(&author).cloned().unwrap_or(0),
        });
    }

    /// Merge another map into this one.
    pub fn merge(&mut self, other: &Self) {
        self.inner.merge(&other.inner);
        // Merge vector clocks
        for (agent, &count) in &other.vclock.entries {
            let entry = self.vclock.entries.entry(*agent).or_insert(0);
            *entry = (*entry).max(count);
        }
        // Merge operation logs (simple concatenation, may contain duplicates)
        self.op_log.extend(other.op_log.clone());
    }

    /// Export the map as a plain HashMap.
    pub fn to_hashmap<V: for<'de> Deserialize<'de>>(&self) -> HashMap<String, V> {
        self.inner
            .iter()
            .filter_map(|(k, vals)| {
                vals.iter()
                    .next()
                    .and_then(|wrapped| serde_json::from_value(wrapped.0.clone()).ok())
                    .map(|v| (k.clone(), v))
            })
            .collect()
    }

    /// Generate a delta representing changes since the given vector clock.
    pub fn delta_since(&self, since: &VectorClock) -> Option<Delta> {
        let mut ops = Vec::new();
        for op in &self.op_log {
            let author = op.author();
            let seq = op.seq();
            if let Some(&known_seq) = since.entries.get(&author) {
                if seq > known_seq {
                    ops.push(op.clone());
                }
            } else {
                // author not in since, include all ops from that author
                ops.push(op.clone());
            }
        }
        if ops.is_empty() {
            None
        } else {
            Some(Delta::new(AgentId(0), ops, self.vclock.clone()))
        }
    }

    /// Apply a delta to this map.
    pub fn apply_delta(&mut self, delta: Delta) {
        delta.apply_to_map(&mut self.inner);
        // Merge vector clocks.
        for (agent, &count) in &delta.vclock.entries {
            let entry = self.vclock.entries.entry(*agent).or_insert(0);
            *entry = (*entry).max(count);
        }
        // Add ops to log
        self.op_log.extend(delta.ops);
    }
}

impl Default for CrdtMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_set_get() {
        let mut map = CrdtMap::new();
        let agent = AgentId(1);
        map.set("foo", json!("bar"), agent);
        let value: serde_json::Value = map.get("foo").unwrap();
        assert_eq!(value, json!("bar"));
        assert!(map.get::<serde_json::Value>("nonexistent").is_none());
    }

    #[test]
    fn test_delete() {
        let mut map = CrdtMap::new();
        let agent = AgentId(1);
        map.set("foo", json!("bar"), agent);
        assert!(map.get::<serde_json::Value>("foo").is_some());
        map.delete("foo", agent);
        assert!(map.get::<serde_json::Value>("foo").is_none());
    }

    #[test]
    fn test_merge() {
        let mut map1 = CrdtMap::new();
        let mut map2 = CrdtMap::new();
        let agent1 = AgentId(1);
        let agent2 = AgentId(2);

        map1.set("key1", json!("value1"), agent1);
        map2.set("key2", json!("value2"), agent2);

        map1.merge(&map2);

        let v1: serde_json::Value = map1.get("key1").unwrap();
        let v2: serde_json::Value = map1.get("key2").unwrap();
        assert_eq!(v1, json!("value1"));
        assert_eq!(v2, json!("value2"));
    }

    #[test]
    fn test_delta_since() {
        let mut map = CrdtMap::new();
        let agent = AgentId(1);
        let empty_vclock = VectorClock::default();

        // No changes yet
        assert!(map.delta_since(&empty_vclock).is_none());

        map.set("foo", json!("bar"), agent);
        let delta = map.delta_since(&empty_vclock).unwrap();
        assert_eq!(delta.ops.len(), 1);
        match &delta.ops[0] {
            Op::Set { key, value, author, seq: _ } => {
                assert_eq!(key, "foo");
                assert_eq!(value, &json!("bar"));
                assert_eq!(*author, agent);
            }
            _ => panic!("Expected Set op"),
        }

        // Delta since a later vclock (no new changes)
        let later_vclock = map.vclock.clone();
        assert!(map.delta_since(&later_vclock).is_none());
    }

    #[test]
    fn test_apply_delta() {
        let mut map1 = CrdtMap::new();
        let mut map2 = CrdtMap::new();
        let agent = AgentId(1);

        map1.set("foo", json!("bar"), agent);
        let delta = map1.delta_since(&VectorClock::default()).unwrap();

        map2.apply_delta(delta);
        let value: serde_json::Value = map2.get("foo").unwrap();
        assert_eq!(value, json!("bar"));
    }

    #[test]
    fn test_to_hashmap() {
        let mut map = CrdtMap::new();
        let agent = AgentId(1);
        map.set("a", json!(1), agent);
        map.set("b", json!(2), agent);

        let hashmap: HashMap<String, serde_json::Value> = map.to_hashmap();
        assert_eq!(hashmap.len(), 2);
        assert_eq!(hashmap.get("a").unwrap(), &json!(1));
        assert_eq!(hashmap.get("b").unwrap(), &json!(2));
    }
}