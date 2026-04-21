//! Predictive load balancing using time‑series forecasting.

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, Duration};
use chrono::{DateTime, Utc};
use crate::error::{LoadBalancingError, Result};
use crate::metrics::{AgentLoad, LoadMetrics};

/// Configuration for predictive load balancing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PredictiveConfig {
    /// Forecast horizon in seconds.
    pub forecast_horizon_secs: u64,
    /// History window size for training.
    pub history_window: usize,
    /// Update interval for predictions.
    pub update_interval_secs: u64,
    /// Confidence threshold for predictions (0.0 to 1.0).
    pub confidence_threshold: f64,
    /// Whether to enable trend detection.
    pub enable_trend_detection: bool,
    /// Whether to enable seasonality detection.
    pub enable_seasonality_detection: bool,
    /// Maximum number of prediction models to maintain.
    pub max_models: usize,
}

impl Default for PredictiveConfig {
    fn default() -> Self {
        Self {
            forecast_horizon_secs: 60, // 1 minute
            history_window: 100,
            update_interval_secs: 10,
            confidence_threshold: 0.7,
            enable_trend_detection: true,
            enable_seasonality_detection: false,
            max_models: 10,
        }
    }
}

/// Time‑series data point.
#[derive(Debug, Clone)]
struct TimeSeriesPoint {
    /// Timestamp.
    timestamp: DateTime<Utc>,
    /// Value.
    value: f64,
    /// Metadata.
    metadata: HashMap<String, f64>,
}

impl TimeSeriesPoint {
    fn new(value: f64) -> Self {
        Self {
            timestamp: Utc::now(),
            value,
            metadata: HashMap::new(),
        }
    }
    
    fn with_metadata(mut self, key: &str, value: f64) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

/// Simple moving average predictor.
struct MovingAveragePredictor {
    /// Window size.
    window_size: usize,
    /// History.
    history: VecDeque<f64>,
    /// Current prediction.
    current_prediction: Option<f64>,
}

impl MovingAveragePredictor {
    fn new(window_size: usize) -> Self {
        Self {
            window_size,
            history: VecDeque::with_capacity(window_size),
            current_prediction: None,
        }
    }
    
    fn update(&mut self, value: f64) {
        self.history.push_back(value);
        if self.history.len() > self.window_size {
            self.history.pop_front();
        }
        
        if !self.history.is_empty() {
            let sum: f64 = self.history.iter().sum();
            self.current_prediction = Some(sum / self.history.len() as f64);
        }
    }
    
    fn predict(&self) -> Option<f64> {
        self.current_prediction
    }
    
    fn confidence(&self) -> f64 {
        if self.history.len() < 2 {
            return 0.0;
        }
        
        // Simple confidence based on history size and variance
        let history_size = self.history.len() as f64;
        let max_size = self.window_size as f64;
        let size_confidence = history_size / max_size;
        
        // Calculate variance
        let mean = self.current_prediction.unwrap_or(0.0);
        let variance: f64 = self.history.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / history_size;
        
        let variance_confidence = 1.0 / (1.0 + variance.sqrt());
        
        (size_confidence * 0.3 + variance_confidence * 0.7).min(1.0)
    }
}

/// Exponential smoothing predictor.
struct ExponentialSmoothingPredictor {
    /// Alpha parameter (0.0 to 1.0).
    alpha: f64,
    /// Current smoothed value.
    smoothed: Option<f64>,
    /// Current prediction.
    prediction: Option<f64>,
    /// Number of observations.
    observations: usize,
}

impl ExponentialSmoothingPredictor {
    fn new(alpha: f64) -> Self {
        Self {
            alpha,
            smoothed: None,
            prediction: None,
            observations: 0,
        }
    }
    
    fn update(&mut self, value: f64) {
        self.observations += 1;
        
        if let Some(smoothed) = self.smoothed {
            self.smoothed = Some(self.alpha * value + (1.0 - self.alpha) * smoothed);
        } else {
            self.smoothed = Some(value);
        }
        
        self.prediction = self.smoothed;
    }
    
    fn predict(&self) -> Option<f64> {
        self.prediction
    }
    
    fn confidence(&self) -> f64 {
        if self.observations < 2 {
            return 0.0;
        }
        
        // Confidence increases with observations
        (self.observations as f64 / 50.0).min(1.0)
    }
}

/// Linear regression predictor.
struct LinearRegressionPredictor {
    /// Slope.
    slope: f64,
    /// Intercept.
    intercept: f64,
    /// Number of observations.
    observations: usize,
    /// Sum of x.
    sum_x: f64,
    /// Sum of y.
    sum_y: f64,
    /// Sum of x*y.
    sum_xy: f64,
    /// Sum of x^2.
    sum_x2: f64,
}

impl LinearRegressionPredictor {
    fn new() -> Self {
        Self {
            slope: 0.0,
            intercept: 0.0,
            observations: 0,
            sum_x: 0.0,
            sum_y: 0.0,
            sum_xy: 0.0,
            sum_x2: 0.0,
        }
    }
    
