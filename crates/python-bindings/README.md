# Python Bindings for Offline-First Multi-Agent Autonomy SDK

Full Python access to the SDK via PyO3.

## Installation

### From Source

```bash
# Install maturin (Python build tool for Rust)
pip install maturin

# Build and install
cd crates/python-bindings
maturin develop --release
```

### Development Mode

```bash
maturin develop --release --editable
```

## Quick Start

```python
import sdk
from sdk import MeshNode, TaskPlanner, WorkflowEngine, DashboardClient

# Check version
print(f"SDK Version: {sdk.version()}")

# Create a mesh node
node = MeshNode("my-agent")
await node.start()

# Use task planner
planner = TaskPlanner("auction")
```

## API Reference

### Core Components

#### MeshNode

P2P mesh networking node.

```python
# Create node
node = MeshNode(node_id="agent-1")

# Start/stop
await node.start()
await node.stop()

# Get info
print(node.node_id())
print(node.connected_peers())

# Communication
await node.connect("peer-1", "/ip4/127.0.0.1/tcp/4001")
await node.send("peer-1", b"message")
await node.broadcast(b"broadcast message")
```

#### StateSync

CRDT-based state synchronization.

```python
# Create state
state = StateSync()

# Set/Get/Delete
state.set("key", b"value")
value = state.get("key")
state.delete("key")

# Iterate
keys = state.keys()
count = state.len()
is_empty = state.is_empty()

# Merge state from other node
state.merge(delta_bytes)
```

#### Task

Task definition.

```python
task = Task(
    id="task-1",
    description="Explore zone A",
    priority=150,
    required_capabilities=["navigation", "lidar"],
    dependencies=["task-0"]
)
```

#### TaskPlanner

Multi-agent task planning.

```python
# Create planner
planner = TaskPlanner(algorithm="auction")

# Available algorithms
algorithms = TaskPlanner.available_algorithms()
# ['round_robin', 'auction', 'multi_objective', 
#  'reinforcement_learning', 'dynamic_load_balancer', 'hybrid']

# Add tasks
task = Task("task-1", "Description", priority=100)
planner.add_task(task)

# Plan assignments
assignments = await planner.plan()
# Returns: {agent_id: [task_ids]}
```

#### Workflow

Workflow definition.

```python
# Create programmatically
workflow = Workflow(
    id="exploration",
    name="Area Exploration",
    description="Collaborative mapping",
    version="1.0.0"
)

# Load from YAML
workflow = Workflow.from_yaml_file("workflow.yaml")
workflow = Workflow.from_yaml(yaml_string)
```

#### WorkflowEngine

Workflow orchestration engine.

```python
# Create engine
engine = WorkflowEngine(max_concurrent=4)

# Register workflow
workflow_id = await engine.register_workflow(workflow)

# Start workflow
instance_id = await engine.start_workflow(
    workflow_id,
    parameters={"area": "warehouse"}
)

# Monitor
status = await engine.get_workflow_status(instance_id)

# Control
await engine.pause_workflow(instance_id)
await engine.resume_workflow(instance_id)
await engine.cancel_workflow(instance_id)

# Wait for completion
result = await engine.wait_for_completion(instance_id)
print(result.status)  # 'completed', 'failed', 'cancelled'
if result.error:
    print(f"Error: {result.error}")
```

#### DashboardClient

REST API client for monitoring.

```python
# Create client
client = DashboardClient("http://localhost:3000")

# Health check
health = await client.health()

# Metrics
metrics = await client.metrics()
print(metrics['total_agents'])

# Agents
agents = await client.list_agents()
for agent in agents:
    print(f"{agent['id']}: {agent['status']}")

# Tasks
task = await client.create_task(
    description="Explore zone",
    priority=150
)
task_details = await client.get_task(task['id'])
await client.cancel_task(task['id'])

# Workflows
workflows = await client.list_workflows()
instance = await client.start_workflow("workflow_id", {"param": "value"})

# WebSocket URL for real-time updates
ws_url = client.websocket_url()
```

### Utility Functions

```python
# Get SDK version
version = sdk.version()

# Initialize logging
sdk.init_logging("debug")  # levels: 'error', 'warn', 'info', 'debug', 'trace'
```

## Examples

### Multi-Agent Coordination

