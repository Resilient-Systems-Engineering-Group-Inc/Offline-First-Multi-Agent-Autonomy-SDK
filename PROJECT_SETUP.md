# Project Setup Instructions

This document describes the steps to create the monorepo structure for the Offline‑First Multi‑Agent Autonomy SDK.

## 1. Workspace Root

Create a `Cargo.toml` with the following content:

```toml
[workspace]
members = [
    "crates/mesh-transport",
    "crates/state-sync",
    "crates/agent-core",
    "crates/common",
]
resolver = "2"

[workspace.dependencies]
tokio = { version = "1.38", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
thiserror = "2.0"
tracing = "0.1"
```

## 2. Create Crate Directories

Run these commands in the terminal:

```bash
mkdir -p crates/{mesh-transport,state-sync,agent-core,common}
```

For each crate, create a minimal `Cargo.toml` and `src/lib.rs`.

### Example: `crates/mesh-transport/Cargo.toml`

```toml
[package]
name = "mesh-transport"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
anyhow = { workspace = true }
tracing = { workspace = true }
libp2p = { version = "0.54", features = ["tcp", "dns", "noise", "yamux"] }
```

### Example: `crates/mesh-transport/src/lib.rs`

```rust
pub mod discovery;
pub mod connection;
pub mod message;

pub use discovery::Discovery;
pub use connection::ConnectionManager;
```

## 3. Python Bindings Directory

Create `python/` with `pyproject.toml` and `src/lib.rs` for PyO3.

```bash
mkdir -p python/src
```

`python/pyproject.toml`:

```toml
[build-system]
requires = ["maturin>=1.0,<2.0"]
build-backend = "maturin"

[project]
name = "offline_first_autonomy"
version = "0.1.0"
description = "Python bindings for Offline‑First Multi‑Agent Autonomy SDK"
requires-python = ">=3.8"
dependencies = []

[tool.maturin]
module-name = "offline_first_autonomy"
bindings = "pyo3"
```

`python/src/lib.rs` will be filled later.

## 4. Examples and Simulation

```bash
mkdir -p examples/simple-sync
mkdir -p simulation/gazebo
mkdir -p simulation/ros2
```

## 5. Documentation

```bash
mkdir -p docs
touch docs/overview.md
```

## 6. CI/CD

Create `.github/workflows/ci.yml` with Rust and Python testing.

## Next Steps

After the skeleton is created, switch to **Code Mode** to implement the actual Rust crates.