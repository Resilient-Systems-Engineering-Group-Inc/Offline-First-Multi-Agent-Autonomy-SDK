# State Sync (CRDT) Specification

## Overview
State Sync provides a conflict‑free replicated key‑value store that allows agents to maintain a shared, eventually‑consistent view of the swarm’s state without central coordination. It uses Conflict‑Free Replicated Data Types (CRDTs) to guarantee merge convergence.

## Requirements

### Functional
1. **Key‑Value Store**
   - Insert, update, delete operations on arbitrary keys (string‑like).
   - Values can be primitive types (integers, floats, strings) or nested maps/lists.
   - Tombstone‑free garbage collection (e.g., using LSEQ trees).

2. **CRDT Semantics**
   - Operation‑based CRDTs (op‑based) for low overhead.
   - Support for `AWMap` (add‑win map) and `LSeq` (list with unique positions).
   - Merge of concurrent updates yields the same result on all replicas.

3. **Delta Synchronization**
   - Transmit only the changes (deltas) between peers.
   - Compress deltas when possible.
   - Support for causal ordering (vector clocks).

4. **Persistence**
   - Optional snapshotting to disk for crash recovery.
   - Incremental log of operations for audit.

5. **Integration with Transport**
   - Subscribe to specific keys or prefixes.
   - Automatically propagate deltas via Mesh Transport.
   - Handle network partitions gracefully.

### Non‑Functional
- **Merge latency**: < 10 ms for typical map sizes (< 1000 entries).
- **Memory overhead**: < 2× the size of the stored data.
- **Scalability**: Support at least 10 000 keys per agent.
- **Concurrency**: Allow concurrent reads and writes from multiple threads.

## Design

### Architecture
```
┌─────────────────────────────────────────┐
│            State Sync Core              │
├─────────────┬─────────────┬─────────────┤
│   CRDT      │   Delta     │  Persistence│
│   Engine    │   Manager   │   Layer     │
├─────────────┼─────────────┼─────────────┤
│           Conflict‑Free Merge           │
├─────────────────────────────────────────┤
│           Transport Adapter             │
└─────────────────────────────────────────┘
```

### Components

#### 1. CRDT Engine
- Implements the actual CRDT data structures.
- Provides `CrdtMap` and `CrdtSeq` abstractions.
- Exposes `apply_op(operation: Op)` and `merge(other: State)`.

#### 2. Delta Manager
- Tracks local changes since the last synchronization.
- Generates compact deltas for transmission.
- Applies incoming deltas to the local state.

#### 3. Persistence Layer
- Optional disk storage via `serde` and `bincode`.
- Snapshots at configurable intervals.
- Recovery from snapshot + operation log.

#### 4. Transport Adapter
- Listens for incoming delta messages from Mesh Transport.
- Sends deltas to peers that subscribe to relevant keys.
- Implements back‑pressure when network is saturated.

### Data Structures

```rust
pub type Key = String;
pub type Value = CrdtValue; // enum for supported types

pub enum CrdtValue {
    Integer(i64),
    Float(f64),
    Text(String),
    Map(CrdtMap),
    Seq(CrdtSeq),
    Boolean(bool),
    Bytes(Vec<u8>),
}

pub struct CrdtMap {
    inner: aw_map::AWMap<Key, Value>,
    vclock: VectorClock,
}

pub struct Op {
    pub id: OpId,
    pub key: Key,
    pub change: Change,
    pub causal_deps: Vec<OpId>,
}

pub enum Change {
    Set(Value),
    Delete,
    Increment(i64),
    // ... other operations
}

pub struct Delta {
    pub source: AgentId,
    pub ops: Vec<Op>,
    pub timestamp: Timestamp,
}
```

### Merge Algorithm
1. Each operation is assigned a unique ID (agent ID + logical timestamp).
2. Operations are stored in a causal‑order DAG.
3. When two states merge, the CRDT engine combines the DAGs and recomputes the resulting values according to the CRDT semantics (add‑wins, last‑write‑wins, etc.).
4. The merge is deterministic and commutative.

## Implementation Plan

### Phase 1 – Basic CRDT Map
1. Create `crates/state-sync` with `automerge` or `crdts` as dependency.
2. Implement a wrapper around `AWMap` from `crdts` crate.
3. Provide `get`, `set`, `delete` API.
4. Unit test merge of concurrent updates.

### Phase 2 – Delta Propagation
1. Integrate with Mesh Transport: send deltas as broadcast or targeted messages.
2. Implement subscription mechanism (keys of interest).
3. Add vector clocks for causal consistency.

### Phase 3 – Persistence & Recovery
1. Add snapshotting via `serde`.
2. Write operation log to disk (optional).
3. Benchmarks for merge performance.

### Phase 4 – Advanced Types
1. Support CRDT sequences (LSeq) for ordered lists.
2. Support counters (PN‑Counter).
3. Support registers (LWW‑Register).

## Dependencies
- `crdts` (or `automerge‑rs`) for CRDT algorithms
- `serde` for serialization
- `tokio` for async operations
- `tracing` for logging

## Testing Strategy
- **Property‑based tests**: Use `proptest` to verify CRDT invariants (commutativity, associativity, idempotence).
- **Network simulation**: Run multiple in‑memory agents that exchange deltas and verify eventual consistency.
- **Fault injection**: Simulate packet loss, duplication, and reordering.

## Open Questions
1. Should we use operation‑based or state‑based CRDTs? Op‑based reduces bandwidth but requires reliable broadcast.
2. How to handle garbage collection of old operations? Use dot‑kernel approach?
3. Should we support custom CRDT types defined by the user?

## References
- [CRDTs: The Hard Parts](https://www.youtube.com/watch?v=x7drE24geUw)
- [Automerge](https://automerge.org/)
- [crdts crate](https://crates.io/crates/crdts)
- [Vector Clocks Explained](https://www.cs.rutgers.edu/~pxk/417/notes/clocks/index.html)