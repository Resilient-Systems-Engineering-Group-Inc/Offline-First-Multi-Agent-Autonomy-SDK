#!/usr/bin/env python3
"""
Launch file for multi‑robot simulation with the Offline‑First Multi‑Agent Autonomy SDK.

Launches two robot nodes with different agent IDs.
"""

from launch import LaunchDescription
from launch_ros.actions import Node
from launch.actions import DeclareLaunchArgument
from launch.substitutions import LaunchConfiguration


def generate_launch_description():
    return LaunchDescription([
        DeclareLaunchArgument(
            'agent1_id',
            default_value='1',
            description='Agent ID for first robot'
        ),
        DeclareLaunchArgument(
            'agent2_id',
            default_value='2',
            description='Agent ID for second robot'
        ),
        Node(
            package='offline_first_autonomy',
            executable='ros2_node_example',
            name='robot_agent_1',
            parameters=[{'agent_id': LaunchConfiguration('agent1_id')}],
            output='screen',
            emulate_tty=True,
        ),
        Node(
            package='offline_first_autonomy',
            executable='ros2_node_example',
            name='robot_agent_2',
            parameters=[{'agent_id': LaunchConfiguration('agent2_id')}],
            output='screen',
            emulate_tty=True,
        ),
    ])