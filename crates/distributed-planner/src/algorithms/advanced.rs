//! Advanced planning algorithms for multi-objective optimization.

use super::{PlanningAlgorithm, Task, Assignment};
use common::types::{AgentId, Capability};
use std::collections::{HashMap, HashSet};
use anyhow::Result;

/// Multi-objective planner that optimizes multiple criteria simultaneously.
/// 
/// Uses weighted scoring to balance:
/// - Task priority
/// - Deadline urgency
/// - Resource efficiency
/// - Load balancing
pub struct MultiObjectivePlanner {
    weights: MultiObjectiveWeights,
    agent_resources: HashMap<AgentId, HashMap<String, f64>>,
    agent_capabilities: HashMap<AgentId, HashSet<Capability>>,
    agent_loads: HashMap<AgentId, f64>,
}

#[derive(Debug, Clone)]
pub struct MultiObjectiveWeights {
    pub priority_weight: f64,
    pub deadline_weight: f64,
    pub efficiency_weight: f64,
    pub load_balance_weight: f64,
    pub capability_match_weight: f64,
}

impl Default for MultiObjectiveWeights {
    fn default() -> Self {
        Self {
            priority_weight: 0.3,
            deadline_weight: 0.25,
            efficiency_weight: 0.2,
            load_balance_weight: 0.15,
            capability_match_weight: 0.1,
        }
    }
}

impl MultiObjectivePlanner {
    pub fn new(
        weights: MultiObjectiveWeights,
        agent_resources: HashMap<AgentId, HashMap<String, f64>>,
        agent_capabilities: HashMap<AgentId, HashSet<Capability>>,
    ) -> Self {
        let agent_loads = agent_resources.keys().map(|&id| (id, 0.0)).collect();
        Self {
            weights,
            agent_resources,
            agent_capabilities,
            agent_loads,
        }
    }

    /// Update agent loads based on current assignments
    pub fn update_loads(&mut self, current_assignments: &[Assignment]) {
        self.agent_loads.clear();
        
        for assignment in current_assignments {
            if assignment.status == AssignmentStatus::InProgress 
                || assignment.status == AssignmentStatus::Assigned {
                *self.agent_loads.entry(assignment.agent_id).or_insert(0.0) += 1.0;
            }
        }
    }

    fn calculate_score(&self, task: &Task, agent: AgentId) -> f64 {
        let mut score = 0.0;

        // Priority score (higher priority = higher score)
        let priority_score = task.priority as f64 / 255.0;
        score += priority_score * self.weights.priority_weight;

        // Deadline urgency score
        if let Some(deadline) = task.deadline {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as f64;
            let time_to_deadline = (deadline as f64) - now;
            let urgency = if time_to_deadline > 0.0 {
                1.0 / (1.0 + time_to_deadline / 3600.0) // Normalize to hours
            } else {
                2.0 // Already overdue
            };
            score += urgency * self.weights.deadline_weight;
        }

        // Resource efficiency score
        let efficiency_score = self.calculate_efficiency_score(task, agent);
        score += efficiency_score * self.weights.efficiency_weight;

        // Load balancing score
        let load_score = self.calculate_load_score(agent);
        score += load_score * self.weights.load_balance_weight;

        // Capability match score
        let capability_score = self.calculate_capability_score(task, agent);
        score += capability_score * self.weights.capability_match_weight;

        score
    }

    fn calculate_efficiency_score(&self, task: &Task, agent: AgentId) -> f64 {
        if let Some(resources) = self.agent_resources.get(&agent) {
            let mut total_match = 0.0;
            let mut total_required = 0.0;

            for req in &task.required_resources {
                if let Some(&available) = resources.get(req) {
                    total_match += available.min(100.0) / 100.0;
                }
                total_required += 1.0;
            }

            if total_required > 0.0 {
                total_match / total_required
            } else {
                1.0
            }
        } else {
            0.0
        }
    }

    fn calculate_load_score(&self, agent: AgentId) -> f64 {
        let load = *self.agent_loads.get(&agent).unwrap_or(&0.0);
        // Exponential decay for load
        1.0 / (1.0 + load * 0.5)
    }

