#!/usr/bin/env python3
"""
Task generator for multi-robot simulation scenarios.

Generates tasks based on configured scenarios and publishes them
to the SDK for distributed planning.
"""

import rclpy
from rclpy.node import Node
from rclpy.qos import QoSProfile, QoSReliabilityPolicy, QoSHistoryPolicy
from geometry_msgs.msg import PoseStamped
from std_msgs.msg import String, Int32
import yaml
import random
import time
from typing import List, Dict, Any


class Task:
    def __init__(
        self,
        task_id: str,
        description: str,
        priority: int,
        required_capabilities: List[str],
        dependencies: List[str] = None,
        deadline: int = None,
        estimated_duration: int = 60,
        metadata: Dict[str, Any] = None
    ):
        self.task_id = task_id
        self.description = description
        self.priority = priority
        self.required_capabilities = required_capabilities
        self.dependencies = dependencies or []
        self.deadline = deadline
        self.estimated_duration = estimated_duration
        self.metadata = metadata or {}
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            'task_id': self.task_id,
            'description': self.description,
            'priority': self.priority,
            'required_capabilities': self.required_capabilities,
            'dependencies': self.dependencies,
            'deadline': self.deadline,
            'estimated_duration': self.estimated_duration,
            'metadata': self.metadata
        }


