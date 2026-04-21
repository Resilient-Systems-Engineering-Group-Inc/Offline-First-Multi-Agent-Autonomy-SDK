//! Adaptive load balancing algorithms.

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, Duration};
use rand::Rng;
use crate::error::{LoadBalancingError, Result};
use crate::metrics::{AgentLoad, LoadMetrics};
use crate::strategy::{LoadBalancingStrategy, LoadBalancer};

/// Configuration for adaptive load balancing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdaptiveConfig {
    /// Learning rate for weight updates (0.0 to 1.0).
    pub learning_rate: f64,
    /// Exploration rate for trying new strategies (0.0 to 1.0).
    pub exploration_rate: f64,
    /// Decay factor for exploration rate.
    pub exploration_decay: f64,
    /// Minimum exploration rate.
    pub min_exploration_rate: f64,
    /// Window size for performance history.
    pub history_window: usize,
    /// Reward scaling factor.
    pub reward_scale: f64,
    /// Whether to enable strategy switching.
    pub enable_strategy_switching: bool,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            exploration_rate: 0.3,
            exploration_decay: 0.995,
            min_exploration_rate: 0.01,
            history_window: 100,
            reward_scale: 1.0,
            enable_strategy_switching: true,
        }
    }
}

/// Performance record for a strategy.
#[derive(Debug, Clone)]
struct StrategyPerformance {
    /// Strategy identifier.
    strategy: LoadBalancingStrategy,
    /// Total reward accumulated.
    total_reward: f64,
    /// Number of times used.
    usage_count: usize,
    /// Average reward.
    average_reward: f64,
    /// Recent rewards (for moving average).
    recent_rewards: VecDeque<f64>,
}

impl StrategyPerformance {
    fn new(strategy: LoadBalancingStrategy) -> Self {
        Self {
            strategy,
            total_reward: 0.0,
            usage_count: 0,
            average_reward: 0.0,
            recent_rewards: VecDeque::new(),
        }
    }

    fn update(&mut self, reward: f64, window_size: usize) {
        self.total_reward += reward;
        self.usage_count += 1;
        
        self.recent_rewards.push_back(reward);
        if self.recent_rewards.len() > window_size {
            self.recent_rewards.pop_front();
        }
        
        // Update moving average
        let sum: f64 = self.recent_rewards.iter().sum();
        self.average_reward = sum / self.recent_rewards.len() as f64;
    }

    fn get_reward(&self) -> f64 {
        self.average_reward
    }
}

/// Adaptive load balancer that learns from experience.
pub struct AdaptiveLoadBalancer {
    /// Base load balancer.
    base_balancer: LoadBalancer,
    /// Configuration.
    config: AdaptiveConfig,
    /// Strategy performances.
    strategy_performances: HashMap<LoadBalancingStrategy, StrategyPerformance>,
    /// Current strategy.
    current_strategy: LoadBalancingStrategy,
    /// Performance history.
    performance_history: VecDeque<(LoadBalancingStrategy, f64)>,
    /// Exploration rate.
    exploration_rate: f64,
    /// Random number generator.
    rng: rand::rngs::ThreadRng,
}

impl AdaptiveLoadBalancer {
    /// Create a new adaptive load balancer.
    pub fn new(base_balancer: LoadBalancer, config: AdaptiveConfig) -> Self {
        let strategies = vec![
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::LeastLoaded,
            LoadBalancingStrategy::WeightedRoundRobin,
            LoadBalancingStrategy::ConsistentHashing,
        ];
        
        let mut strategy_performances = HashMap::new();
        for &strategy in &strategies {
            strategy_performances.insert(strategy, StrategyPerformance::new(strategy));
        }
        
        let current_strategy = LoadBalancingStrategy::LeastLoaded;
        
        Self {
            base_balancer,
            config: config.clone(),
            strategy_performances,
            current_strategy,
            performance_history: VecDeque::with_capacity(config.history_window),
            exploration_rate: config.exploration_rate,
            rng: rand::thread_rng(),
        }
    }

