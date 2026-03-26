//! Delta representation for state synchronization.

use common::types::{AgentId, VectorClock};
use crdts::awmap;
use serde::{Deserialize, Serialize};

/// A single CRDT operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    Set {
        key: String,
        value: serde_json::Value,
        author: AgentId,
        seq: u64,
    },
    Delete {
        key: String,
        author: AgentId,
        seq: u64,
    },
}

impl Op {
    pub fn author(&self) -> AgentId {
        match self {
            Op::Set { author, .. } => *author,
            Op::Delete { author, .. } => *author,
        }
    }

    pub fn seq(&self) -> u64 {
        match self {
            Op::Set { seq, .. } => *seq,
            Op::Delete { seq, .. } => *seq,
        }
    }
}

/// A delta represents a set of changes made by a particular agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Agent that produced this delta.
    pub author: AgentId,
    /// Vector clock after applying the delta.
    pub vclock: VectorClock,
    /// List of operations.
    pub ops: Vec<Op>,
    /// Timestamp (logical).
    pub timestamp: u64,
}

impl Delta {
    /// Create a new delta from a list of operations.
    pub fn new(author: AgentId, ops: Vec<Op>, vclock: VectorClock) -> Self {
        Self {
            author,
            vclock,
            ops,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    /// Merge two deltas (combine changes).
    pub fn merge(&mut self, other: &Self) {
        // Merge ops: just concatenate (in a real implementation we would deduplicate)
        self.ops.extend(other.ops.clone());
        // Merge vector clocks
        for (agent, &count) in &other.vclock.entries {
            let entry = self.vclock.entries.entry(*agent).or_insert(0);
            *entry = (*entry).max(count);
        }
        self.timestamp = self.timestamp.max(other.timestamp);
    }

    /// Apply this delta to an AWMap.
    pub fn apply_to_map(&self, map: &mut awmap::AWMap<AgentId, String, awmap::ValWrapper<serde_json::Value>>) {
        for op in &self.ops {
            match op {
                Op::Set { key, value, author, seq: _ } => {
                    let op = map.update(
                        key.clone(),
                        awmap::ValWrapper(value.clone()),
                        author.0,
                        awmap::AWMap::new(),
                    );
                    map.apply(op);
                }
                Op::Delete { key, author, seq: _ } => {
                    let op = map.rm(key.clone(), author.0);
                    map.apply(op);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_op_author_seq() {
        let op_set = Op::Set {
            key: "k".to_string(),
            value: json!("v"),
            author: AgentId(5),
            seq: 42,
        };
        assert_eq!(op_set.author(), AgentId(5));
        assert_eq!(op_set.seq(), 42);

        let op_del = Op::Delete {
            key: "k".to_string(),
            author: AgentId(7),
            seq: 99,
        };
        assert_eq!(op_del.author(), AgentId(7));
        assert_eq!(op_del.seq(), 99);
    }

    #[test]
    fn test_delta_new() {
        let vclock = VectorClock::from_entries(vec![(AgentId(1), 5)]);
        let ops = vec![
            Op::Set {
                key: "foo".to_string(),
                value: json!("bar"),
                author: AgentId(1),
                seq: 5,
            },
        ];
        let delta = Delta::new(AgentId(1), ops.clone(), vclock.clone());
        assert_eq!(delta.author, AgentId(1));
        assert_eq!(delta.vclock, vclock);
        assert_eq!(delta.ops.len(), 1);
        assert!(delta.timestamp > 0);
    }

    #[test]
    fn test_delta_merge() {
        let vclock1 = VectorClock::from_entries(vec![(AgentId(1), 5)]);
        let ops1 = vec![Op::Set {
            key: "a".to_string(),
            value: json!(1),
            author: AgentId(1),
            seq: 5,
        }];
        let mut delta1 = Delta::new(AgentId(1), ops1, vclock1);

        let vclock2 = VectorClock::from_entries(vec![(AgentId(2), 3)]);
        let ops2 = vec![Op::Delete {
            key: "b".to_string(),
            author: AgentId(2),
            seq: 3,
        }];
        let delta2 = Delta::new(AgentId(2), ops2, vclock2);

        delta1.merge(&delta2);
        assert_eq!(delta1.ops.len(), 2);
        assert_eq!(delta1.vclock.entries.get(&AgentId(1)), Some(&5));
        assert_eq!(delta1.vclock.entries.get(&AgentId(2)), Some(&3));
    }

    #[test]
    fn test_apply_to_map() {
        use crdts::awmap::AWMap;
        let mut map = AWMap::new();
        let delta = Delta::new(
            AgentId(1),
            vec![
                Op::Set {
                    key: "x".to_string(),
                    value: json!("hello"),
                    author: AgentId(1),
                    seq: 1,
                },
                Op::Set {
                    key: "y".to_string(),
                    value: json!(42),
                    author: AgentId(1),
                    seq: 2,
                },
            ],
            VectorClock::from_entries(vec![(AgentId(1), 2)]),
        );

        delta.apply_to_map(&mut map);
        // Check that values are present
        let vals_x = map.get("x");
        let vals_y = map.get("y");
        assert!(vals_x.is_some());
        assert!(vals_y.is_some());
        // The map stores values as ValWrapper, we can't directly compare.
        // Instead we can verify that the map contains the keys.
        let keys: Vec<_> = map.iter().map(|(k, _)| k).collect();
        assert!(keys.contains(&"x".to_string()));
        assert!(keys.contains(&"y".to_string()));
    }
}