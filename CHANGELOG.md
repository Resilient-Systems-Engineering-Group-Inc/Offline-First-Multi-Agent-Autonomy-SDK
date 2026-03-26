# Changelog

All notable changes to the Offline‑First Multi‑Agent Autonomy SDK will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
