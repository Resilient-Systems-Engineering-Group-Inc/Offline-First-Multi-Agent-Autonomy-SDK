//! Simulation scenarios for testing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Scenario definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub name: String,
    pub description: String,
    pub agents: Vec<AgentDefinition>,
    pub obstacles: Vec<Obstacle>,
    pub tasks: Vec<TaskDefinition>,
    pub environment: EnvironmentConfig,
    pub expected_duration_secs: Option<f64>,
}

/// Agent definition in scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub model: String,
    pub initial_position: [f64; 3],
    pub capabilities: Vec<String>,
}

/// Obstacle in simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obstacle {
    pub id: String,
    pub position: [f64; 3],
    pub dimensions: [f64; 3],
    pub shape: ObstacleShape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObstacleShape {
    Box,
    Sphere,
    Cylinder,
}

/// Task definition for scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub description: String,
    pub target_position: [f64; 3],
    pub required_capabilities: Vec<String>,
    pub priority: i32,
}

/// Environment configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub map_size: [f64; 2],
    pub obstacles_count: usize,
    pub dynamic_obstacles: bool,
    pub weather: WeatherCondition,
    pub lighting: LightingCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeatherCondition {
    Clear,
    Rain,
    Fog,
    Snow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LightingCondition {
    Day,
    Night,
    Dawn,
    Dusk,
    Indoor,
}

/// Scenario library.
pub struct ScenarioLibrary {
    scenarios: HashMap<String, Scenario>,
}

impl ScenarioLibrary {
    pub fn new() -> Self {
        let mut library = Self {
            scenarios: HashMap::new(),
        };

        // Add default scenarios
        library.add_default_scenarios();
        
        library
    }

    /// Add default scenarios.
    fn add_default_scenarios(&mut self) {
        // Single agent exploration
        self.scenarios.insert("single_agent_exploration".to_string(), Scenario {
            name: "Single Agent Exploration".to_string(),
            description: "Single agent explores unknown environment".to_string(),
            agents: vec![AgentDefinition {
                id: "agent-1".to_string(),
                name: "Explorer".to_string(),
                model: "turtlebot3".to_string(),
                initial_position: [0.0, 0.0, 0.0],
                capabilities: vec!["navigation".to_string(), "sensors".to_string()],
            }],
            obstacles: vec![
                Obstacle {
                    id: "obs-1".to_string(),
                    position: [5.0, 5.0, 0.0],
                    dimensions: [1.0, 1.0, 2.0],
                    shape: ObstacleShape::Box,
                },
            ],
            tasks: vec![TaskDefinition {
                id: "task-1".to_string(),
                description: "Explore entire area".to_string(),
                target_position: [10.0, 10.0, 0.0],
                required_capabilities: vec!["navigation".to_string()],
                priority: 100,
            }],
            environment: EnvironmentConfig {
                map_size: [20.0, 20.0],
                obstacles_count: 1,
                dynamic_obstacles: false,
                weather: WeatherCondition::Clear,
                lighting: LightingCondition::Day,
            },
            expected_duration_secs: Some(300.0),
        });

        // Multi-agent coordination
        self.scenarios.insert("multi_agent_coordination".to_string(), Scenario {
            name: "Multi-Agent Coordination".to_string(),
            description: "Multiple agents coordinate to complete tasks".to_string(),
            agents: vec![
                AgentDefinition {
                    id: "agent-1".to_string(),
                    name: "Leader".to_string(),
                    model: "turtlebot3".to_string(),
                    initial_position: [0.0, 0.0, 0.0],
                    capabilities: vec!["navigation".to_string(), "communication".to_string()],
                },
                AgentDefinition {
                    id: "agent-2".to_string(),
                    name: "Follower".to_string(),
                    model: "turtlebot3".to_string(),
                    initial_position: [2.0, 0.0, 0.0],
                    capabilities: vec!["navigation".to_string()],
                },
            ],
            obstacles: vec![],
            tasks: vec![
                TaskDefinition {
                    id: "task-1".to_string(),
                    description: "Reach target location together".to_string(),
                    target_position: [10.0, 5.0, 0.0],
                    required_capabilities: vec!["navigation".to_string()],
                    priority: 100,
                },
            ],
            environment: EnvironmentConfig {
                map_size: [20.0, 20.0],
                obstacles_count: 0,
                dynamic_obstacles: false,
                weather: WeatherCondition::Clear,
                lighting: LightingCondition::Day,
            },
            expected_duration_secs: Some(120.0),
        });

        // Emergency response
        self.scenarios.insert("emergency_response".to_string(), Scenario {
            name: "Emergency Response".to_string(),
            description: "Agents respond to emergency situation".to_string(),
            agents: vec![
                AgentDefinition {
                    id: "agent-1".to_string(),
                    name: "First Responder".to_string(),
                    model: "quadcopter".to_string(),
                    initial_position: [0.0, 0.0, 5.0],
                    capabilities: vec!["flight".to_string(), "sensors".to_string()],
                },
                AgentDefinition {
                    id: "agent-2".to_string(),
                    name: "Ground Unit".to_string(),
                    model: "turtlebot3".to_string(),
                    initial_position: [0.0, 2.0, 0.0],
                    capabilities: vec!["navigation".to_string(), "manipulation".to_string()],
                },
            ],
            obstacles: vec![
                Obstacle {
                    id: "obs-1".to_string(),
                    position: [5.0, 5.0, 0.0],
                    dimensions: [2.0, 2.0, 0.5],
                    shape: ObstacleShape::Box,
                },
            ],
            tasks: vec![
                TaskDefinition {
                    id: "task-1".to_string(),
                    description: "Assess emergency location".to_string(),
                    target_position: [10.0, 10.0, 0.0],
                    required_capabilities: vec!["sensors".to_string()],
                    priority: 200,
                },
                TaskDefinition {
                    id: "task-2".to_string(),
                    description: "Provide assistance".to_string(),
                    target_position: [10.0, 10.0, 0.0],
                    required_capabilities: vec!["manipulation".to_string()],
                    priority: 200,
                },
            ],
            environment: EnvironmentConfig {
                map_size: [30.0, 30.0],
                obstacles_count: 1,
                dynamic_obstacles: true,
                weather: WeatherCondition::Rain,
                lighting: LightingCondition::Dusk,
            },
            expected_duration_secs: Some(180.0),
        });
    }

    /// Get scenario by name.
    pub fn get(&self, name: &str) -> Option<&Scenario> {
        self.scenarios.get(name)
    }

    /// List all scenarios.
    pub fn list(&self) -> Vec<&str> {
        self.scenarios.keys().map(|s| s.as_str()).collect()
    }

    /// Add custom scenario.
    pub fn add(&mut self, scenario: Scenario) {
        self.scenarios.insert(scenario.name.clone(), scenario);
    }
}

impl Default for ScenarioLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_library() {
        let library = ScenarioLibrary::new();

        let scenarios = library.list();
        assert!(scenarios.contains(&"single_agent_exploration"));
        assert!(scenarios.contains(&"multi_agent_coordination"));
        assert!(scenarios.contains(&"emergency_response"));

        let scenario = library.get("single_agent_exploration").unwrap();
        assert_eq!(scenario.agents.len(), 1);
        assert_eq!(scenario.tasks.len(), 1);
    }
}
