//! Model representation for federated learning.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Neural network layer type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayerType {
    /// Fully connected layer.
    Dense(usize),
    /// Convolutional layer.
    Conv2D {
        filters: usize,
        kernel_size: (usize, usize),
        stride: (usize, usize),
    },
    /// Recurrent layer.
    LSTM(usize),
    /// Dropout layer.
    Dropout(f64),
    /// Batch normalization.
    BatchNorm,
}

/// A neural network layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// Layer type.
    pub layer_type: LayerType,
    /// Activation function.
    pub activation: Option<String>,
    /// Whether the layer is trainable.
    pub trainable: bool,
}

/// A machine learning model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model name.
    pub name: String,
    /// Layers.
    pub layers: Vec<Layer>,
    /// Total number of parameters.
    pub parameter_count: usize,
    /// Model version.
    pub version: u64,
    /// Metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Model {
    /// Create a new model.
    pub fn new(name: &str, layers: Vec<Layer>) -> Self {
        let parameter_count = Self::compute_parameter_count(&layers);
        Self {
            name: name.to_string(),
            layers,
            parameter_count,
            version: 1,
            metadata: HashMap::new(),
        }
    }

    /// Compute total number of parameters.
    fn compute_parameter_count(layers: &[Layer]) -> usize {
        // Simplified calculation.
        layers.iter().map(|layer| match &layer.layer_type {
            LayerType::Dense(size) => *size,
            LayerType::Conv2D { filters, kernel_size, .. } => filters * kernel_size.0 * kernel_size.1,
            LayerType::LSTM(size) => 4 * size * size, // approximate
            _ => 0,
        }).sum()
    }

    /// Serialize model parameters to a flat vector.
    pub fn parameters_to_vector(&self) -> Vec<f64> {
        // Placeholder: return dummy parameters.
        vec![0.0; self.parameter_count]
    }

    /// Update model parameters from a flat vector.
    pub fn update_from_vector(&mut self, parameters: &[f64]) -> Result<(), String> {
        if parameters.len() != self.parameter_count {
            return Err(format!(
                "Parameter vector length {} does not match model parameter count {}",
                parameters.len(),
                self.parameter_count
            ));
        }
        // In a real implementation, you would assign parameters to each layer.
        Ok(())
    }

    /// Increment version.
    pub fn bump_version(&mut self) {
        self.version += 1;
    }
}

/// Model registry for managing multiple models.
pub struct ModelRegistry {
    models: HashMap<String, Model>,
}

impl ModelRegistry {
    /// Create a new model registry.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Register a model.
    pub fn register(&mut self, model: Model) -> Result<(), String> {
        if self.models.contains_key(&model.name) {
            return Err(format!("Model '{}' already registered", model.name));
        }
        self.models.insert(model.name.clone(), model);
        Ok(())
    }

    /// Get a model by name.
    pub fn get(&self, name: &str) -> Option<&Model> {
        self.models.get(name)
    }

    /// Get mutable model.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Model> {
        self.models.get_mut(name)
    }

    /// Remove a model.
    pub fn remove(&mut self, name: &str) -> Option<Model> {
        self.models.remove(name)
    }

    /// List all model names.
    pub fn list(&self) -> Vec<String> {
        self.models.keys().cloned().collect()
    }
}