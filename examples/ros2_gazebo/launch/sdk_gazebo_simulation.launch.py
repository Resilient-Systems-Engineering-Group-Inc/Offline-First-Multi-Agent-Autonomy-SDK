"""
Launch file for multi-robot simulation with SDK integration.

This launch file starts:
- Gazebo simulation with specified world
- Multiple TurtleBot3 robots
- SDK ROS2 nodes for each robot
- RViz2 for visualization
- Task generator for demo scenarios
"""

import os
from ament_index_python.packages import get_package_share_directory
from launch import LaunchDescription
from launch.actions import (
    DeclareLaunchArgument,
    ExecuteProcess,
    IncludeLaunchDescription,
    RegisterEventHandler,
)
from launch.conditions import IfCondition
from launch.event_handlers import OnProcessExit
from launch.launch_description_sources import PythonLaunchDescriptionSource
from launch.substitutions import LaunchConfiguration, PathJoinSubstitution
from launch_ros.actions import Node


def generate_launch_description():
    # Package directory
    pkg_dir = get_package_share_directory('ros2_gazebo')
    
    # Declare launch arguments
    num_robots_arg = DeclareLaunchArgument(
        'num_robots',
        default_value='3',
        description='Number of robots to spawn'
    )
    
    world_arg = DeclareLaunchArgument(
        'world',
        default_value='warehouse',
        description='World to load (warehouse, office, outdoor)'
    )
    
    scenario_arg = DeclareLaunchArgument(
        'scenario',
        default_value='object_transport',
        description='Demo scenario to run'
    )
    
    use_rviz_arg = DeclareLaunchArgument(
        'use_rviz',
        default_value='true',
        description='Launch RViz2'
    )
    
    # World file
    world_name = LaunchConfiguration('world')
    world_file = PathJoinSubstitution([
        pkg_dir, 'worlds', f'{world_name}.world'
    ])
    
    # Gazebo server
    gz_server = ExecuteProcess(
        cmd=['gazebo', '--verbose', '-s', 'libgazebo_ros_factory.so', 
             '-s', 'libgazebo_ros_init.so', world_file],
        output='screen'
    )
    
    # Gazebo client
    gz_client = ExecuteProcess(
        cmd=['gazebo', '--verbose', world_file],
        output='screen'
    )
    
    # Spawn robots
    num_robots = LaunchConfiguration('num_robots')
    spawn_robots = []
    
    # Robot positions for warehouse world
    robot_positions = [
        ('0.0', '0.0', '0.0'),
        ('5.0', '0.0', '0.0'),
        ('0.0', '5.0', '0.0'),
        ('5.0', '5.0', '0.0'),
        ('2.5', '2.5', '0.0'),
    ]
    
    for i in range(int(num_robots)):
        robot_id = i + 1
        x, y, z = robot_positions[i % len(robot_positions)]
        
        # Set ROS_DOMAIN_ID for each robot to enable multi-robot communication
        ros_domain_id = 10 + i
        
        # Spawn robot model
        spawn_node = Node(
            package='gazebo_ros',
            executable='spawn_entity.py',
            name=f'spawn_robot_{robot_id}',
            output='screen',
            arguments=[
                '-entity', f'robot_{robot_id}',
                '-topic', '/robot_description',
                '-x', x,
                '-y', y,
                '-z', z,
                '-Y', '0.0'
            ],
            additional_env={'ROS_DOMAIN_ID': str(ros_domain_id)}
        )
        spawn_robots.append(spawn_node)
        
        # SDK ROS2 node for this robot
        sdk_node = Node(
            package='offline_first_autonomy',
            executable='agent_node',
            name=f'agent_{robot_id}',
            namespace=f'robot_{robot_id}',
            output='screen',
            parameters=[
                PathJoinSubstitution([pkg_dir, 'config', 'sdk_config.yaml']),
                {'agent_id': f'agent-{robot_id}'},
                {'ros_domain_id': ros_domain_id},
                {'robot_namespace': f'/robot_{robot_id}'}
            ],
            additional_env={'ROS_DOMAIN_ID': str(ros_domain_id)}
        )
        spawn_robots.append(sdk_node)
    
    # Task generator based on scenario
    scenario = LaunchConfiguration('scenario')
    task_generator = Node(
        package='offline_first_autonomy',
        executable='task_generator',
        name='task_generator',
        output='screen',
        parameters=[
            PathJoinSubstitution([pkg_dir, 'config', 'task_config.yaml']),
            {'scenario': scenario}
        ],
        condition=IfCondition(LaunchConfiguration('use_rviz'))
    )
    
    # RViz2
    rviz_config = PathJoinSubstitution([
        pkg_dir, 'config', 'sdk_rviz.rviz'
    ])
    
    rviz = Node(
        package='rviz2',
        executable='rviz2',
        name='rviz2',
        arguments=['-d', rviz_config],
        condition=IfCondition(use_rviz_arg)
    )
    
    # Prometheus metrics exporter
    metrics_exporter = Node(
        package='offline_first_autonomy',
        executable='metrics_exporter',
        name='metrics_exporter',
        output='screen',
        parameters=[
            {'metrics_port': 9090}
        ]
    )
    
    return LaunchDescription([
        num_robots_arg,
        world_arg,
        scenario_arg,
        use_rviz_arg,
        
        gz_server,
        gz_client,
        
        *spawn_robots,
        
        task_generator,
        metrics_exporter,
        rviz,
    ])
