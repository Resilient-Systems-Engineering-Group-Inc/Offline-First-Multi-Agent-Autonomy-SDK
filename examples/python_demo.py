#!/usr/bin/env python3
"""
Python demo for the Offline-First Multi-Agent Autonomy SDK.

Demonstrates:
- Mesh networking
- State synchronization
- Task planning
- Workflow orchestration
- Dashboard monitoring
"""

import asyncio
import sdk
from sdk import (
    MeshNode,
    StateSync,
    Task,
    TaskPlanner,
    Workflow,
    WorkflowEngine,
    DashboardClient,
    version,
    init_logging,
)


async def demo_mesh_networking():
    """Demonstrate mesh networking capabilities."""
    print("=== Mesh Networking Demo ===\n")

    # Create mesh nodes
    node1 = MeshNode("node-1")
    node2 = MeshNode("node-2")

    print(f"Node 1 ID: {node1.node_id()}")
    print(f"Node 2 ID: {node2.node_id()}")

    # Start nodes
    await node1.start()
    await node2.start()

    print("Nodes started successfully")

    # Connect nodes
    await node1.connect("node-2", "/ip4/127.0.0.1/tcp/4001")

    # Send message
    message = b"Hello from node 1!"
    await node1.send("node-2", message)

    print(f"Sent message to node-2: {message}")

    # Get connected peers
    peers = node1.connected_peers()
    print(f"Connected peers: {peers}")

    # Stop nodes
    await node1.stop()
    await node2.stop()

    print("\n✓ Mesh networking demo complete\n")


async def demo_state_sync():
    """Demonstrate CRDT state synchronization."""
    print("=== State Synchronization Demo ===\n")

    # Create CRDT state
    state = StateSync()

    # Set values
    state.set("agent-1/location", b"warehouse-north")
    state.set("agent-1/battery", b"85")
    state.set("agent-2/location", b"warehouse-south")
    state.set("agent-2/battery", b"92")

    print(f"State has {state.len()} keys")

    # Get values
    location = state.get("agent-1/location")
    if location:
        print(f"Agent 1 location: {location.decode()}")

    # List all keys
    keys = state.keys()
    print(f"All keys: {keys}")

    # Delete a key
    state.delete("agent-1/battery")
    print(f"Keys after deletion: {state.keys()}")

    print("\n✓ State sync demo complete\n")


async def demo_task_planning():
    """Demonstrate task planning algorithms."""
    print("=== Task Planning Demo ===\n")

    # Get available algorithms
    algorithms = TaskPlanner.available_algorithms()
    print(f"Available algorithms: {algorithms}")

    # Create planner with auction algorithm
    planner = TaskPlanner("auction")

    # Create tasks
    tasks = [
        Task("task-1", "Explore zone A", priority=150, required_capabilities=["navigation", "lidar"]),
        Task("task-2", "Explore zone B", priority=100, required_capabilities=["navigation"]),
        Task("task-3", "Map area", priority=200, required_capabilities=["lidar", "computation"]),
    ]

    # Add tasks to planner
    for task in tasks:
        planner.add_task(task)

    print(f"Added {len(tasks)} tasks to planner")

    # Plan task assignment
    assignments = await planner.plan()
    print(f"Task assignments: {assignments}")

    print("\n✓ Task planning demo complete\n")


async def demo_workflow_orchestration():
    """Demonstrate workflow orchestration."""
    print("=== Workflow Orchestration Demo ===\n")

    # Create workflow engine
    engine = WorkflowEngine(max_concurrent=4)

    # Create workflow programmatically
    workflow = Workflow(
        id="exploration_workflow",
        name="Area Exploration",
        description="Collaborative exploration of unknown area",
        version="1.0.0"
    )

    print(f"Created workflow: {workflow.name} (v{workflow.version})")

    # Alternative: Load from YAML
    # workflow = Workflow.from_yaml_file("workflow.yaml")

    # Register workflow
    workflow_id = await engine.register_workflow(workflow)
    print(f"Workflow registered: {workflow_id}")

    # Start workflow with parameters
    parameters = {
        "area": "warehouse",
        "robot_count": "4",
        "resolution": "0.05"
    }

    instance_id = await engine.start_workflow(workflow_id, parameters)
    print(f"Workflow started: {instance_id}")

    # Monitor status
    status = await engine.get_workflow_status(instance_id)
    print(f"Status: {status}")

    # Wait for completion
    result = await engine.wait_for_completion(instance_id)
    print(f"Workflow completed: {result.status}")
    if result.error:
        print(f"Error: {result.error}")

    print("\n✓ Workflow orchestration demo complete\n")


async def demo_dashboard_client():
    """Demonstrate dashboard monitoring."""
    print("=== Dashboard Client Demo ===\n")

    # Create dashboard client
    client = DashboardClient("http://localhost:3000")

    # Check health
    health = await client.health()
    print(f"Dashboard status: {health}")

    # Get metrics
    metrics = await client.metrics()
    print(f"Metrics: {metrics}")

    # List agents
    agents = await client.list_agents()
    print(f"Agents: {len(agents)} connected")
    for agent in agents:
        print(f"  - {agent['id']}: {agent['status']}")

    # Create a task
    task = await client.create_task("Explore zone A", priority=150)
    print(f"\nCreated task: {task['id']}")

    # Get task details
    task_details = await client.get_task(task['id'])
    print(f"Task status: {task_details['status']}")

    # List workflows
    workflows = await client.list_workflows()
    print(f"\nActive workflows: {len(workflows)}")

    # Start a workflow
    workflow_instance = await client.start_workflow(
        "exploration_workflow",
        {"area": "warehouse"}
    )
    print(f"Started workflow: {workflow_instance['id']}")

    # WebSocket URL for real-time updates
    ws_url = client.websocket_url()
    print(f"\nWebSocket URL: {ws_url}")

    print("\n✓ Dashboard client demo complete\n")


async def main():
    """Run all demos."""
    print("=" * 60)
    print("Offline-First Multi-Agent Autonomy SDK - Python Demo")
    print(f"SDK Version: {version()}")
    print("=" * 60)
    print()

    # Initialize logging
    init_logging("debug")

    # Run demos
    try:
        await demo_mesh_networking()
        await demo_state_sync()
        await demo_task_planning()
        await demo_workflow_orchestration()
        await demo_dashboard_client()

        print("=" * 60)
        print("All demos completed successfully!")
        print("=" * 60)

    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