    fn update(&mut self, x: f64, y: f64) {
        self.observations += 1;
        self.sum_x += x;
        self.sum_y += y;
        self.sum_xy += x * y;
        self.sum_x2 += x * x;
        
        if self.observations >= 2 {
            let n = self.observations as f64;
            let denominator = n * self.sum_x2 - self.sum_x * self.sum_x;
            
            if denominator.abs() > 1e-10 {
                self.slope = (n * self.sum_xy - self.sum_x * self.sum_y) / denominator;
                self.intercept = (self.sum_y - self.slope * self.sum_x) / n;
            }
        }
    }
    
    fn predict(&self, x: f64) -> f64 {
        self.slope * x + self.intercept
    }
    
    fn confidence(&self) -> f64 {
        if self.observations < 3 {
            return 0.0;
        }
        
        // Calculate R-squared
        let n = self.observations as f64;
        let mean_y = self.sum_y / n;
        
        let mut ss_total = 0.0;
        let mut ss_residual = 0.0;
        
        // Simplified confidence calculation
        // In practice, we'd need to store all points for proper R² calculation
        let r_squared = if self.observations > 10 {
            0.8 // Placeholder
        } else {
            (self.observations as f64 / 20.0).min(0.7)
        };
        
        r_squared.min(1.0)
    }
}

/// Load prediction for an agent.
#[derive(Debug, Clone)]
pub struct LoadPrediction {
    /// Agent ID.
    pub agent_id: String,
    /// Predicted load (0.0 to 1.0).
    pub predicted_load: f64,
    /// Confidence in prediction (0.0 to 1.0).
    pub confidence: f64,
    /// Prediction horizon in seconds.
    pub horizon_secs: u64,
    /// Trend (positive = increasing, negative = decreasing).
    pub trend: f64,
    /// Timestamp of prediction.
    pub timestamp: SystemTime,
}

impl LoadPrediction {
    fn new(agent_id: &str, predicted_load: f64, confidence: f64, horizon_secs: u64) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            predicted_load: predicted_load.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
            horizon_secs,
            trend: 0.0,
            timestamp: SystemTime::now(),
        }
    }
    
    /// Check if prediction is reliable.
    pub fn is_reliable(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }
    
    /// Check if agent is predicted to be overloaded.
    pub fn is_predicted_overloaded(&self, threshold: f64) -> bool {
        self.predicted_load > threshold
    }
}

/// Predictive load balancer.
pub struct PredictiveLoadBalancer {
    /// Configuration.
    config: PredictiveConfig,
    /// Moving average predictors per agent.
    ma_predictors: HashMap<String, MovingAveragePredictor>,
    /// Exponential smoothing predictors per agent.
    es_predictors: HashMap<String, ExponentialSmoothingPredictor>,
    /// Linear regression predictors per agent.
    lr_predictors: HashMap<String, LinearRegressionPredictor>,
    /// Current predictions.
    predictions: HashMap<String, LoadPrediction>,
    /// Update timer.
    last_update: SystemTime,
}

impl PredictiveLoadBalancer {
    /// Create a new predictive load balancer.
    pub fn new(config: PredictiveConfig) -> Self {
        Self {
            config: config.clone(),
            ma_predictors: HashMap::new(),
            es_predictors: HashMap::new(),
            lr_predictors: HashMap::new(),
            predictions: HashMap::new(),
            last_update: SystemTime::now(),
        }
    }
    
    /// Create with default configuration.
    pub fn with_default_config() -> Self {
        Self::new(PredictiveConfig::default())
    }
    
    /// Update with new load data.
    pub fn update_load(&mut self, agent_id: &str, load: f64) {
        let timestamp = SystemTime::now();
        
        // Update moving average predictor
        let ma_predictor = self.ma_predictors
            .entry(agent_id.to_string())
            .or_insert_with(|| MovingAveragePredictor::new(self.config.history_window));
        ma_predictor.update(load);
        
        // Update exponential smoothing predictor
        let es_predictor = self.es_predictors
            .entry(agent_id.to_string())
            .or_insert_with(|| ExponentialSmoothingPredictor::new(0.3));
        es_predictor.update(load);
        
        // Update linear regression predictor (using time as x)
        let time_x = timestamp.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let lr_predictor = self.lr_predictors
            .entry(agent_id.to_string())
            .or_insert_with(LinearRegressionPredictor::new);
        lr_predictor.update(time_x, load);
        
        self.last_update = timestamp;
    }
    
    /// Update with AgentLoad.
    pub fn update_agent_load(&mut self, agent_id: &str, agent_load: &AgentLoad) {
        self.update_load(agent_id, agent_load.total_load());
    }
    
