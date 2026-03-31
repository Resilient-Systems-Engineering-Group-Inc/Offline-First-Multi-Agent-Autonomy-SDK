# Changelog

All notable changes to the Offline‑First Multi‑Agent Autonomy SDK will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Git configuration files (.gitattributes, .gitignore)
- GitHub Actions workflows for CI/CD (ci.yml, cd.yml, security.yml, benchmarks.yml, coverage.yml, publish.yml, docs.yml)
- GitLab CI configuration (.gitlab-ci.yml)
- VERSION file with current version
- CONTRIBUTING.md guide for contributors

### Changed
- Updated CHANGELOG.md to reflect recent developments
- Improved CI/CD pipeline with multiple stages

## [0.3.0] - 2026‑03‑31

### Added
- **Fault tolerance and self‑healing**: `fault_tolerance` module in `agent‑core` with failure detection and task reallocation.
- **Heterogeneous agent support**: `Capability` types, `AgentCapabilities`, enhanced `ResourceMetrics`, `Task` with capability requirements.
- **Enhanced security**: Extended `security.rs` in `mesh‑transport` with signing, verification, encryption, Diffie‑Hellman key exchange, hybrid encryption, and access control lists.
- **Profiling tools**: New `profiling` crate with metrics collection, distributed tracing, snapshot capture, and debug HTTP server.
- **Cloud integration**: New `cloud‑integration` crate with adapters for AWS IoT, Azure IoT, and MQTT.
- **Dynamic consensus membership**: Enhanced `bounded‑consensus` with epochs and `update_participants` method.
- **Advanced task planner**: Extended `Task` and `Assignment` structures with deadlines, priorities, dependencies; added `DeadlineAwarePlanner` and `DependencyAwarePlanner`.
- **Swarm simulator**: New `swarm‑simulator` crate for configurable agent simulation with network models, delays, and failures.
- **Enhanced monitoring**: Added Prometheus metrics for tasks, resources, and health checks in `common/metrics.rs`.
- **State migration improvements**: Enhanced `migration.rs` with key‑rename and value‑transform migrations, versioned schemas.
- **Distributed key‑value store**: New `distributed‑kv` crate with CRDT‑based map, replication, persistence, indexing, and querying.
- **IoT interfaces**: New `iot‑interface` crate with `Sensor` and `Actuator` abstractions, drivers for MQTT, CoAP, Modbus, ROS2, and device registry.
- **Reinforcement learning planner**: New `rl‑planner` crate with `PlanningEnvironment`, `Policy`, `RlPlanner`, and episodic training.
- **Blockchain proof‑of‑stake consensus**: New `blockchain‑consensus` crate with block/transaction structures, chain validation, and stake‑based validator selection.
- **Kubernetes operator**: New `k8s‑operator` crate with CRDs for Agent and Task, controller, reconciliation, and deployment manifests.
- **Configuration management**: New `configuration` crate with centralized config, hot‑reload, validation, and schema support.
- **Streaming data**: New `streaming` crate with publish‑subscribe channels, QoS levels, and compression.
- **Web dashboard**: New `dashboard` crate (Yew/WASM) with real‑time monitoring, WebSocket connections, and data models.
- **Edge computing**: New `edge‑computing` crate with hardware detection, optimization for Raspberry Pi/Jetson, and energy‑aware scheduling.
- **Audit system**: New `audit` crate with structured events, backends (file, Elasticsearch, Loki), and filtering.
- **OTA updates**: New `ota‑updates` crate with delta updates, signature verification, and package management.
- **Federated learning**: New `federated‑learning` crate with aggregation, differential privacy, and homomorphic encryption.
- **Digital twin**: New `digital‑twin` crate with physics engine integration, visualization, and IoT synchronization.
- **Quantum computing**: New `quantum‑computing` crate with interfaces to quantum simulators and optimization algorithms.
- **Power management**: New `power‑management` crate with battery monitoring, CPU frequency scaling, and energy‑efficient planning.
- **Comprehensive demo**: `examples/comprehensive_demo.rs` showcasing integration of multiple components.

### Changed
- Updated all crate versions to 0.3.0.
- Improved documentation across all modules.
- Enhanced error handling with more specific error types.
- Optimized performance of CRDT map merging and delta generation.
- Refactored mesh transport to support multiple backends (libp2p, WebRTC, LoRa) more cleanly.
- Upgraded dependencies to latest compatible versions.

### Fixed
- Python bindings for `send_to` and `broadcast` no longer consume the transport.
- `MeshTransport::start` method now works without consuming the transport.
- Libp2p backend discovery improved for cross‑platform compatibility.
- Various memory leaks in state synchronization.
- Race conditions in task assignment.

### Known Issues
- LoRa backend requires specific hardware and may not work in simulation.
- Quantum computing interfaces are experimental and depend on external simulators.
- Kubernetes operator requires cluster‑level permissions.

## [0.2.0] - 2026‑03‑26

### Added
- Web‑based monitoring dashboard (`examples/web_monitor.rs`).
- Bounded consensus algorithm (two‑phase commit with timeouts).
- Delta compression (Zlib) and deduplication in `state‑sync`.
- Python bindings for `peers()` method returning live peer list.
- Integration of delta compression into mesh‑transport.

### Changed
- Improved libp2p backend: peer‑id to agent‑id mapping, synchronous `peers()`.
- Updated all crate versions to 0.2.0.
- Enhanced documentation with new features and examples.

### Fixed
- Python `peers()` method now correctly returns list of connected peers.
- Delta batching and deduplication logic.

## [0.1.0] - 2026‑03‑26

### Added
- Initial project skeleton with Rust workspace and six core crates:
  - `common`: shared types, error handling, serialization utilities.
  - `mesh‑transport`: mesh networking with libp2p backend, discovery, connection management.
  - `state‑sync`: CRDT‑based map with operation‑based deltas and vector clocks.
  - `agent‑core`: high‑level agent abstraction integrating transport and state sync.
  - `local‑planner`: trait for autonomous decision‑making.
  - `resource‑monitor`: trait for system resource tracking.
- Python bindings via PyO3, providing `PyAgent` and `PyMeshTransport` classes.
- Comprehensive documentation: architecture specs, API overview, getting started guide.
- Example applications:
  - `simple_sync`: two agents synchronizing a counter.
  - `ros2_gazebo`: placeholder for ROS2/Gazebo simulation.
- CI/CD pipeline (GitHub Actions) for Rust and Python.
- Benchmark suite for CRDT map performance (using Criterion).

### Changed
- Refactored mesh transport to resolve mutability mismatch between `Backend` and `Transport` traits.
- Improved error handling with dedicated error types and logging.
- Enhanced CRDT map to generate deltas based on vector clocks, reducing bandwidth.

### Fixed
- Various compilation warnings and clippy lints.
- Inconsistent trait bounds in `Backend` and `Transport`.

### Known Issues
- Libp2p backend discovery may not work out‑of‑the‑box on all platforms.
- Python bindings for `send_to` and `broadcast` currently consume the transport (bug).
- The `MeshTransport::start` method consumes the transport (bug).
- No real‑world multi‑agent simulation yet (only simple examples).

## [0.1.0] - 2026‑03‑26

### Added
- First alpha release of the SDK.
- Basic functionality for offline‑first multi‑agent systems.
- All core components are present and can be integrated.

### Notes
This release is intended for early adopters and developers who want to experiment with the SDK. It is not yet production‑ready, but provides a solid foundation for building decentralized, offline‑first autonomous systems.
