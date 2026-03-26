# Python Bindings Specification

## Overview
Python bindings expose the core Rust functionality (Mesh Transport, State Sync, Integration) as a high‑level Python API, enabling rapid prototyping, scripting, and integration with existing Python‑based robotics stacks (ROS2, OpenCV, etc.).

## Requirements

### Functional
1. **Full Feature Parity**
   - All major Rust structs and functions accessible from Python.
   - Asynchronous support compatible with `asyncio`.
   - Error translation (Rust `Result` → Python exceptions).

2. **Ease of Use**
   - Idiomatic Python naming (snake_case, context managers, etc.).
   - Comprehensive docstrings with examples.
   - Type hints (PEP 484) for better IDE support.

3. **Performance**
   - Minimal overhead for crossing the language boundary.
   - Zero‑copy data sharing where possible (e.g., bytes, arrays).
   - Ability to run callbacks from Rust on Python objects.

4. **Distribution**
   - Package on PyPI as `offline-first-autonomy`.
   - Support Linux, macOS, Windows (with Rust toolchain).
   - Provide pre‑built wheels for common platforms.

### Non‑Functional
- **Startup time**: < 100 ms for importing the module.
- **Memory overhead**: < 5 MB per imported module.
- **Thread safety**: Safe to use from multiple Python threads.

## Design

### Technology Stack
- **Binding framework**: PyO3 (with `maturin` for building).
- **Async runtime**: `pyo3‑asyncio` to bridge Tokio and `asyncio`.
- **Serialization**: `serde` + `pyo3‑serde` for automatic conversion of complex types.

### Module Structure
```
offline_first_autonomy/
├── __init__.py
├── transport.py          # Mesh Transport
├── state_sync.py         # CRDT map & sequences
├── integration.py        # Integrated adapter
├── agent.py              # High‑level Agent class
├── exceptions.py         # Custom exceptions
└── utils.py              # Helper functions
```

### Key Classes

#### `MeshTransport`
```python
class MeshTransport:
    def __init__(self, config: TransportConfig) -> None: ...
    async def start(self) -> None: ...
    async def stop(self) -> None: ...
    async def broadcast(self, data: bytes) -> None: ...
    async def send_to(self, peer_id: PeerId, data: bytes) -> None: ...
    def peers(self) -> List[PeerInfo]: ...
    async def events(self) -> AsyncIterator[TransportEvent]: ...
```

#### `CrdtMap`
```python
class CrdtMap:
    def __init__(self) -> None: ...
    def get(self, key: str) -> Optional[Any]: ...
    def set(self, key: str, value: Any) -> None: ...
    def delete(self, key: str) -> None: ...
    def merge(self, other: CrdtMap) -> None: ...
    def to_dict(self) -> Dict[str, Any]: ...
```

#### `SyncAgent`
```python
class SyncAgent:
    def __init__(self, transport: MeshTransport, crdt_map: CrdtMap) -> None: ...
    async def sync(self) -> None: ...
    def subscribe(self, pattern: str) -> None: ...
    async def wait_for_key(self, key: str, timeout: float = None) -> Any: ...
```

### Data Conversion
- Rust `Vec<u8>` ↔ Python `bytes` (zero‑copy via `PyBytes`).
- Rust `String` ↔ Python `str`.
- Rust `HashMap` ↔ Python `dict`.
- Rust `enum` ↔ Python `Enum` or discriminated unions.
- Custom types (e.g., `PeerId`) become opaque Python objects with `__repr__` and `__str__`.

### Asynchronous Pattern
- Rust async functions are exposed as Python coroutines.
- Use `pyo3‑asyncio` to run Tokio futures on the same event loop as `asyncio`.
- Example:
```python
import asyncio
from offline_first_autonomy import MeshTransport

async def main():
    transport = MeshTransport(config)
    await transport.start()
    await transport.broadcast(b"hello")
```

### Error Handling
- Rust `anyhow::Error` → Python `RuntimeError` with a descriptive message.
- Custom error types (e.g., `TransportError`, `MergeConflictError`) become dedicated Python exception classes.

## Implementation Plan

### Phase 1 – PyO3 Setup
1. Create `python/` directory with `pyproject.toml` and `Cargo.toml` (as a PyO3 crate).
2. Define a minimal Rust module that exports a “hello world” function.
3. Build with `maturin develop` and verify import works.

### Phase 2 – Mesh Transport Bindings
1. Wrap the Rust `MeshTransport` struct with PyO3.
2. Expose essential methods (`start`, `stop`, `broadcast`).
3. Implement conversion of `TransportEvent` to Python objects.
4. Write Python unit tests using `pytest`.

### Phase 3 – State Sync Bindings
1. Wrap `CrdtMap` and `CrdtSeq`.
2. Provide Pythonic `__getitem__`/`__setitem__` for map‑like access.
3. Implement `merge` and `to_dict`.

### Phase 4 – Integration & High‑Level API
1. Create `SyncAgent` class that combines transport and CRDT.
2. Add subscription and delta‑streaming callbacks.
3. Provide example scripts.

### Phase 5 – Packaging & Distribution
1. Configure `maturin` to produce wheels.
2. Add GitHub Actions workflow to build and upload to PyPI on tags.
3. Write installation instructions.

## Dependencies
- `pyo3 = "0.21"`
- `pyo3‑asyncio = { version = "0.21", features = ["tokio‑runtime"] }`
- `maturin` (build‑time)
- `pytest` (dev)

## Testing Strategy
- **Unit tests**: Rust‑side tests for PyO3 bindings (using `pyo3‑test`).
- **Integration tests**: Python scripts that import the built module and verify functionality.
- **End‑to‑end tests**: Spawn multiple Python processes that communicate via the bindings and assert consistency.

## Open Questions
1. Should we support Python 3.7? (PyO3 supports 3.7+ but 3.8 is recommended.)
2. How to handle Rust panics across the FFI boundary? (PyO3 converts them to `SystemError`.)
3. Should we provide a synchronous API for simplicity, or force users to use `asyncio`?

## References
- [PyO3 User Guide](https://pyo3.rs/)
- [Maturin Documentation](https://www.maturin.rs/)
- [Asynchronous Rust and Python with pyo3‑asyncio](https://github.com/awestlake87/pyo3-asyncio)