    fn calculate_capability_score(&self, task: &Task, agent: AgentId) -> f64 {
        if let Some(caps) = self.agent_capabilities.get(&agent) {
            if task.required_capabilities.is_empty() {
                1.0
            } else {
                let matched = task.required_capabilities.iter()
                    .filter(|c| caps.contains(*c))
                    .count();
                matched as f64 / task.required_capabilities.len() as f64
            }
        } else {
            0.0
        }
    }
}

#[async_trait::async_trait]
impl PlanningAlgorithm for MultiObjectivePlanner {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        // Create a mutable copy to update loads (in real impl, this would be shared state)
        let mut planner = MultiObjectivePlanner {
            weights: self.weights.clone(),
            agent_resources: self.agent_resources.clone(),
            agent_capabilities: self.agent_capabilities.clone(),
            agent_loads: self.agent_loads.clone(),
        };
        planner.update_loads(&current_assignments);

        let mut assignments = Vec::new();
        let mut agent_loads = HashMap::new();

        // Sort tasks by priority and deadline
        let mut sorted_tasks = tasks;
        sorted_tasks.sort_by(|a, b| {
            let a_score = a.priority as i32 
                + a.deadline.map(|d| -((d % 1000) as i32)).unwrap_or(0);
            let b_score = b.priority as i32 
                + b.deadline.map(|d| -((d % 1000) as i32)).unwrap_or(0);
            b_score.cmp(&a_score)
        });

        for task in sorted_tasks {
            let mut best_agent = None;
            let mut best_score = f64::NEG_INFINITY;

            for &agent in &agents {
                let score = planner.calculate_score(&task, agent);
                if score > best_score {
                    best_score = score;
                    best_agent = Some(agent);
                }
                *agent_loads.entry(agent).or_insert(0.0) += 1.0;
            }

            if let Some(agent) = best_agent {
                let mut assignment = task.create_assignment(agent);
                assignment.dependencies_satisfied = task.dependencies.is_empty();
                assignments.push(assignment);
            }
        }

        Ok(assignments)
    }

    fn name(&self) -> &'static str {
        "multi_objective"
    }
}

/// Reinforcement learning-based planner that learns optimal assignment policies.
/// 
/// Note: This is a simplified version. In production, would use proper RL framework.
pub struct RLPlanner {
    q_table: HashMap<(String, AgentId), f64>,
    learning_rate: f64,
    discount_factor: f64,
    exploration_rate: f64,
}

impl RLPlanner {
    pub fn new(learning_rate: f64, discount_factor: f64, exploration_rate: f64) -> Self {
        Self {
            q_table: HashMap::new(),
            learning_rate,
            discount_factor,
            exploration_rate,
        }
    }

    fn get_q_value(&self, task_id: &str, agent: AgentId) -> f64 {
        *self.q_table.get(&(task_id.to_string(), agent)).unwrap_or(&0.0)
    }

    fn update_q_value(&mut self, task_id: &str, agent: AgentId, reward: f64) {
        let current_q = self.get_q_value(task_id, agent);
        let new_q = current_q + self.learning_rate * (reward - current_q);
        self.q_table.insert((task_id.to_string(), agent), new_q);
    }

    /// Report completion reward for learning
    pub fn report_completion(&mut self, task_id: &str, agent: AgentId, success: bool) {
        let reward = if success { 1.0 } else { -1.0 };
        self.update_q_value(task_id, agent, reward);
    }
}

#[async_trait::async_trait]
impl PlanningAlgorithm for RLPlanner {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        _current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        let mut assignments = Vec::new();
        let agents: Vec<AgentId> = agents.into_iter().collect();

        if agents.is_empty() {
            return Ok(assignments);
        }

        for task in tasks {
            // Exploration vs exploitation
            if rand::random::<f64>() < self.exploration_rate {
                // Random exploration
                let agent = agents[rand::random::<usize>() % agents.len()];
                assignments.push(task.create_assignment(agent));
            } else {
                // Exploitation based on Q-values
                let mut best_agent = None;
                let mut best_q = f64::NEG_INFINITY;

                for &agent in &agents {
                    let q_value = self.get_q_value(&task.id, agent);
                    if q_value > best_q {
                        best_q = q_value;
                        best_agent = Some(agent);
                    }
                }

                if let Some(agent) = best_agent {
                    assignments.push(task.create_assignment(agent));
                }
            }
        }