class TaskGeneratorNode(Node):
    """Node that generates tasks for multi-robot scenarios."""
    
    def __init__(self):
        super().__init__('task_generator')
        
        # Declare and read parameters
        self.declare_parameter('scenario', 'object_transport')
        self.declare_parameter('tasks_per_minute', 5)
        self.declare_parameter('max_concurrent_tasks', 10)
        self.declare_parameter('random_seed', 42)
        
        self.scenario = self.get_parameter('scenario').value
        self.tasks_per_minute = self.get_parameter('tasks_per_minute').value
        self.max_concurrent_tasks = self.get_parameter('max_concurrent_tasks').value
        self.random_seed = self.get_parameter('random_seed').value
        
        random.seed(self.random_seed)
        
        # Load task templates
        self.task_templates = self._load_task_templates()
        
        # State tracking
        self.active_tasks: Dict[str, Task] = {}
        self.completed_tasks: List[str] = []
        self.task_counter = 0
        
        # Publishers
        qos_profile = QoSProfile(
            reliability=QoSReliabilityPolicy.RELIABLE,
            history=QoSHistoryPolicy.KEEP_LAST,
            depth=10
        )
        
        self.task_publisher = self.create_publisher(
            String,
            '/sdk/task/new',
            qos_profile
        )
        
        self.task_status_publisher = self.create_publisher(
            Int32,
            '/sdk/task/status/count',
            qos_profile
        )
        
        # Timer for task generation
        generation_interval = 60.0 / self.tasks_per_minute if self.tasks_per_minute > 0 else 60.0
        self.generation_timer = self.create_timer(
            generation_interval,
            self._generate_task_timer_callback
        )
        
        # Scenario-specific timers
        self._setup_scenario_timers()
        
        self.get_logger().info(f"Task generator started for scenario: {self.scenario}")
        self.get_logger().info(f"Task generation rate: {self.tasks_per_minute} tasks/minute")
    
    def _load_task_templates(self) -> List[Dict[str, Any]]:
        """Load task templates from configuration."""
        # Default templates
        templates = [
            {
                'name': 'navigate_to_point',
                'priority': 100,
                'duration': 30,
                'capabilities': ['navigation'],
                'description_template': 'Navigate to {location}'
            },
            {
                'name': 'scan_area',
                'priority': 150,
                'duration': 60,
                'capabilities': ['navigation', 'lidar'],
                'description_template': 'Scan area {area_id}'
            },
            {
                'name': 'transport_object',
                'priority': 200,
                'duration': 120,
                'capabilities': ['navigation', 'gripper'],
                'description_template': 'Transport object from {pickup} to {dropoff}'
            },
            {
                'name': 'emergency_response',
                'priority': 255,
                'duration': 30,
                'capabilities': ['navigation'],
                'description_template': 'Emergency response at {location}'
            }
        ]
        
        return templates
    
    def _setup_scenario_timers(self):
        """Setup scenario-specific task generation timers."""
        if self.scenario == 'object_transport':
            # Generate transport tasks
            self.get_logger().info("Object Transport scenario active")
            
        elif self.scenario == 'collaborative_mapping':
            # Generate mapping tasks
            self.get_logger().info("Collaborative Mapping scenario active")
            
        elif self.scenario == 'search_rescue':
            # Generate search and rescue tasks with high priority
            self.get_logger().info("Search and Rescue scenario active")
            # Increase generation rate for emergency tasks
            emergency_timer = self.create_timer(30.0, self._generate_emergency_task)
            
        elif self.scenario == 'formation':
            # Generate formation control tasks
            self.get_logger().info("Formation Control scenario active")
    
    def _generate_task_timer_callback(self):
        """Timer callback for regular task generation."""
        if len(self.active_tasks) >= self.max_concurrent_tasks:
            self.get_logger().warn("Max concurrent tasks reached, skipping generation")
            return
        
        task = self._generate_scenario_task()
        if task:
            self._publish_task(task)
            self.active_tasks[task.task_id] = task
            self._update_status()
    
    def _generate_scenario_task(self) -> Task:
        """Generate a task based on current scenario."""
        self.task_counter += 1
        task_id = f"task-{self.scenario}-{self.task_counter}"
        
        if self.scenario == 'object_transport':
            locations = ['zone_a', 'zone_b', 'zone_c', 'zone_x', 'zone_y', 'zone_z']
            pickup = random.choice(locations)
            dropoff = random.choice([l for l in locations if l != pickup])
            
            priorities = [150, 180, 200]
            priority = random.choice(priorities)
            
            return Task(
                task_id=task_id,
                description=f"Transport object from {pickup} to {dropoff}",
                priority=priority,
                required_capabilities=['navigation', 'gripper'],
                estimated_duration=random.randint(90, 150),
                metadata={
                    'pickup_zone': pickup,
                    'dropoff_zone': dropoff,
                    'object_type': random.choice(['small', 'medium', 'large'])
                }
            )
        
        elif self.scenario == 'collaborative_mapping':
            zones = ['north', 'south', 'east', 'west', 'center']
            zone = random.choice(zones)
            
            return Task(
                task_id=task_id,
                description=f"Map {zone} zone",
                priority=150,
                required_capabilities=['navigation', 'lidar'],
                estimated_duration=random.randint(180, 300),
                metadata={'zone': zone}
            )
        
        elif self.scenario == 'search_rescue':
            locations = [f"sector_{chr(65+i)}" for i in range(6)]
            location = random.choice(locations)
            
            return Task(
                task_id=task_id,
                description=f"Search for targets in {location}",
                priority=255,
                required_capabilities=['navigation', 'camera'],
                deadline=int(time.time()) + 300,
                estimated_duration=random.randint(60, 120),
                metadata={
                    'sector': location,
                    'search_type': random.choice(['visual', 'thermal', 'audio'])
                }
            )
        
        elif self.scenario == 'formation':
            formations = ['triangle', 'line', 'wedge', 'circle']
            formation = random.choice(formations)
            
            return Task(
                task_id=task_id,
                description=f"Maintain {formation} formation",
                priority=120,
                required_capabilities=['navigation'],
                estimated_duration=60,
                metadata={'formation_type': formation}
            )
        
        else:
            # Default random task
            return Task(
                task_id=task_id,
                description=f"Generic task {self.task_counter}",
                priority=random.randint(100, 200),
                required_capabilities=['navigation'],
                estimated_duration=random.randint(30, 120)
            )
    
    def _generate_emergency_task(self):
        """Generate high-priority emergency task."""
        self.task_counter += 1
        task_id = f"emergency-{self.task_counter}"
        
        locations = ['sector_a', 'sector_b', 'sector_c', 'sector_d']
        location = random.choice(locations)
        
        task = Task(
            task_id=task_id,
            description=f"Emergency response at {location}",
            priority=255,
            required_capabilities=['navigation'],
            deadline=int(time.time()) + 60,
            estimated_duration=30,
            metadata={'emergency_type': random.choice(['fire', 'medical', 'security'])}
        )
        
        self._publish_task(task)
        self.active_tasks[task.task_id] = task
        self._update_status()
    
    def _publish_task(self, task: Task):
        """Publish task to SDK."""
        task_yaml = yaml.dump(task.to_dict())
        msg = String()
        msg.data = task_yaml
        self.task_publisher.publish(msg)
        
        self.get_logger().info(f"Generated task: {task.task_id} - {task.description}")
    
    def _update_status(self):
        """Update task status counter."""
        msg = Int32()
        msg.data = len(self.active_tasks)
        self.task_status_publisher.publish(msg)
    
    def task_completed_callback(self, task_id: str):
        """Callback when a task is completed."""
        if task_id in self.active_tasks:
            del self.active_tasks[task_id]
            self.completed_tasks.append(task_id)
            self._update_status()
            self.get_logger().info(f"Task completed: {task_id}")


def main(args=None):
    rclpy.init(args=args)
    node = TaskGeneratorNode()
    
    try:
        rclpy.spin(node)
    except KeyboardInterrupt:
        pass
    finally:
        node.destroy_node()
        rclpy.shutdown()


if __name__ == '__main__':
    main()
