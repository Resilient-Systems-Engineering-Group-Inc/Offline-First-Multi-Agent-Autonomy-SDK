# ROS2/Gazebo Simulation Example

This directory contains an example of using the Offline‑First Multi‑Agent Autonomy SDK with ROS2 and Gazebo for robotics simulation.

## Overview

The SDK provides the foundational components for decentralized multi‑agent systems:
- **Mesh Transport**: Peer‑to‑peer communication between agents.
- **State Sync**: Conflict‑free replicated state (CRDT) synchronization.
- **Local Planner**: Decision‑making for each agent.
- **Resource Monitor**: System resource tracking.

In a robotics context, each agent corresponds to a robot (real or simulated). The SDK enables robots to share state, coordinate tasks, and operate reliably even when network connectivity is intermittent.

## Prerequisites

- ROS2 Humble or later
- Gazebo (with ROS2 integration)
- Python 3.8+
- Rust (for building the SDK)

## Example: Two Simulated Robots

The script `simple_robot.py` demonstrates two simulated robots (Agent 1 and Agent 2) that:

1. Discover each other via the mesh transport.
2. Share their battery levels (simulated) via the CRDT map.
3. Use a local planner to decide which robot should recharge based on battery levels.
4. Execute a simple “move to charging station” action (simulated).

## Running the Example

1. Build the SDK and Python bindings:

   ```bash
   cargo build --release
   cd python
   pip install -e .
   ```

2. Ensure you have a ROS2 workspace with a Gazebo simulation running (or use the provided dummy simulation).

3. Run the example:

   ```bash
   cd examples/ros2_gazebo
   python simple_robot.py
   ```

## Integration with ROS2

The SDK can be integrated into a ROS2 node using the Python bindings. Each robot runs a ROS2 node that subscribes to sensor topics, updates the agent’s CRDT map, and publishes commands based on the local planner’s decisions.

A more complete example would include:

- A ROS2 package that wraps the SDK.
- Launch files for multi‑robot simulation.
- Custom message types for robot‑specific state (position, battery, sensor readings).
- Gazebo plugins to simulate robot dynamics.

## Next Steps

- Implement a real ROS2 node that uses the SDK.
- Create a Gazebo world with multiple robots.
- Extend the local planner with more sophisticated task allocation algorithms.
- Add fault‑tolerance mechanisms (e.g., leader election, consensus).

## License

Same as the main SDK (Apache‑2.0 or MIT).