        Ok(assignments)
    }

    fn name(&self) -> &'static str {
        "rl_planner"
    }
}

/// Dynamic load-balancing planner that continuously adjusts assignments.
pub struct DynamicLoadBalancer {
    target_load: f64,
    rebalance_threshold: f64,
    agent_capacity: HashMap<AgentId, f64>,
}

impl DynamicLoadBalancer {
    pub fn new(target_load: f64, rebalance_threshold: f64) -> Self {
        Self {
            target_load,
            rebalance_threshold,
            agent_capacity: HashMap::new(),
        }
    }

    pub fn set_agent_capacity(&mut self, agent: AgentId, capacity: f64) {
        self.agent_capacity.insert(agent, capacity);
    }

    fn calculate_current_load(&self, assignments: &[Assignment], agent: AgentId) -> f64 {
        assignments.iter()
            .filter(|a| a.agent_id == agent && 
                   (a.status == AssignmentStatus::InProgress || 
                    a.status == AssignmentStatus::Assigned))
            .count() as f64
    }
}

#[async_trait::async_trait]
impl PlanningAlgorithm for DynamicLoadBalancer {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        let mut assignments = Vec::new();
        let agents: Vec<AgentId> = agents.into_iter().collect();

        if agents.is_empty() {
            return Ok(assignments);
        }

        // Find least loaded agents
        let mut agent_loads: Vec<(AgentId, f64)> = agents.iter()
            .map(|&agent| {
                let capacity = *self.agent_capacity.get(&agent).unwrap_or(&1.0);
                let current = self.calculate_current_load(&current_assignments, agent);
                let load_ratio = current / capacity;
                (agent, load_ratio)
            })
            .collect();

        agent_loads.sort_by_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Assign tasks to least loaded agents
        for task in tasks {
            if let Some(&(agent, _)) = agent_loads.first() {
                assignments.push(task.create_assignment(agent));
                
                // Update load
                for (a, load) in &mut agent_loads {
                    if *a == agent {
                        *load += 1.0;
                    }
                }
                agent_loads.sort_by_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            }
        }

        Ok(assignments)
    }

    fn name(&self) -> &'static str {
        "dynamic_load_balancer"
    }
}

/// Hybrid planner that combines multiple strategies based on context.
pub struct HybridPlanner {
    strategies: Vec<HybridStrategy>,
    context_weights: HashMap<PlannerContext, f64>,
}

#[derive(Debug, Clone)]
pub struct HybridStrategy {
    pub name: &'static str,
    pub weight: f64,
    pub conditions: Vec<StrategyCondition>,
}

#[derive(Debug, Clone)]
pub enum StrategyCondition {
    HighPriorityTaskRatio(f64),
    ManyTasksThreshold(u32),
    LowNetworkBandwidth,
    HighAgentCount(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlannerContext {
    Normal,
    HighLoad,
    LowResources,
    TimeCritical,
}

impl HybridPlanner {
    pub fn new(strategies: Vec<HybridStrategy>) -> Self {
        let context_weights = HashMap::new();
        Self {
            strategies,
            context_weights,
        }
    }

    fn detect_context(&self, tasks: &[Task], agents: usize) -> PlannerContext {
        let high_priority_ratio = tasks.iter()
            .filter(|t| t.priority > 200)
            .count() as f64 / tasks.len() as f64;

        if high_priority_ratio > 0.5 {
            PlannerContext::TimeCritical
        } else if tasks.len() > 100 {
            PlannerContext::HighLoad
        } else if agents < 3 {
            PlannerContext::LowResources
        } else {
            PlannerContext::Normal
        }
    }
}

#[async_trait::async_trait]
impl PlanningAlgorithm for HybridPlanner {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        let context = self.detect_context(&tasks, agents.len());
        
        // In a real implementation, this would combine multiple strategies
        // based on context weights. For now, we use a simple fallback.
        
        // Use multi-objective as default
        let mut result = Vec::new();
        let agents: Vec<_> = agents.into_iter().collect();
        
        for (i, task) in tasks.into_iter().enumerate() {
            let agent = agents[i % agents.len()];
            result.push(task.create_assignment(agent));
        }

        Ok(result)
    }

    fn name(&self) -> &'static str {
        "hybrid"
    }
}

// Re-export AssignmentStatus
use crate::AssignmentStatus;