```python
import asyncio
import sdk
from sdk import MeshNode, TaskPlanner, Task

async def main():
    # Create mesh nodes for agents
    agents = [
        MeshNode(f"agent-{i}")
        for i in range(3)
    ]

    # Start all agents
    for agent in agents:
        await agent.start()

    # Connect agents to each other
    for i, agent in enumerate(agents):
        for j, other in enumerate(agents):
            if i != j:
                await agent.connect(other.node_id(), f"/ip4/127.0.0.1/tcp/{4000+j}")

    # Create task planner
    planner = TaskPlanner("auction")

    # Add tasks
    tasks = [
        Task(f"task-{i}", f"Task {i}", priority=100 + i * 10)
        for i in range(6)
    ]

    for task in tasks:
        planner.add_task(task)

    # Plan assignments
    assignments = await planner.plan()

    # Execute tasks on each agent
    for agent_id, task_ids in assignments.items():
        print(f"{agent_id} assigned tasks: {task_ids}")

    # Cleanup
    for agent in agents:
        await agent.stop()

asyncio.run(main())
```

### Workflow Orchestration

```python
import asyncio
from sdk import WorkflowEngine, Workflow

async def main():
    # Create workflow engine
    engine = WorkflowEngine(max_concurrent=4)

    # Define workflow
    workflow = Workflow(
        id="warehouse_exploration",
        name="Warehouse Exploration",
        version="1.0.0"
    )

    # Register and start
    workflow_id = await engine.register_workflow(workflow)
    instance_id = await engine.start_workflow(
        workflow_id,
        {"warehouse": "main", "robots": "4"}
    )

    # Monitor progress
    while True:
        status = await engine.get_workflow_status(instance_id)
        print(f"Status: {status}")

        if status in ["completed", "failed", "cancelled"]:
            break

        await asyncio.sleep(1)

    # Get results
    result = await engine.wait_for_completion(instance_id)
    print(f"Result: {result.status}")

asyncio.run(main())
```

### Real-Time Monitoring

```python
import asyncio
import sdk
from sdk import DashboardClient

async def main():
    client = DashboardClient("http://localhost:3000")

    # Continuous monitoring
    while True:
        # Get metrics
        metrics = await client.metrics()
        
        print(f"Active agents: {metrics['active_agents']}/{metrics['total_agents']}")
        print(f"Pending tasks: {metrics['pending_tasks']}")
        print(f"Network latency: {metrics['network_latency_ms']:.1f}ms")

        # List agents
        agents = await client.list_agents()
        for agent in agents:
            if agent['status'] != 'active':
                print(f"Warning: {agent['id']} is {agent['status']}")

        await asyncio.sleep(5)

asyncio.run(main())
```

### State Synchronization

```python
import asyncio
from sdk import StateSync

async def main():
    # Create CRDT state
    state = StateSync()

    # Set initial state
    state.set("map/data", b"map_bytes")
    state.set("exploration/progress", b"75")
    state.set("obstacles/list", b"[obstacle1, obstacle2]")

    # Share with other nodes
    delta = serialize_state(state)  # Your serialization logic
    other_state = StateSync()
    other_state.merge(delta)

    # Verify consistency
    assert state.get("map/data") == other_state.get("map/data")

asyncio.run(main())
```

## Building from Source

### Prerequisites

- Rust toolchain (rustup)
- Python 3.8+
- maturin

```bash
pip install maturin
```

### Build Commands

```bash
# Debug build (fast compilation)
maturin develop

# Release build (optimized)
maturin develop --release

# Build wheel
maturin build --release

# Install from wheel
pip install target/wheels/*.whl
```

## Testing

```python
import pytest
import sdk

def test_version():
    assert sdk.version() is not None

def test_mesh_node():
    node = sdk.MeshNode("test")
    assert node.node_id() is not None

def test_state_sync():
    state = sdk.StateSync()
    state.set("key", b"value")
    assert state.get("key") == b"value"
```

## Performance

- **Rust performance** - All core logic runs at native Rust speeds
- **Zero-copy** - Efficient data transfer between Python and Rust
- **Async support** - Native async/await in Python
- **Thread-safe** - Full thread safety via Rust's type system

## Troubleshooting

### ImportError: No module named 'sdk'

```bash
# Rebuild and install
cd crates/python-bindings
maturin develop --release
```

### Async/Await Errors

Ensure you're using Python 3.7+ with async support:

```python
import asyncio

async def main():
    # async code here
    pass

asyncio.run(main())
```

### Build Errors

```bash
# Update Rust
rustup update

# Clean build
maturin clean
maturin develop --release
```

## Contributing

See main repository CONTRIBUTING.md for guidelines.

## License

MIT OR Apache-2.0
