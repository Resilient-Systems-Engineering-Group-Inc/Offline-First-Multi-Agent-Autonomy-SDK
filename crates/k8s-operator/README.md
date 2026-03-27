# Kubernetes Operator for Offline‑First Multi‑Agent Autonomy SDK

This crate provides a Kubernetes operator that manages custom resources representing autonomous agents and tasks, reconciling them with the underlying mesh network and agent core.

## Features

- **Custom Resource Definitions (CRDs)** for `Agent` and `Task`
- **Reconciliation loops** that keep resource status up‑to‑date
- **Integration ready** with the existing `agent‑core` and `mesh‑transport` crates (optional features)
- **Production‑grade** RBAC, deployment manifests, and health checks

## Usage

### Prerequisites

- Rust 1.70+ and Cargo
- Kubernetes cluster (e.g., minikube, kind, or a cloud provider)
- `kubectl` configured to talk to your cluster

### Building the Operator

```bash
cargo build --release
```

### Running Locally

Set the `KUBECONFIG` environment variable and run:

```bash
cargo run -- --namespace default
```

The operator will start watching for `Agent` and `Task` resources in the specified namespace.

### Deploying to a Cluster

1. Apply the CRDs:

   ```bash
   kubectl apply -f manifests/agent-crd.yaml
   kubectl apply -f manifests/task-crd.yaml
   ```

2. Deploy the operator (you need to build a Docker image first):

   ```bash
   docker build -t autonomy/operator:latest -f Dockerfile .
   docker push autonomy/operator:latest   # if using a registry
   kubectl apply -f manifests/operator-deployment.yaml
   ```

3. Create example resources:

   ```bash
   kubectl apply -f examples/agent-example.yaml
   kubectl apply -f examples/task-example.yaml
   ```

## Architecture

The operator is built with the [`kube‑rs`](https://github.com/kube-rs/kube-rs) library and follows the standard controller pattern:

- **Controller** – watches for changes to `Agent` and `Task` resources.
- **Reconciler** – contains the business logic that brings the actual state closer to the desired state.
- **CRD** – defines the schema of the custom resources.

## Development

### Adding New Fields to CRDs

Edit `src/crd.rs` and regenerate the manifests (or update them manually). The `#[derive(CustomResource)]` macro from `kube` automatically generates the OpenAPI schema.

### Integrating with Agent Core

Enable the `agent‑core` feature in `Cargo.toml`:

```toml
k8s-operator = { path = "../k8s-operator", features = ["agent-core"] }
```

Then you can use `agent_core::Agent` inside the reconciler to manage real agents.

### Testing

Unit tests are located in `src/`. Integration tests require a running Kubernetes cluster; use `cargo test --features integration`.

## License

MIT OR Apache‑2.0