# Offline‑First Multi‑Agent Autonomy SDK

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Python](https://img.shields.io/badge/python-3.8%2B-blue.svg)](https://www.python.org)

A production‑grade SDK for building decentralized, offline‑first multi‑agent systems, designed for robot swarms, IoT networks, and other distributed autonomous systems.

## 🎯 Vision

Enable groups of agents (robots, drones, edge devices) to collaborate reliably without a central server, tolerating network partitions, intermittent connectivity, and dynamic topology changes.

## ✨ Key Features

- **Offline‑First Design**: Agents operate fully locally and synchronize state opportunistically.
- **Conflict‑Free Replicated Data Types (CRDTs)**: Automatically merge divergent state without conflicts.
- **Mesh Networking**: Peer‑to‑peer discovery and communication using libp2p (mDNS, TCP, WebSocket) with improved peer mapping and in‑memory backend for simulation.
- **Modular Architecture**: Pluggable components for local planning, resource monitoring, and transport.
- **Python Bindings**: Use the SDK from Python for rapid prototyping and integration with ROS2/Gazebo, with async support and peer listing.
- **Benchmarking & Testing**: Comprehensive test suites and performance benchmarks.
- **Bounded Consensus**: Two‑phase commit protocol for agreement within bounded rounds.
- **Delta Compression & Batching**: Optimized CRDT delta serialization with compression (Zlib) and deduplication.
- **Web Monitor**: Built‑in web interface for real‑time agent monitoring.
- **Distributed Task Planning**: Algorithms for coordinating tasks across agents (round‑robin, auction, resource‑aware).
- **Resource Monitoring & Alerting**: Collect system metrics (CPU, battery, memory) and trigger alerts based on thresholds.
- **State Migration**: Tools for upgrading CRDT schema versions without data loss.
- **Multiple Transport Backends**: Support for libp2p, in‑memory, WebRTC, and LoRa (stub) backends.
- **Swarm Simulation & Visualization**: Terminal‑based real‑time visualization of agent interactions.

## 🏗️ Architecture

```
Agent Core
├── Local Planner        – Decision‑making and task allocation
├── Distributed Planner  – Multi‑agent task coordination
├── State Sync (CRDT)    – Conflict‑free state synchronization
├── Mesh Transport       – Peer discovery and message routing (libp2p, WebRTC, LoRa)
├── Resource Monitor     – CPU, memory, battery, network monitoring with alerting
└── Bounded Consensus   – Agreement within bounded rounds
```

The SDK is organized as a Rust workspace with the following crates:

| Crate | Description |
|-------|-------------|
| `common` | Shared types, error handling, utilities (CBOR serialization). |
| `mesh‑transport` | Mesh networking with libp2p backend, discovery, connection management, in‑memory simulation, WebRTC and LoRa stubs. |
| `state‑sync` | CRDT‑based map, delta‑based synchronization, vector clocks, compression, batching, deduplication, state migration. |
| `agent‑core` | High‑level agent abstraction integrating transport and state sync. |
| `local‑planner` | Trait and implementations for autonomous decision‑making. |
| `distributed‑planner` | Distributed task planning algorithms (round‑robin, auction, resource‑aware, consensus). |
| `resource‑monitor` | System resource tracking (CPU, memory, battery, network) with alerting. |
| `bounded‑consensus` | Bounded‑round consensus protocol (two‑phase commit) for agreement. |
| `python/` | PyO3 bindings for Python integration with async support, covering all major components. |

## 🚀 Getting Started

### Prerequisites

- Rust (latest stable) – [install](https://rustup.rs/)
- Python 3.8+ (optional, for Python bindings)
- libp2p dependencies (automatically handled by Cargo)

### Building

```bash
git clone https://github.com/your-org/Offline-First-Multi-Agent-Autonomy-SDK.git
cd Offline-First-Multi-Agent-Autonomy-SDK
cargo build --release
```

### Running the Examples

**Simple synchronization demo** (two agents exchanging a counter):

```bash
cargo run --example simple_sync
```

**Extended multi‑agent demo** (three agents with in‑memory transport and simulated network):

```bash
cargo run --example multi_agent_demo
```

**Web monitor demo** (real‑time web interface for monitoring agents):

```bash
cargo run --example web_monitor
```
Then open http://127.0.0.1:3030 in your browser.

**Swarm simulation with real‑time visualization** (terminal‑based):

```bash
cargo run --example swarm_simulation
```

**ROS2/Gazebo simulation example** (dummy simulation):

```bash
cd examples/ros2_gazebo
python simple_robot.py
```

### In‑Memory Backend for Testing

The SDK includes an in‑memory transport backend that simulates network communication within a single process, ideal for unit tests and simulations. Enable it by setting `backend_type: BackendType::InMemory` in `MeshTransportConfig`.

Example:

```rust
let config = MeshTransportConfig::in_memory();
```

### Integration Tests

Run the integration tests for mesh‑transport and state‑sync:

```bash
cargo test -p mesh-transport --test integration
cargo test -p state-sync --test integration
```

### Bounded Consensus Component

A new crate `bounded-consensus` provides a protocol for reaching agreement within a bounded number of communication rounds. It is currently a placeholder for future implementation.

To use it, add `bounded-consensus` as a dependency and implement the `BoundedConsensus` trait.

### Using the Python Bindings

1. Install the Python package:

   ```bash
   cd python
   pip install -e .
   ```

2. Write a Python script:

   ```python
   from offline_first_autonomy import PyAgent, PyDistributedPlanner, PyResourceMonitor

   agent = PyAgent(42)
   agent.start()
   agent.set_value("counter", "123")

   planner = PyDistributedPlanner(1, [1, 2, 3])
   planner.start()
   planner.add_task("task1", "Move to point A", [], 10)

   monitor = PyResourceMonitor(1)
   cpu = monitor.cpu_usage()
   print(f"CPU usage: {cpu}%")
   ```

## 📖 Documentation

- [Architecture Overview](ARCHITECTURE.md) – detailed design decisions and component interactions.
- [API Reference](https://docs.rs) (coming soon)
- [Examples](./examples/) – practical usage examples.

## 🧪 Testing & Benchmarking

Run the unit and integration tests:

```bash
cargo test --workspace
```

Run the CRDT map benchmarks (requires criterion):

```bash
cargo bench -p state-sync
```

## 🧩 Extending the SDK

### Adding a New Transport Backend

Implement the `Backend` trait (see `crates/mesh‑transport/src/backend.rs`) and add a variant to `BackendType` in `transport.rs`.

### Implementing a Custom Local Planner

Implement the `LocalPlanner` trait (see `crates/local‑planner/src/lib.rs`) and integrate it with `Agent`.

### Adding New CRDT Types

Extend `state‑sync` with new CRDT structures that implement the `Crdt` trait.

### Adding a New Planning Algorithm

Implement the `PlanningAlgorithm` trait in `distributed‑planner/src/algorithms.rs` and register it with `DistributedPlanner`.

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgements

- The [libp2p](https://libp2p.io/) project for robust peer‑to‑peer networking.
- The [CRDTs](https://crdt.tech/) community for conflict‑free replication patterns.
- The [ROS](https://www.ros.org/) and [Gazebo](https://gazebosim.org/) communities for robotics simulation.

---

**Built with ❤️ for the future of autonomous systems.**