    /// Create with default configuration.
    pub fn with_default_config(base_balancer: LoadBalancer) -> Self {
        Self::new(base_balancer, AdaptiveConfig::default())
    }

    /// Select an agent using adaptive strategy.
    pub async fn select_agent(&mut self) -> Result<String> {
        // Decide whether to explore or exploit
        let strategy = if self.rng.gen::<f64>() < self.exploration_rate && self.config.enable_strategy_switching {
            self.explore_strategy()
        } else {
            self.exploit_strategy()
        };
        
        // Set strategy in base balancer
        self.base_balancer.set_strategy(strategy);
        self.current_strategy = strategy;
        
        // Select agent using chosen strategy
        self.base_balancer.select_agent().await
    }

    /// Update with performance feedback.
    pub async fn update_feedback(&mut self, agent_id: &str, performance: &PerformanceFeedback) -> Result<()> {
        // Calculate reward
        let reward = self.calculate_reward(performance);
        
        // Update strategy performance
        if let Some(strategy_perf) = self.strategy_performances.get_mut(&self.current_strategy) {
            strategy_perf.update(reward, self.config.history_window);
        }
        
        // Update performance history
        self.performance_history.push_back((self.current_strategy, reward));
        if self.performance_history.len() > self.config.history_window {
            self.performance_history.pop_front();
        }
        
        // Decay exploration rate
        self.exploration_rate = (self.exploration_rate * self.config.exploration_decay)
            .max(self.config.min_exploration_rate);
        
        // Update agent load in base balancer
        if let Some(load) = &performance.agent_load {
            self.base_balancer.update_agent_load(agent_id, load.clone()).await?;
        }
        
        Ok(())
    }

    /// Get the best strategy based on current knowledge.
    pub fn get_best_strategy(&self) -> LoadBalancingStrategy {
        self.strategy_performances
            .iter()
            .max_by(|(_, a), (_, b)| {
                a.get_reward().partial_cmp(&b.get_reward()).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(strategy, _)| *strategy)
            .unwrap_or(LoadBalancingStrategy::LeastLoaded)
    }

    /// Get strategy performances.
    pub fn get_strategy_performances(&self) -> &HashMap<LoadBalancingStrategy, StrategyPerformance> {
        &self.strategy_performances
    }

    /// Get current exploration rate.
    pub fn exploration_rate(&self) -> f64 {
        self.exploration_rate
    }

    /// Reset learning state.
    pub fn reset(&mut self) {
        self.exploration_rate = self.config.exploration_rate;
        self.performance_history.clear();
        
        for strategy_perf in self.strategy_performances.values_mut() {
            strategy_perf.total_reward = 0.0;
            strategy_perf.usage_count = 0;
            strategy_perf.average_reward = 0.0;
            strategy_perf.recent_rewards.clear();
        }
    }

    fn explore_strategy(&mut self) -> LoadBalancingStrategy {
        let strategies: Vec<LoadBalancingStrategy> = self.strategy_performances.keys().cloned().collect();
        let idx = self.rng.gen_range(0..strategies.len());
        strategies[idx]
    }

    fn exploit_strategy(&mut self) -> LoadBalancingStrategy {
        self.get_best_strategy()
    }

    fn calculate_reward(&self, feedback: &PerformanceFeedback) -> f64 {
        let mut reward = 0.0;
        
        // Positive reward for low response time
        if feedback.response_time_ms > 0.0 {
            reward += 1.0 / (feedback.response_time_ms / 1000.0).max(0.001);
        }
        
        // Positive reward for success
        if feedback.success {
            reward += 10.0;
        } else {
            reward -= 5.0;
        }
        
        // Positive reward for load balance (if we have metrics)
        if let Some(load) = &feedback.agent_load {
            if !load.is_overloaded(0.8) {
                reward += 5.0;
            } else {
                reward -= 3.0;
            }
        }
        
        reward * self.config.reward_scale
    }
}

/// Performance feedback for adaptive learning.
#[derive(Debug, Clone)]
pub struct PerformanceFeedback {
    /// Whether the task was successful.
    pub success: bool,
    /// Response time in milliseconds.
    pub response_time_ms: f64,
    /// Agent load after task completion.
    pub agent_load: Option<AgentLoad>,
    /// Error message if any.
    pub error_message: Option<String>,
    /// Task complexity score (0.0 to 1.0).
    pub task_complexity: f64,
    /// Timestamp.
    pub timestamp: SystemTime,
}

impl PerformanceFeedback {
    /// Create new performance feedback.
    pub fn new(success: bool, response_time_ms: f64) -> Self {
        Self {
            success,
            response_time_ms,
            agent_load: None,
            error_message: None,
            task_complexity: 0.5,
            timestamp: SystemTime::now(),
        }
    }
    
