# Contributing to Offline‑First Multi‑Agent Autonomy SDK

Thank you for your interest in contributing to this project! This document provides guidelines and instructions for contributing.

## Code of Conduct

We expect all contributors to adhere to a respectful and inclusive environment. Please be kind and considerate in all interactions.

## How to Contribute

### Reporting Bugs

If you find a bug, please open an issue on GitHub with the following information:

- A clear, descriptive title.
- Steps to reproduce the bug.
- Expected behavior.
- Actual behavior.
- Environment details (OS, Rust version, etc.).
- Any relevant logs or screenshots.

### Suggesting Features

Feature suggestions are welcome! Open an issue and describe:

- The problem you're trying to solve.
- Your proposed solution.
- Any alternative solutions you've considered.
- Additional context (use cases, examples).

### Pull Requests

We welcome pull requests! To ensure a smooth review process, please follow these steps:

1. **Fork the repository** and create a branch from `main`.
2. **Make your changes** following the coding standards below.
3. **Write or update tests** for your changes.
4. **Run the test suite** to ensure everything passes.
5. **Update documentation** if needed.
6. **Submit the pull request** with a clear description of the changes.

## Development Setup

### Prerequisites

- Rust (latest stable) – install via [rustup](https://rustup.rs/)
- Python 3.8+ (for Python bindings)
- Git

### Building the Project

```bash
git clone https://github.com/your-org/Offline-First-Multi-Agent-Autonomy-SDK.git
cd Offline-First-Multi-Agent-Autonomy-SDK
cargo build --workspace
```

### Running Tests

```bash
cargo test --workspace
```

### Running Benchmarks

```bash
cargo bench --workspace
```

### Checking Code Quality

```bash
cargo clippy --workspace -- -D warnings
cargo fmt -- --check
```

## Coding Standards

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `rustfmt` to format code (configuration in `rustfmt.toml` if present).
- Use `clippy` to catch common mistakes and improve code quality.
- Write documentation comments for all public items (`///`).
- Prefer `anyhow` for application errors and `thiserror` for library errors.

### Python (Bindings)

- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/).
- Use type hints where possible.
- Write docstrings for all public functions and classes.

### Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Common types:
- `feat`: new feature
- `fix`: bug fix
- `docs`: documentation changes
- `style`: formatting, missing semicolons, etc.
- `refactor`: code restructuring without changing behavior
- `test`: adding or updating tests
- `chore`: maintenance tasks (dependencies, CI, etc.)

Example:
```
feat(agent-core): add fault tolerance module
```

### Documentation

- Keep the `ARCHITECTURE.md`, `README.md`, and `CHANGELOG.md` up to date.
- Document new features in the relevant module's `lib.rs` or `README`.
- Update examples if your changes affect public APIs.

## Project Structure

The project is organized as a Rust workspace with multiple crates:

- `crates/` – core Rust libraries
  - `agent-core` – high‑level agent abstraction
  - `mesh-transport` – mesh networking
  - `state-sync` – CRDT‑based state synchronization
  - `bounded-consensus` – consensus algorithms
  - `distributed-planner` – task planning
  - `local-planner` – local decision‑making
  - `resource-monitor` – system resource monitoring
  - … (many more specialized crates)
- `python/` – Python bindings via PyO3
- `examples/` – example applications
- `.github/` – GitHub Actions workflows
- `docs/` – additional documentation (if any)

## Testing

- Write unit tests in the same file as the code (within `mod tests`).
- Write integration tests in the `tests/` directory of each crate.
- For end‑to‑end tests, use the `examples/` directory.
- Ensure tests pass on all supported platforms (Linux, macOS, Windows).

## Continuous Integration

The project uses GitHub Actions for CI/CD. Before submitting a PR, ensure that:

- All tests pass (`cargo test`).
- Code formatting is correct (`cargo fmt`).
- Clippy reports no warnings (`cargo clippy`).
- Security checks pass (`cargo audit`, `cargo deny`).

## Release Process

Releases are managed by the maintainers. The version number follows [Semantic Versioning](https://semver.org/).

1. Update the `VERSION` file and `CHANGELOG.md`.
2. Create a Git tag (`vX.Y.Z`).
3. Push the tag to trigger the release workflow.
4. The workflow will build, test, and publish crates to crates.io (if configured).
5. Update the documentation and release notes on GitHub.

## Getting Help

If you have questions or need assistance, you can:

- Open a discussion on GitHub.
- Contact the maintainers via email (if listed).
- Join our community chat (if available).

## License

By contributing, you agree that your contributions will be licensed under the project's [LICENSE](LICENSE) (Apache 2.0).