    /// Generate predictions for all agents.
    pub fn generate_predictions(&mut self) {
        self.predictions.clear();
        
        let future_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64() + self.config.forecast_horizon_secs as f64;
        
        for agent_id in self.ma_predictors.keys() {
            if let Some(prediction) = self.predict_agent(agent_id, future_time) {
                self.predictions.insert(agent_id.clone(), prediction);
            }
        }
    }
    
    /// Get prediction for an agent.
    pub fn get_prediction(&self, agent_id: &str) -> Option<&LoadPrediction> {
        self.predictions.get(agent_id)
    }
    
    /// Get all predictions.
    pub fn get_all_predictions(&self) -> &HashMap<String, LoadPrediction> {
        &self.predictions
    }
    
    /// Find the agent with lowest predicted load.
    pub fn find_best_agent(&self) -> Option<&LoadPrediction> {
        self.predictions.values()
            .filter(|p| p.is_reliable(self.config.confidence_threshold))
            .min_by(|a, b| {
                a.predicted_load.partial_cmp(&b.predicted_load).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
    
    /// Find agents predicted to be overloaded.
    pub fn find_predicted_overloaded(&self, threshold: f64) -> Vec<&LoadPrediction> {
        self.predictions.values()
            .filter(|p| p.is_reliable(self.config.confidence_threshold))
            .filter(|p| p.is_predicted_overloaded(threshold))
            .collect()
    }
    
    /// Check if predictions need updating.
    pub fn needs_update(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_update)
            .unwrap_or_default()
            .as_secs() >= self.config.update_interval_secs
    }
    
    fn predict_agent(&self, agent_id: &str, future_time: f64) -> Option<LoadPrediction> {
        let ma_pred = self.ma_predictors.get(agent_id)?.predict()?;
        let ma_conf = self.ma_predictors.get(agent_id)?.confidence();
        
        let es_pred = self.es_predictors.get(agent_id)?.predict()?;
        let es_conf = self.es_predictors.get(agent_id)?.confidence();
        
        let lr_pred = self.lr_predictors.get(agent_id)?.predict(future_time);
        let lr_conf = self.lr_predictors.get(agent_id)?.confidence();
        
        // Weighted average based on confidence
        let total_conf = ma_conf + es_conf + lr_conf;
        if total_conf < 1e-10 {
            return None;
        }
        
        let weighted_pred = (ma_pred * ma_conf + es_pred * es_conf + lr_pred * lr_conf) / total_conf;
        let avg_confidence = (ma_conf + es_conf + lr_conf) / 3.0;
        
        // Calculate trend (difference between LR prediction and current)
        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let current_pred = self.lr_predictors.get(agent_id)?.predict(current_time);
        let trend = lr_pred - current_pred;
        
        let mut prediction = LoadPrediction::new(
            agent_id,
            weighted_pred,
            avg_confidence,
            self.config.forecast_horizon_secs,
        );
        prediction.trend = trend;
        
        Some(prediction)
    }
}

/// Predictive load balancing strategy.
pub struct PredictiveLoadBalancingStrategy {
    /// Predictive model.
    predictor: PredictiveLoadBalancer,
    /// Fallback strategy (used when predictions are unreliable).
    fallback_strategy: crate::strategy::LoadBalancingStrategy,
    /// Overload threshold.
    overload_threshold: f64,
}

impl PredictiveLoadBalancingStrategy {
    /// Create a new predictive strategy.
    pub fn new(
        predictor: PredictiveLoadBalancer,
        fallback_strategy: crate::strategy::LoadBalancingStrategy,
        overload_threshold: f64,
    ) -> Self {
        Self {
            predictor,
            fallback_strategy,
            overload_threshold,
        }
    }
    
    /// Select an agent using predictive load balancing.
    pub fn select_agent(&mut self, agent_loads: &HashMap<String, AgentLoad>) -> Result<String> {
        // Update predictor with current loads
        for (agent_id, load) in agent_loads {
            self.predictor.update_agent_load(agent_id, load);
        }
        
        // Generate predictions if needed
        if self.predictor.needs_update() {
            self.predictor.generate_predictions();
        }
        
        // Try to use prediction
        if let Some(best_prediction) = self.predictor.find_best_agent() {
            if best_prediction.is_reliable(self.predictor.config.confidence_threshold) {
                return Ok(best_prediction.agent_id.clone());
            }
        }
        
        // Fall back to least loaded
        let (agent_id, _) = agent_loads
            .iter()
            .min_by(|(_, a), (_, b)| {
                a.total_load().partial_cmp(&b.total_load()).unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or(LoadBalancingError::NoAgentsAvailable)?;
        
        Ok(agent_id.clone())
    }
    
    /// Get the predictor.
    pub fn predictor(&self) -> &PredictiveLoadBalancer {
        &self.predictor
    }
    
    /// Get mutable predictor.
    pub fn predictor_mut(&mut self) -> &mut PredictiveLoadBalancer {
        &mut self.predictor
    }
}