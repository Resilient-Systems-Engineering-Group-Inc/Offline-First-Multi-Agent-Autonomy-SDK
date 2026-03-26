# Demo Simulation Specification

## Overview
A realistic demonstration of the SDK’s capabilities using Gazebo (or Ignition) and ROS2. The simulation will showcase multiple robots that coordinate via the offline‑first SDK without a central server, performing a collaborative task (e.g., distributed mapping, formation control, or search‑and‑rescue).

## Goals
1. **Validate the SDK** in a near‑real‑world scenario.
2. **Provide a reference implementation** for users to adapt.
3. **Measure performance** (latency, bandwidth, convergence time) under simulated network conditions.
4. **Demonstrate robustness** to network partitions and agent failures.

## Scenario: Collaborative Area Coverage
- **Environment**: Indoor warehouse with obstacles.
- **Agents**: 3–5 differential‑drive robots.
- **Task**: Each robot explores a portion of the environment, builds a local occupancy grid, and merges grids via the CRDT map. The combined map is used to plan coverage paths.
- **Constraints**:
  - Robots communicate only when within wireless range (simulated).
  - No central coordinator; each robot decides its own moves based on the shared map.
  - Robots may temporarily lose connectivity (partitions).

## Architecture

### Simulation Stack
```
┌─────────────────────────────────────────────────┐
│                Gazebo / Ignition                │
│                   (Physics)                     │
└─────────────────────────┬───────────────────────┘
                          │
┌─────────────────────────▼───────────────────────┐
│                ROS2 (Navigation2)               │
│          /cmd_vel     /scan      /map           │
└─────────────────────────┬───────────────────────┘
                          │
┌─────────────────────────▼───────────────────────┐
│          SDK Agent (Rust/Python Binding)        │
│   ├─ Mesh Transport (simulated wireless)        │
│   ├─ CRDT Map (shared occupancy grid)           │
│   └─ Local Planner (coverage path)              │
└─────────────────────────────────────────────────┘
```

### Components

#### 1. Gazebo World
- Warehouse model with walls, shelves, and open spaces.
- Robot models: TurtleBot3 or similar.
- Plugin to simulate wireless signal strength based on distance.

#### 2. ROS2 Nodes
- **Navigation Stack**: `nav2` for local and global planning.
- **Perception**: Simulated LiDAR (`/scan`) and odometry.
- **Map Server**: Publishes occupancy grid from CRDT map.

#### 3. SDK Integration
- Each robot runs an instance of the SDK (via Python bindings).
- The SDK’s Mesh Transport uses a custom simulator backend that respects distance‑based connectivity.
- The CRDT map stores:
  - `occupancy_grid`: 2D array of probabilities (shared).
  - `robot_pose`: each robot’s current position (updated periodically).
  - `task_assignment`: which region each robot is responsible for.

#### 4. Local Planner
- Simple behavior: if a cell is unexplored and assigned to this robot, move toward it.
- Assignment is dynamic: robots claim cells via CRDT operations.

## Implementation Steps

### Phase 1 – Simulation Environment Setup
1. Create a Dockerfile or installation script that sets up:
   - ROS2 Humble
   - Gazebo Classic (or Ignition)
   - TurtleBot3 packages
2. Build a custom world file (`warehouse.world`).
3. Write a launch file that spawns multiple robots.

### Phase 2 – SDK Integration with ROS2
1. Create a ROS2 node in Python that:
   - Subscribes to `/scan` and updates the local occupancy grid.
   - Publishes the robot’s pose to the CRDT map.
   - Reads the shared map and publishes it as `/map` (for visualization).
2. Implement a simulator‑aware Mesh Transport backend that uses Gazebo’s plugin to determine which robots are within communication range.

### Phase 3 – Collaborative Algorithm
1. Implement a distributed coverage algorithm:
   - Each robot randomly selects an unexplored cell and marks it as “claimed” in the CRDT map.
   - If two robots claim the same cell, the conflict is resolved via CRDT (last writer wins or explicit negotiation).
   - The robot navigates to the cell using `nav2`.
2. Add a visualization RVIZ configuration to watch the shared map and robot positions.

### Phase 4 – Fault Injection & Metrics
1. Introduce network partitions (disable communication between certain robots for a period).
2. Measure:
   - Time to converge to a complete map.
   - Communication overhead (messages per second).
   - CPU/memory usage per agent.
3. Record a demo video.

## Files & Directories
```
simulation/
├── docker/
│   ├── Dockerfile
│   └── docker‑compose.yml
├── worlds/
│   └── warehouse.world
├── launch/
│   └── multi_robot.launch.py
├── scripts/
│   ├── start_simulation.sh
│   └── metrics_collector.py
├── src/
│   └── coverage_agent/
│       ├── package.xml
│       ├── setup.py
│       ├── coverage_agent/
│       │   ├── __init__.py
│       │   ├── agent_node.py
│       │   └── sdk_wrapper.py
│       └── test/
└── README.md
```

## Dependencies
- ROS2 Humble (or Galactic)
- `gazebo_ros_pkgs`
- `nav2`, `turtlebot3_gazebo`
- Python 3.8+
- `offline-first-autonomy` Python package (our SDK)
- `rclpy`, `geometry_msgs`, `nav_msgs`, `sensor_msgs`

## Testing Strategy
- **Unit tests**: Test the coverage algorithm in isolation (without Gazebo).
- **Integration tests**: Run the simulation in headless mode and verify that the map converges.
- **Regression tests**: Use recorded bag files to ensure changes don’t break existing behavior.

## Open Questions
1. Should we use Ignition instead of Gazebo Classic? (Ignition is more modern but less ROS2‑mature.)
2. How to simulate wireless range realistically? (Use Gazebo’s `libgazebo_ros_range` plugin?)
3. Should we support hardware‑in‑the‑loop (HITL) for future real‑robot testing?

## References
- [ROS2 Navigation2](https://navigation.ros.org/)
- [Gazebo ROS Integration](http://gazebosim.org/tutorials?cat=connect_ros)
- [TurtleBot3 Simulation](https://emanual.robotis.com/docs/en/platform/turtlebot3/simulation/)