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
- **Mesh Networking**: Peer‑to‑peer discovery and communication using libp2p (mDNS, TCP, WebSocket).
- **Modular Architecture**: Pluggable components for local planning, resource monitoring, and transport.
- **Python Bindings**: Use the SDK from Python for rapid prototyping and integration with ROS2/Gazebo.
- **Benchmarking & Testing**: Comprehensive test suites and performance benchmarks.

## 🏗️ Architecture

```
Agent Core
├── Local Planner        – Decision‑making and task allocation
├── State Sync (CRDT)    – Conflict‑free state synchronization
├── Mesh Transport       – Peer discovery and message routing
└── Resource Monitor     – CPU, memory, battery, network monitoring
```

The SDK is organized as a Rust workspace with the following crates:

| Crate | Description |
|-------|-------------|
| `common` | Shared types, error handling, utilities (CBOR serialization). |
| `mesh‑transport` | Mesh networking with libp2p backend, discovery, connection management. |
| `state‑sync` | CRDT‑based map, delta‑based synchronization, vector clocks. |
| `agent‑core` | High‑level agent abstraction integrating transport and state sync. |
| `local‑planner` | Trait and implementations for autonomous decision‑making. |
| `resource‑monitor` | System resource tracking (CPU, memory, battery, network). |
| `python/` | PyO3 bindings for Python integration. |

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

**ROS2/Gazebo simulation example** (dummy simulation):

```bash
cd examples/ros2_gazebo
python simple_robot.py
```

### Using the Python Bindings

1. Install the Python package:

   ```bash
   cd python
   pip install -e .
   ```

2. Write a Python script:

   ```python
   from offline_first_autonomy import PyAgent

   agent = PyAgent(42)
   agent.start()
   agent.set_value("counter", "123")
   # ...
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

Implement the `Transport` trait (see `crates/mesh‑transport/src/transport.rs`) and plug it into `MeshTransport`.

### Implementing a Custom Local Planner

Implement the `LocalPlanner` trait (see `crates/local‑planner/src/lib.rs`) and integrate it with `Agent`.

### Adding New CRDT Types

Extend `state‑sync` with new CRDT structures that implement the `Crdt` trait.

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
