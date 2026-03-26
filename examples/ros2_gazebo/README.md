# ROS2/Gazebo Simulation Example

This directory contains examples of using the Offline‑First Multi‑Agent Autonomy SDK with ROS2 and Gazebo for robotics simulation.

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

## Example 1: Simple Simulation (No ROS2)

The script `simple_robot.py` demonstrates two simulated robots (Agent 1 and Agent 2) that:

1. Discover each other via the mesh transport.
2. Share their battery levels (simulated) via the CRDT map.
3. Use a local planner to decide which robot should recharge based on battery levels.
4. Execute a simple “move to charging station” action (simulated).

### Running the Example

1. Build the SDK and Python bindings:

   ```bash
   cargo build --release
   cd python
   pip install -e .
   ```

2. Run the example:

   ```bash
   cd examples/ros2_gazebo
   python simple_robot.py
   ```

## Example 2: ROS2 Node Integration

The file `ros2_node_example.py` is a fully functional ROS2 node that integrates the SDK. It:

- Subscribes to a simulated battery topic.
- Publishes velocity commands.
- Shares robot state (battery, position, charging status) with other agents via the SDK's CRDT map.
- Makes decisions based on shared state (e.g., which robot should recharge).

### Running the ROS2 Node

1. Ensure the SDK Python bindings are installed (as above).

2. Make the script executable (optional):

   ```bash
   chmod +x examples/ros2_gazebo/ros2_node_example.py
   ```

3. Run the node with a specific agent ID:

   ```bash
   ros2 run offline_first_autonomy ros2_node_example --ros-args -p agent_id:=1
   ```

   (You may need to install the package first; see below.)

### Launch File for Multi‑Robot Simulation

A launch file `launch/multi_robot.launch.py` is provided to start two robot nodes simultaneously.

```bash
ros2 launch offline_first_autonomy multi_robot.launch.py
```

## Creating a ROS2 Package

To integrate the SDK into your own ROS2 workspace, you can create a package:

1. Create a new ROS2 package (if you haven't already):

   ```bash
   ros2 pkg create --build-type ament_python offline_first_autonomy
   ```

2. Copy the example node and launch files into the package.

3. Add dependencies to `package.xml`:

   ```xml
   <exec_depend>rclpy</exec_depend>
   <exec_depend>std_msgs</exec_depend>
   <exec_depend>geometry_msgs</exec_depend>
   ```

4. Update `setup.py` to include the script and launch files.

5. Build and install the package:

   ```bash
   colcon build --packages-select offline_first_autonomy
   source install/setup.bash
   ```

## Integration with Gazebo

For a complete simulation, you can connect the ROS2 nodes to a Gazebo world:

- Use Gazebo plugins to simulate robot dynamics.
- Publish sensor data (battery, position) to the topics the node subscribes to.
- Subscribe to the `cmd_vel` topic published by the node to move the robot.

A sample Gazebo world and robot model are not included in this example but can be added in future extensions.

## Next Steps

- Implement a real ROS2 node that uses the SDK with actual sensor data.
- Create a Gazebo world with multiple robots.
- Extend the local planner with more sophisticated task allocation algorithms.
- Add fault‑tolerance mechanisms (e.g., leader election, consensus).
- Use the SDK's bounded consensus for coordinated decision‑making.

## License

Same as the main SDK (Apache‑2.0 or MIT).