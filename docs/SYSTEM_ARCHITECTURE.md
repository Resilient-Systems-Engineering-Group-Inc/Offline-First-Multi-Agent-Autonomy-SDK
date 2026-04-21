# System Architecture Documentation

## Overview

This document provides a comprehensive overview of the Offline-First Multi-Agent Autonomy SDK architecture, component interactions, and design decisions.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Agent Node                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Mesh       │  │   State      │  │   Security   │          │
│  │  Transport   │◄─┤   Sync       │◄─┤   Manager    │          │
│  │  (libp2p)    │  │   (CRDT)     │  │              │          │
│  └──────┬───────┘  └──────┬───────┘  └──────────────┘          │
│         │                 │                                      │
│         ▼                 ▼                                      │
│  ┌──────────────────────────────────────┐                       │
│  │        Agent Core / Orchestrator     │                       │
│  └──────────────┬───────────────────────┘                       │
│                 │                                                │
│         ┌───────┴────────┐                                      │
│         ▼                ▼                                      │
│  ┌──────────────┐  ┌──────────────┐                             │
│  │   Distributed│  │   Local      │                             │
│  │   Planner    │  │   Planner    │                             │
│  │              │  │   (FSM)      │                             │
│  └──────┬───────┘  └──────────────┘                             │
│         │                                                        │
│         ▼                                                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Workflow    │  │  Resource    │  │  Metrics &   │          │
│  │  Engine      │  │  Monitor     │  │  Observability│         │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
         ▲                         ▲
         │                         │
         └─────────────────────────┘
              Mesh Network (P2P)

┌─────────────────────────────────────────────────────────────────┐
│                    External Integrations                         │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │    ROS2      │  │  Python      │  │  Dashboard   │          │
│  │  Adapter     │  │  Bindings    │  │  (Web UI)    │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Prometheus  │  │  Gazebo      │  │  Kubernetes  │          │
│  │  Monitoring  │  │  Simulation  │  │  Operator    │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Mesh Transport Layer

**Purpose:** Reliable peer-to-peer communication over ad-hoc networks.

**Key Components:**
- `libp2p_backend.rs`: Production-grade P2P networking
- `in_memory_backend.rs`: Testing backend
- `webrtc_backend.rs`: WebRTC support for browser integration
- `lora_backend.rs`: LoRa/WAN support for long-range communication

**Features:**
- Automatic peer discovery (mDNS, manual)
- Connection management (TCP, WebSocket, WebRTC)
- Message routing (flooding, greedy perimeter)
- QoS prioritization
- End-to-end encryption

**Interfaces:**
```rust
pub trait Transport: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn send(&self, destination: &AgentId, message: Message) -> Result<()>;
    async fn broadcast(&self, message: Message) -> Result<()>;
    fn subscribe(&self) -> Receiver<Message>;
    fn connected_peers(&self) -> Vec<AgentId>;
}
```

### 2. State Sync (CRDT)

**Purpose:** Eventually consistent state synchronization across agents.

**Key Components:**
- `crdt_map.rs`: G-Counter, LWW-Register based map
- `delta.rs`: Delta propagation for efficient sync
- `sync.rs`: Synchronization protocol implementation
- `compression.rs`: Delta compression

**Data Structures:**
```
CrdtMap {
    entries: HashMap<Key, CrdtValue>,
    version_vectors: VersionVector,
    tombstones: HashSet<Key>, // For garbage collection
}

CrdtValue {
    value: Bytes,
    timestamp: u64,
    node_id: NodeId,
    vector_clock: VectorClock,
}
```

**Properties:**
- Commutative: a.merge(b) == b.merge(a)
- Associative: (a.merge(b)).merge(c) == a.merge(b.merge(c))
- Idempotent: a.merge(a) == a

### 3. Distributed Planner

**Purpose:** Collaborative task assignment and coordination.

