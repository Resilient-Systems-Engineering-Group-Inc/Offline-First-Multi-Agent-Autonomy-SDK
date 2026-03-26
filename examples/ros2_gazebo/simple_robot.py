#!/usr/bin/env python3
"""
Simple ROS2/Gazebo simulation example using the Offline‑First Multi‑Agent Autonomy SDK.

This script simulates two robots that share battery levels and decide which one should recharge.
It does not require an actual ROS2 or Gazebo installation; it uses dummy simulations.
"""

import asyncio
import json
import sys
from typing import Optional

# Import the SDK Python bindings
try:
    from offline_first_autonomy import PyAgent, PyMeshTransport
except ImportError:
    print("SDK Python bindings not found. Build the SDK with `cargo build --release` and install the Python package.")
    sys.exit(1)


class SimulatedRobot:
    """A simulated robot with battery level and position."""

    def __init__(self, robot_id: int, x: float = 0.0, y: float = 0.0):
        self.id = robot_id
        self.x = x
        self.y = y
        self.battery = 100.0  # percent
        self.charging = False

    def move_toward(self, target_x: float, target_y: float, distance: float = 1.0):
        """Move the robot towards a target (simulated)."""
        dx = target_x - self.x
        dy = target_y - self.y
        dist = (dx ** 2 + dy ** 2) ** 0.5
        if dist > 0:
            self.x += dx / dist * min(distance, dist)
            self.y += dy / dist * min(distance, dist)
        # Battery consumption
        self.battery = max(0.0, self.battery - 0.5)

    def recharge(self):
        """Simulate recharging."""
        if self.charging:
            self.battery = min(100.0, self.battery + 5.0)

    def update(self):
        """Update robot state (called each simulation step)."""
        if self.charging:
            self.recharge()
            if self.battery >= 100.0:
                self.charging = False
                print(f"Robot {self.id}: fully charged.")

    def __repr__(self):
        return f"Robot(id={self.id}, pos=({self.x:.1f},{self.y:.1f}), battery={self.battery:.1f}%)"


async def main():
    print("Starting ROS2/Gazebo simulation example (dummy simulation)")

    # Create two agents (robots)
    agent1 = PyAgent(1)
    agent2 = PyAgent(2)

    # Start agents (non‑blocking)
    agent1.start()
    agent2.start()

    # Create simulated robots
    robot1 = SimulatedRobot(1, x=0.0, y=0.0)
    robot2 = SimulatedRobot(2, x=5.0, y=5.0)

    # Charging station location
    charging_station = (10.0, 10.0)

    # Simulation loop
    for step in range(20):
        print(f"\n--- Step {step} ---")
        # Update robot states
        robot1.update()
        robot2.update()

        # Share battery levels via CRDT map
        battery1 = {"robot_id": 1, "battery": robot1.battery, "x": robot1.x, "y": robot1.y}
        battery2 = {"robot_id": 2, "battery": robot2.battery, "x": robot2.x, "y": robot2.y}

        agent1.set_value("robot/1", json.dumps(battery1))
        agent2.set_value("robot/2", json.dumps(battery2))

        # Broadcast changes (simulate network)
        await agent1.broadcast_changes()
        await agent2.broadcast_changes()

        # Retrieve peer battery levels (in a real scenario this would happen asynchronously)
        # For simplicity, we just read from the local map after a short delay.
        await asyncio.sleep(0.1)

        # Decision: which robot should go to charge?
        if robot1.battery < 30.0 and not robot1.charging:
            print(f"Robot 1 low battery ({robot1.battery:.1f}%), moving to charging station.")
            robot1.move_toward(*charging_station)
            if robot1.x == charging_station[0] and robot1.y == charging_station[1]:
                robot1.charging = True
                print("Robot 1 started charging.")
        if robot2.battery < 30.0 and not robot2.charging:
            print(f"Robot 2 low battery ({robot2.battery:.1f}%), moving to charging station.")
            robot2.move_toward(*charging_station)
            if robot2.x == charging_station[0] and robot2.y == charging_station[1]:
                robot2.charging = True
                print("Robot 2 started charging.")

        # Print status
        print(robot1)
        print(robot2)

        await asyncio.sleep(0.5)

    # Stop agents
    await agent1.stop()
    await agent2.stop()
    print("\nSimulation completed.")


if __name__ == "__main__":
    asyncio.run(main())