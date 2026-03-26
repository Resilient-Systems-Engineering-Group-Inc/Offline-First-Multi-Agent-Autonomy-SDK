# Architecture Overview

## Vision
Offline-First Multi-Agent Autonomy SDK enables a group of robots to operate collaboratively without a central server, using local state machines, conflict‑free replicated data types (CRDTs) for state synchronization, opportunistic mesh networking, and bounded consensus.

## Core Principles
1. **Offline‑first** – Every agent remains fully functional when disconnected.
2. **Decentralized** – No single point of failure; peer‑to‑peer communication.
3. **Eventually consistent** – State converges across the swarm via CRDTs.
4. **Resource‑aware** – Agents monitor and adapt to local constraints (CPU, battery, bandwidth).
5. **Pluggable transport** – Support for various network layers (Wi‑Fi, Bluetooth, LoRa, etc.).

## High‑Level Architecture

```mermaid
graph TB
    subgraph "Agent Node"
        AC[Agent Core]
        LP[Local Planner]
        SS[State Sync CRDT]
        MT[Mesh Transport]
        RM[Resource Monitor]

        AC --> LP
        AC --> SS
        AC --> MT
        AC --> RM
        SS --> MT
        MT --> SS
    end

    subgraph "Swarm"
        N1[Agent 1]
        N2[Agent 2]
        N3[Agent 3]
        N1 -- Mesh Network --> N2
        N2 -- Mesh Network --> N3
        N3 -- Mesh Network --> N1
    end

    subgraph "External"
        SIM[Simulation Gazebo/ROS2]
        CLI[Python CLI]
        SIM --> AC
        CLI --> AC
    end
```

## Component Responsibilities

### 1. Mesh Transport
- **Purpose**: Reliable, unordered, peer‑to‑peer message passing over ad‑hoc networks.
- **Features**:
  - Discovery (mDNS, manual peer list)
  - Connection management (TCP, WebRTC, QUIC)
  - Message routing (flooding, greedy perimeter)
  - Quality‑of‑Service (priority, retransmission)
- **Technology**: Rust crate built on `libp2p` or `smol‑net`.

### 2. State Sync (CRDT)
- **Purpose**: Maintain a shared, eventually‑consistent key‑value store across agents.
- **Features**:
  - CRDT‑based map (`aw‑map`, `lseq‑tree`)
  - Conflict‑free merge of concurrent updates
  - Tombstone‑free garbage collection
  - Version vectors / dotted version vectors
- **Technology**: Rust crate leveraging `automerge` or custom CRDT implementation.

### 3. Local Planner
- **Purpose**: Execute autonomous tasks based on local state and shared swarm intent.
- **Features**:
  - Finite‑state machine (FSM) definition and execution
  - Task scheduling and interruption
  - Integration with ROS2 navigation stack
- **Technology**: Rust crate with `behavior‑tree` or `smach`‑like DSL.

### 4. Resource Monitor
- **Purpose**: Observe local hardware constraints and adjust agent behavior.
- **Metrics**: CPU usage, battery level, network latency, memory pressure.
- **Actions**: Throttle planning frequency, reduce communication rate, switch to low‑power mode.

### 5. Agent Core
- **Purpose**: Glue component that orchestrates the above modules.
- **Lifecycle**: Initialization, event loop, graceful shutdown.
- **API**: Exposes a unified Rust trait and Python binding.

## Data Flow
1. Agent starts, joins mesh network via Transport.
2. Agent subscribes to shared CRDT keys (e.g., `swarm/goal`).
3. Local Planner reads local CRDT copy and decides next action.
4. Actions may update CRDT (e.g., `agent/status = moving`).
5. Transport propagates CRDT deltas to neighbors.
6. Resource Monitor may throttle outgoing messages if battery low.
7. On network partition, each agent continues with its last known state; merge occurs when connectivity resumes.

## Development Roadmap

### Phase 1 – Foundation (Current)
- Mesh Transport (basic peer discovery + messaging)
- State Sync (single‑type CRDT map)
- Integration test with two nodes

### Phase 2 – Autonomy
- Local Planner FSM
- Resource Monitor skeleton
- Python bindings for all components

### Phase 3 – Realism
- ROS2 integration
- Gazebo simulation with multiple robots
- Performance benchmarking

### Phase 4 – Production
- CI/CD, packaging (Debian, PyPI, crates.io)
- Comprehensive documentation
- Security audit

## Technology Stack
- **Language**: Rust (core), Python (bindings & high‑level API)
- **Networking**: `libp2p‑rust` or custom `smol‑net`
- **CRDT**: `automerge‑rs` or custom implementation
- **Simulation**: ROS2 Humble, Gazebo Classic / Ignition
- **Build**: Cargo workspace, `pyo3`, `maturin`
- **CI**: GitHub Actions, `cargo‑test`, `pytest`

## Directory Layout
See `README.md` for the exact folder structure.

## Contributing
Please read `CONTRIBUTING.md` (to be created) for guidelines on code style, testing, and pull requests.

---
*Last updated: 2026‑03‑26*