**Architecture:**
```
┌─────────────────────────────────────────┐
│         Planning Algorithm              │
│  (RoundRobin, Auction, Multi-Objective) │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│      Task Assignment Generator          │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│      Bounded Consensus (Paxos/2PC)      │
│  - Propose assignment                   │
│  - Vote collection                      │
│  - Decision propagation                 │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│      Task Lifecycle Manager             │
│  - State machine                        │
│  - Retry logic                          │
│  - Dependency resolution                │
└─────────────────────────────────────────┘
```

**Planning Algorithms:**
1. **RoundRobin:** Simple, O(n)
2. **Auction:** Cost-based bidding, O(n*m)
3. **ResourceAware:** Constraints-based, O(n*m)
4. **MultiObjective:** Weighted scoring, O(n*m*k)
5. **RL-based:** Learning-based, O(n*m)

### 4. Security Manager

**Purpose:** Cryptographic security for all communications.

**Components:**
- Classical crypto: Ed25519, X25519, ChaCha20-Poly1305
- Post-quantum: Kyber, Dilithium, Falcon
- Hybrid mode: Classical + PQ for transition

**Security Model:**
```
┌─────────────────────────────────────┐
│         Authentication              │
│  - Ed25519 signatures               │
│  - Agent identity verification      │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Key Exchange                │
│  - X25519 (classical)               │
│  - Kyber (post-quantum)             │
│  - Hybrid (both)                    │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│         Encryption                  │
│  - ChaCha20-Poly1305                │
│  - AES-GCM                          │
└─────────────────────────────────────┘
```

### 5. Workflow Orchestration

**Purpose:** Complex task coordination and automation.

**Workflow Definition (YAML):**
```yaml
workflow_id: exploration_task
name: Area Exploration
description: Collaborative mapping of unknown area

tasks:
  - id: initialize
    type: setup
    timeout: 30s
    retries: 3
    
  - id: explore_zone_a
    type: action
    dependencies: [initialize]
    capabilities: [navigation, lidar]
    timeout: 120s
    
  - id: explore_zone_b
    type: action
    dependencies: [initialize]
    capabilities: [navigation, lidar]
    timeout: 120s
    
  - id: merge_maps
    type: action
    dependencies: [explore_zone_a, explore_zone_b]
    timeout: 60s
    
  - id: cleanup
    type: teardown
    dependencies: [merge_maps]
    timeout: 30s

on_failure: rollback
```

### 6. Resource Monitor

**Purpose:** System health monitoring and adaptation.

**Monitored Metrics:**
- CPU usage (%)
- Memory usage (%)
- Disk usage (%)
- Battery level (%)
- Network latency (ms)
- Connected peers (count)

**Actions:**
- Throttle planning frequency when CPU > 80%
- Reduce communication rate when battery < 20%
- Switch to low-power mode when battery < 10%
- Trigger alerts on critical thresholds

### 7. Dashboard & Observability

**Purpose:** Real-time monitoring and visualization.

**Components:**
- **Backend (Rust):**
  - Prometheus metrics exporter
  - WebSocket for live updates
  - REST API for queries
  
- **Frontend (Yew/Rust):**
  - Agent topology visualization
  - Task progress tracking
  - Resource utilization charts
  - Network mesh graph

**Metrics:**
```rust
metrics! {
    counter: sdk_tasks_completed_total,
    counter: sdk_tasks_failed_total,
    gauge: sdk_active_agents,
    gauge: sdk_connected_peers,
    histogram: sdk_message_latency_ms,
    histogram: sdk_consensus_round_time_ms,
    gauge: sdk_agent_battery_level,
    counter: sdk_messages_sent_total,
    counter: sdk_messages_received_total,
}
```

## Data Flow Examples

### Example 1: Task Assignment Flow

```
Agent 1                    Agent 2                    Agent 3
   │                          │                          │
   │── Discover peers ───────►│                          │
   │◄────── mDNS ─────────────│                          │
   │                          │                          │
   │── Create task ──────────────────────────────────────│
   │                          │                          │
   │── Publish to CRDT ─────►│◄────── Sync ─────────────│
   │                          │                          │
   │── Run planning algo ────│                          │
   │◄────── Assignments ─────│                          │
   │                          │                          │
   │── Assign task ─────────►│                          │
   │                          │                          │
   │                          │── Execute task ─────────►│
   │                          │                          │
   │◄────── Complete ─────────│                          │
   │                          │                          │
```

