# Mesh Transport Specification

## Overview
Mesh Transport is a peer‑to‑peer networking layer that enables agents to discover each other and exchange messages in an ad‑hoc, possibly partitioned, network.

## Requirements

### Functional
1. **Discovery**
   - Automatic discovery of peers on the same local network (via mDNS, UDP broadcast, or manual list).
   - Support for static configuration of known peer addresses.
   - Ability to filter peers by agent ID or role.

2. **Connection Management**
   - Establish bidirectional connections (TCP, WebRTC, QUIC) with fallback.
   - Handle connection loss and automatic reconnection with exponential backoff.
   - Keep‑alive heartbeats to detect dead peers.

3. **Message Routing**
   - Unreliable broadcast (flooding) for small‑swarm scenarios.
   - Reliable unicast for point‑to‑point commands.
   - Optional multicast groups for topic‑based messaging.

4. **Quality of Service**
   - Priority queues for critical messages (e.g., emergency stop).
   - Configurable retransmission for reliable delivery.
   - Bandwidth throttling per peer.

5. **Security**
   - TLS‑like encryption (Noise protocol) for all links.
   - Peer authentication via pre‑shared keys or certificate authority.
   - Message integrity and replay protection.

### Non‑Functional
- **Latency**: < 100 ms for local‑network messages.
- **Throughput**: Support at least 1000 messages/second per agent.
- **Scalability**: Up to 50 agents in a single mesh.
- **Resource usage**: < 5 MB RAM and < 1% CPU when idle.

## Design

### Architecture
```
┌─────────────────────────────────────────┐
│            Mesh Transport               │
├─────────────┬─────────────┬─────────────┤
│  Discovery  │ Connection  │   Routing   │
│    Module   │   Manager   │   Module    │
├─────────────┼─────────────┼─────────────┤
│           Security Layer                │
├─────────────────────────────────────────┤
│           Transport Backend             │
│           (libp2p, smol‑net)            │
└─────────────────────────────────────────┘
```

### Components

#### 1. Discovery Module
- Implements `Discovery` trait.
- Provides `discover_peers() -> Vec<PeerInfo>`.
- Emits `PeerDiscovered` and `PeerLost` events.

#### 2. Connection Manager
- Maintains a `HashMap<PeerId, Connection>`.
- Creates outgoing connections and accepts incoming ones.
- Monitors health and triggers reconnection.

#### 3. Routing Module
- Implements `RouteMessage` trait.
- `broadcast(data: Vec<u8>)` floods to all connected peers.
- `send_to(peer: PeerId, data: Vec<u8>)` delivers to a specific peer.

#### 4. Security Layer
- Wraps each connection with an authenticated encrypted channel.
- Uses Noise protocol framework (XX handshake).

#### 5. Transport Backend
- Abstract trait `TransportBackend` allowing pluggable implementations.
- Default backend: `Libp2pBackend` (using `libp2p` crate).
- Alternative backend: `SmolNetBackend` (custom lightweight TCP/UDP).

### Data Structures

```rust
pub struct PeerId([u8; 32]); // Cryptographic hash of public key

pub struct PeerInfo {
    pub id: PeerId,
    pub addresses: Vec<SocketAddr>,
    pub metadata: HashMap<String, String>,
}

pub enum TransportEvent {
    PeerDiscovered(PeerInfo),
    PeerLost(PeerId),
    MessageReceived {
        from: PeerId,
        payload: Vec<u8>,
        timestamp: Instant,
    },
    ConnectionEstablished(PeerId),
    ConnectionClosed(PeerId),
}

pub trait MeshTransport {
    fn broadcast(&self, payload: Vec<u8>) -> Result<()>;
    fn send_to(&self, peer: PeerId, payload: Vec<u8>) -> Result<()>;
    fn events(&self) -> Box<dyn Stream<Item = TransportEvent> + Send>;
    fn peers(&self) -> Vec<PeerInfo>;
}
```

## Implementation Plan

### Phase 1 – Minimal Viable Transport
1. Create `crates/mesh-transport` with `libp2p` as a dependency.
2. Implement a simple discovery via mDNS (libp2p‑mdns).
3. Establish TCP connections between discovered peers.
4. Send plain‑text “ping‑pong” messages.
5. Unit test with two in‑process nodes.

### Phase 2 – Reliability & Security
1. Add Noise protocol encryption.
2. Implement reliable message delivery with sequence numbers and ACKs.
3. Add connection heartbeat and reconnection logic.
4. Benchmark latency and throughput.

### Phase 3 – Advanced Features
1. Support for UDP (QUIC) for lower latency.
2. Multicast groups for topic‑based subscriptions.
3. Integration with resource monitor for adaptive QoS.

## Dependencies
- `libp2p` (with features `tcp`, `mdns`, `noise`, `yamux`)
- `tokio` for async runtime
- `serde` for configuration serialization
- `tracing` for structured logging

## Testing Strategy
- **Unit tests**: Mock network interfaces with `libp2p‑swarm‑test`.
- **Integration tests**: Spawn multiple OS‑level processes that communicate via loopback.
- **Simulation tests**: Use `tokio‑test` and virtual time to simulate network partitions.

## Open Questions
1. Should we support WebRTC for browser‑based agents?
2. Is mDNS sufficient for discovery, or do we need a custom beacon protocol?
3. How to handle NAT traversal (STUN/TURN)?

## References
- [libp2p Specification](https://github.com/libp2p/specs)
- [Noise Protocol Framework](http://noiseprotocol.org/)
- [RFC 6762 – mDNS](https://tools.ietf.org/html/rfc6762)