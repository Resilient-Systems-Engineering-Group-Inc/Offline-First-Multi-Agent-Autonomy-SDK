# Integration Specification: Mesh Transport + State Sync

## Overview
This document describes how the Mesh Transport and State Sync (CRDT) components are combined to create a fully functional offline‑first multi‑agent state synchronization layer.

## Integration Architecture

### Data Flow
```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   State     │     │  Integration│     │   Mesh      │
│   Sync      │────▶│   Adapter   │────▶│  Transport  │
│   Core      │◀────│             │◀────│             │
└─────────────┘     └─────────────┘     └─────────────┘
       │                    │                    │
       ▼                    ▼                    ▼
  Local CRDT          Delta Encoding        Peer‑to‑Peer
   Updates                                 Broadcast
```

### Components

#### 1. Integration Adapter
- Sits between State Sync and Mesh Transport.
- Translates CRDT deltas into transport messages and vice‑versa.
- Manages subscriptions: which keys are forwarded to which peers.
- Implements back‑pressure and rate limiting.

#### 2. Subscription Manager
- Each agent can express interest in specific key patterns (e.g., `swarm/*`, `agent/42/*`).
- The adapter only sends deltas for keys that match a peer’s subscription.
- Supports wildcard and prefix‑based subscriptions.

#### 3. Delta Encoding / Decoding
- Deltas are serialized using a compact binary format (e.g., CBOR, MessagePack, or custom).
- Compression (zstd) can be applied for large deltas.
- Each delta includes:
  - Source agent ID
  - Vector clock fragment
  - List of operations
  - Timestamp (logical)

#### 4. Conflict Resolution
- When two deltas arrive concurrently, the CRDT engine merges them automatically.
- The integration layer ensures causal order: if delta B depends on delta A, B is not applied before A.
- Uses vector clocks attached to each delta.

## Protocol

### Message Types
```rust
pub enum SyncMessage {
    // Advertisement of subscription interests
    Subscribe { patterns: Vec<String> },
    Unsubscribe { patterns: Vec<String> },
    // Delta transmission
    Delta(Delta),
    // Request missing deltas (catch‑up)
    SyncRequest { since: VectorClock },
    SyncResponse { deltas: Vec<Delta> },
    // Heartbeat / keep‑alive
    Ping,
    Pong,
}
```

### Handshake
When two agents connect:
1. Exchange `Subscribe` messages to inform each other of key interests.
2. Optionally, perform a full sync if one agent is behind (via `SyncRequest`/`SyncResponse`).
3. Thereafter, only incremental deltas are sent.

### Reliability
- Deltas are sent over Mesh Transport’s reliable unicast channel.
- If a delta is lost, the receiver will detect a gap in the vector clock and request a retransmission.
- The integration adapter maintains a small buffer of recent deltas for retransmission.

## Configuration

### Tunable Parameters
- `max_delta_size`: Maximum size of a single delta before splitting (default 64 KB).
- `sync_interval`: How often to broadcast a summary of vector clocks (default 5 s).
- `subscription_timeout`: How long to keep a subscription without refresh (default 30 s).
- `compression_threshold`: Size above which to compress deltas (default 1 KB).

## Implementation Plan

### Phase 1 – Basic Integration
1. Create `crates/integration` (or extend `agent‑core`).
2. Implement `IntegrationAdapter` that holds handles to a `MeshTransport` and a `CrdtMap`.
3. Forward all local CRDT changes as broadcast deltas.
4. Apply incoming deltas directly to the CRDT map.
5. Write a test with two in‑process nodes that synchronize a simple key.

### Phase 2 – Subscriptions & Filtering
1. Add subscription mechanism.
2. Filter outgoing deltas based on peer subscriptions.
3. Add `Subscribe`/`Unsubscribe` message handling.

### Phase 3 – Causal Ordering & Catch‑Up
1. Attach vector clocks to each delta.
2. Implement `SyncRequest`/`SyncResponse` for late‑joining agents.
3. Buffer out‑of‑order deltas and apply them when dependencies are satisfied.

### Phase 4 – Optimization
1. Delta compression.
2. Batch multiple deltas into a single transport message.
3. Adaptive sync intervals based on network quality.

## Testing Strategy
- **Unit tests**: Mock transport and verify delta forwarding.
- **Integration tests**: Spawn multiple real transport instances over loopback and verify state convergence.
- **Property‑based tests**: Generate random sequences of CRDT operations, distribute across simulated agents, assert eventual consistency.
- **Network fault injection**: Drop, reorder, or duplicate messages and verify the system still converges.

## Open Questions
1. Should we support partial synchronization (only a subset of keys) to reduce bandwidth?
2. How to handle malicious peers sending malformed deltas? (Validation layer)
3. Should the integration layer be responsible for conflict resolution beyond CRDT merge? (e.g., application‑specific conflict handlers)

## References
- [Vector Clocks in Practice](https://www.cs.cornell.edu/~asampson/blog/vectorclocks.html)
- [Delta State Replicated Data Types](https://arxiv.org/abs/1603.01529)
- [CRDTs in Real‑World Applications](https://www.youtube.com/watch?v=xxjHC3yLDqw)