### Example 2: Network Partition Recovery

```
Before Partition          During Partition         After Reconnection

Agent 1 ───── Agent 2     Agent 1    Agent 2      Agent 1 ───── Agent 2
    │            │             │          │            │            │
    │            │             │          │            │            │
 Update CRDT    │          Update CRDT  │         Merge CRDTs    │
    │            │             │      Update CRDT    │            │
    │            │             │          │         Converged    │
    │            │             │          │            │            │
    │  [PARTITION]            │          │            │            │
    │            │             │          │    [RECONNECT]         │
    │            │             │          │            │            │
    │            │             │◄────Sync─────────────│            │
    │            │             │          │            │            │
    │            │             │◄──Delta──│            │            │
    │            │             │          │            │            │
    │            │          Converged     │         Converged      │
```

## Performance Characteristics

### Latency

| Operation | Local Network | WAN | High Latency |
|-----------|--------------|-----|--------------|
| Message send | <5ms | <50ms | <200ms |
| CRDT merge | <1ms | <10ms | <50ms |
| Consensus (5 nodes) | <20ms | <100ms | <500ms |
| Task planning (100 tasks) | <50ms | <100ms | <200ms |

### Scalability

| Metric | 10 Agents | 50 Agents | 100 Agents |
|--------|-----------|-----------|------------|
| Message overhead | Low | Medium | High |
| Consensus time | 15ms | 45ms | 100ms |
| CRDT sync time | 5ms | 25ms | 60ms |
| Memory per agent | 50MB | 75MB | 100MB |

## Deployment Models

### Model 1: Standalone Agents
```
Agent 1 ── Mesh ── Agent 2 ── Mesh ── Agent 3
```

### Model 2: ROS2 Integration
```
ROS2 Master
    │
    ├── Robot 1 (Agent + ROS2 Node)
    ├── Robot 2 (Agent + ROS2 Node)
    └── Robot 3 (Agent + ROS2 Node)
```

### Model 3: Kubernetes Deployment
```
K8s Cluster
    │
    ├── Namespace: agents
    │   ├── Agent Pod 1
    │   ├── Agent Pod 2
    │   └── Agent Pod 3
    │
    ├── Namespace: monitoring
    │   ├── Prometheus
    │   └── Grafana
    │
    └── Namespace: dashboard
        └── Dashboard Pod
```

## Security Considerations

### Threat Model

1. **Eavesdropping:** All messages encrypted end-to-end
2. **Man-in-the-middle:** Agent authentication + signature verification
3. **Spoofing:** Cryptographic identity verification
4. **Replay attacks:** Timestamp validation + nonce
5. **Denial of service:** Rate limiting + resource quotas

### Security Features

- ✅ End-to-end encryption
- ✅ Agent authentication
- ✅ Message integrity
- ✅ Post-quantum ready
- ✅ Secure key management
- ✅ Audit logging

## Future Enhancements

1. **Machine Learning Integration**
   - Predictive task allocation
   - Anomaly detection
   - Adaptive resource management

2. **Advanced Consensus**
   - Byzantine fault tolerance
   - Sharded consensus
   - Asynchronous consensus

3. **Edge Computing**
   - Edge-offload for heavy computation
   - Hierarchical agent structures
   - Edge-cloud synchronization

4. **Quantum Networks**
   - Quantum key distribution
   - Quantum-secure communications

## References

- [CRDT Papers](https://crdt.tech/papers.html)
- [libp2p Documentation](https://docs.libp2p.io/)
- [ROS2 Architecture](https://docs.ros.org/en/humble/Concepts/About-Ros2-Architecture.html)
- [Post-Quantum Cryptography](https://pqcrypto.info/)
