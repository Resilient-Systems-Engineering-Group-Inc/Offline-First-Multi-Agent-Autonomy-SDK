#!/usr/bin/env python3
"""
ROS2 node example using the Offline‑First Multi‑Agent Autonomy SDK.

This node:
- Subscribes to a sensor topic (e.g., battery level)
- Publishes commands (e.g., movement)
- Shares state with other agents via the SDK's CRDT map
- Uses local planner to make decisions based on shared state

Run with:
    ros2 run offline_first_autonomy ros2_node_example --ros-args -p agent_id:=1
"""

import rclpy
from rclpy.node import Node
from std_msgs.msg import Float32, String
from geometry_msgs.msg import Twist
import asyncio
import json
import sys
import threading

# Import the SDK Python bindings
try:
    from offline_first_autonomy import PyAgent
except ImportError:
    print("SDK Python bindings not found. Build the SDK with `cargo build --release` and install the Python package.")
    sys.exit(1)


class RobotNode(Node):
    """ROS2 node representing a robot agent."""

    def __init__(self, agent_id: int):
        super().__init__(f'robot_agent_{agent_id}')
        self.agent_id = agent_id

        # Create SDK agent
        self.agent = PyAgent(agent_id)
        self.agent.start()

        # Publisher for velocity commands
        self.cmd_vel_pub = self.create_publisher(Twist, 'cmd_vel', 10)

        # Subscriber for battery level (simulated)
        self.battery_sub = self.create_subscription(
            Float32,
            'battery_level',
            self.battery_callback,
            10
        )

        # Subscriber for incoming SDK messages (custom topic)
        self.sdk_sub = self.create_subscription(
            String,
            'sdk_messages',
            self.sdk_message_callback,
            10
        )

        # Publisher for SDK messages (to other robots)
        self.sdk_pub = self.create_publisher(String, 'sdk_messages', 10)

        # Timer for periodic state sync
        self.timer = self.create_timer(1.0, self.timer_callback)

        # Local state
        self.battery = 100.0
        self.position = (0.0, 0.0)
        self.charging = False

        # Thread for async SDK operations
        self.loop = asyncio.new_event_loop()
        self.thread = threading.Thread(target=self._run_async_loop, daemon=True)
        self.thread.start()

        self.get_logger().info(f'Robot agent {agent_id} started')

    def _run_async_loop(self):
        asyncio.set_event_loop(self.loop)
        self.loop.run_forever()

    async def _broadcast_changes(self):
        """Broadcast CRDT changes asynchronously."""
        await self.agent.broadcast_changes()

    def battery_callback(self, msg: Float32):
        """Update local battery level and share via CRDT."""
        self.battery = msg.data
        # Share battery level with other agents
        state = {
            'robot_id': self.agent_id,
            'battery': self.battery,
            'x': self.position[0],
            'y': self.position[1],
            'charging': self.charging,
        }
        self.agent.set_value(f'robot/{self.agent_id}', json.dumps(state))
        # Schedule broadcast
        asyncio.run_coroutine_threadsafe(self._broadcast_changes(), self.loop)

    def sdk_message_callback(self, msg: String):
        """Handle incoming SDK messages from other robots."""
        # In a real implementation, you would decode the message and update local state.
        # For simplicity, we just log.
        self.get_logger().debug(f'Received SDK message: {msg.data}')

    def timer_callback(self):
        """Periodic task: read shared state and make decisions."""
        # Read other robots' states from CRDT map
        other_states = []
        for key in [f'robot/{i}' for i in (1, 2, 3)]:
            if key == f'robot/{self.agent_id}':
                continue
            val = self.agent.get_value(key)
            if val:
                try:
                    other_states.append(json.loads(val))
                except json.JSONDecodeError:
                    pass

        # Simple decision logic: if my battery is low and no one else is charging, go charge
        if self.battery < 30.0 and not self.charging:
            charging_occupied = any(state.get('charging', False) for state in other_states)
            if not charging_occupied:
                self.get_logger().info(f'Low battery ({self.battery:.1f}%), moving to charging station.')
                self._publish_cmd_vel(0.5, 0.0)  # move forward
                # Simulate reaching charging station
                if self.position[0] > 9.0:
                    self.charging = True
                    self.get_logger().info('Started charging.')
            else:
                self.get_logger().info('Waiting for charging station to be free.')
                self._publish_cmd_vel(0.0, 0.0)  # stop

        # If charging, stop moving and increase battery
        if self.charging:
            self._publish_cmd_vel(0.0, 0.0)
            self.battery = min(100.0, self.battery + 5.0)
            if self.battery >= 100.0:
                self.charging = False
                self.get_logger().info('Fully charged.')

        # Publish updated state
        state = {
            'robot_id': self.agent_id,
            'battery': self.battery,
            'x': self.position[0],
            'y': self.position[1],
            'charging': self.charging,
        }
        self.agent.set_value(f'robot/{self.agent_id}', json.dumps(state))
        asyncio.run_coroutine_threadsafe(self._broadcast_changes(), self.loop)

    def _publish_cmd_vel(self, linear: float, angular: float):
        """Publish a Twist message."""
        msg = Twist()
        msg.linear.x = linear
        msg.angular.z = angular
        self.cmd_vel_pub.publish(msg)

    def destroy_node(self):
        """Cleanup before shutdown."""
        self.get_logger().info('Shutting down robot agent')
        # Stop agent (async)
        future = asyncio.run_coroutine_threadsafe(self.agent.stop(), self.loop)
        future.result(timeout=5.0)
        self.loop.call_soon_threadsafe(self.loop.stop)
        self.thread.join(timeout=2.0)
        super().destroy_node()


def main(args=None):
    rclpy.init(args=args)

    # Get agent ID from parameter
    node = Node('temp_param_reader')
    agent_id = node.declare_parameter('agent_id', 1).value
    node.destroy_node()

    robot_node = RobotNode(agent_id)

    try:
        rclpy.spin(robot_node)
    except KeyboardInterrupt:
        pass
    finally:
        robot_node.destroy_node()
        rclpy.shutdown()


if __name__ == '__main__':
    main()