    /// Create successful feedback.
    pub fn success(response_time_ms: f64) -> Self {
        Self::new(true, response_time_ms)
    }
    
    /// Create failed feedback.
    pub fn failure(error_message: &str) -> Self {
        Self::new(false, 0.0)
            .with_error_message(error_message)
    }
    
    /// Add agent load.
    pub fn with_agent_load(mut self, agent_load: AgentLoad) -> Self {
        self.agent_load = Some(agent_load);
        self
    }
    
    /// Add error message.
    pub fn with_error_message(mut self, error_message: &str) -> Self {
        self.error_message = Some(error_message.to_string());
        self
    }
    
    /// Add task complexity.
    pub fn with_task_complexity(mut self, task_complexity: f64) -> Self {
        self.task_complexity = task_complexity.clamp(0.0, 1.0);
        self
    }
}

/// Adaptive weight adjustment for weighted strategies.
pub struct AdaptiveWeightAdjuster {
    /// Agent weights.
    weights: HashMap<String, f64>,
    /// Performance history per agent.
    performance_history: HashMap<String, VecDeque<f64>>,
    /// Learning rate.
    learning_rate: f64,
    /// Window size.
    window_size: usize,
}

impl AdaptiveWeightAdjuster {
    /// Create a new weight adjuster.
    pub fn new(learning_rate: f64, window_size: usize) -> Self {
        Self {
            weights: HashMap::new(),
            performance_history: HashMap::new(),
            learning_rate,
            window_size,
        }
    }
    
    /// Update weights based on performance.
    pub fn update(&mut self, agent_id: &str, performance: f64) {
        // Initialize if needed
        if !self.weights.contains_key(agent_id) {
            self.weights.insert(agent_id.to_string(), 1.0);
            self.performance_history.insert(agent_id.to_string(), VecDeque::new());
        }
        
        // Update performance history
        let history = self.performance_history.get_mut(agent_id).unwrap();
        history.push_back(performance);
        if history.len() > self.window_size {
            history.pop_front();
        }
        
        // Calculate average performance
        let avg_performance: f64 = history.iter().sum::<f64>() / history.len() as f64;
        
        // Update weight: better performance -> higher weight
        let weight = self.weights.get_mut(agent_id).unwrap();
        *weight = (*weight * (1.0 - self.learning_rate)) + (avg_performance * self.learning_rate);
        
        // Ensure weight is positive
        *weight = weight.max(0.1);
    }
    
    /// Get current weights.
    pub fn get_weights(&self) -> &HashMap<String, f64> {
        &self.weights
    }
    
    /// Get weight for specific agent.
    pub fn get_weight(&self, agent_id: &str) -> f64 {
        self.weights.get(agent_id).copied().unwrap_or(1.0)
    }
    
    /// Reset all weights.
    pub fn reset(&mut self) {
        self.weights.clear();
        self.performance_history.clear();
    }
}