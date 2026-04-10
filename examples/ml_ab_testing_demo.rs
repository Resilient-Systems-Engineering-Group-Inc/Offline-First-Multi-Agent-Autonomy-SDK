//! Demonstration of ML model versioning with A/B testing.
//!
//! This example shows how to:
//! 1. Create a model and register multiple versions
//! 2. Set up an A/B experiment between two versions
//! 3. Simulate traffic and collect metrics
//! 4. Analyze results and determine the winning variant

use ml_model_versioning::{
    ABExperimentConfig, ABTestingManager, ModelMetadata, ModelVersioningConfig,
    ModelVersioningManager, Observation, TrafficAllocationStrategy, Variant,
};
use ml_model_versioning::storage::InMemoryStorage;
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ML Model A/B Testing Demo ===");
    
    // 1. Create model versioning manager
    let storage = InMemoryStorage::new();
    let config = ModelVersioningConfig::default();
    let manager = Arc::new(ModelVersioningManager::new(storage, config));
    
    // 2. Create a model
    println!("Creating model 'sentiment-classifier'...");
    let metadata = ModelMetadata {
        id: "sentiment-classifier".to_string(),
        name: "Sentiment Classifier".to_string(),
        description: "Classifies text as positive/negative sentiment".to_string(),
        model_type: "neural_network".to_string(),
        framework: "pytorch".to_string(),
        input_schema: None,
        output_schema: None,
        tags: vec!["nlp".to_string(), "classification".to_string()],
        custom_metadata: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    
    manager.create_model(metadata).await?;
    
    // 3. Register two versions (control and treatment)
    println!("Registering version v1.0.0 (control)...");
    let control_request = ml_model_versioning::CreateVersionRequest {
        model_id: "sentiment-classifier".to_string(),
        version: "v1.0.0".to_string(),
        semver: Some(semver::Version::parse("1.0.0")?),
        changelog: "Initial version with BERT base".to_string(),
        data: vec![0x01, 0x02, 0x03, 0x04], // Simulated model weights
        dependencies: Vec::new(),
        metrics: [("accuracy".to_string(), 0.89)].into_iter().collect(),
        hyperparameters: [("learning_rate".to_string(), serde_json::json!(0.001))]
            .into_iter()
            .collect(),
        training_data: None,
        set_as_default: true,
        author: "alice".to_string(),
    };
    
    let control_version = manager.register_version(control_request).await?;
    println!("  Registered: {} (accuracy: {})", 
        control_version.version, 
        control_version.metrics.get("accuracy").unwrap()
    );
    
    println!("Registering version v1.1.0 (treatment)...");
    let treatment_request = ml_model_versioning::CreateVersionRequest {
        model_id: "sentiment-classifier".to_string(),
        version: "v1.1.0".to_string(),
        semver: Some(semver::Version::parse("1.1.0")?),
        changelog: "Improved with RoBERTa and better tokenization".to_string(),
        data: vec![0x05, 0x06, 0x07, 0x08], // Simulated model weights
        dependencies: Vec::new(),
        metrics: [("accuracy".to_string(), 0.92)].into_iter().collect(),
        hyperparameters: [("learning_rate".to_string(), serde_json::json!(0.0005))]
            .into_iter()
            .collect(),
        training_data: None,
        set_as_default: false,
        author: "bob".to_string(),
    };
    
    let treatment_version = manager.register_version(treatment_request).await?;
    println!("  Registered: {} (accuracy: {})", 
        treatment_version.version, 
        treatment_version.metrics.get("accuracy").unwrap()
    );
    
    // 4. Create A/B testing manager
    println!("\nSetting up A/B experiment...");
    let mut ab_manager = ABTestingManager::new(manager.clone());
    
    let experiment_config = ABExperimentConfig {
        experiment_id: "exp-sentiment-v2".to_string(),
        name: "Sentiment Classifier v2 Rollout".to_string(),
        description: "Test RoBERTa-based model against BERT baseline".to_string(),
        model_id: "sentiment-classifier".to_string(),
        control_version: "v1.0.0".to_string(),
        treatment_version: "v1.1.0".to_string(),
        treatment_traffic: 0.5, // 50% traffic to treatment
        primary_metric: "accuracy".to_string(),
        secondary_metrics: vec!["inference_latency".to_string()],
        min_sample_size: 100,
        significance_level: 0.05, // 95% confidence
        is_active: true,
        started_at: Utc::now(),
        ended_at: None,
        custom_metadata: HashMap::new(),
    };
    
    ab_manager.create_experiment(experiment_config).await?;
    println!("  Experiment created: exp-sentiment-v2");
    
    // 5. Simulate traffic and collect observations
    println!("\nSimulating traffic (200 requests)...");
    let strategy = TrafficAllocationStrategy::Random;
    
    for i in 0..200 {
        // Assign variant for this request
        let (variant, version) = ab_manager.assign_variant(
            "exp-sentiment-v2",
            &strategy,
            None,
        )?;
        
        // Simulate inference and collect metrics
        // In real scenario, you would run actual inference and measure performance
        let accuracy = match variant {
            Variant::Control => {
                // Control version (v1.0.0) - slightly lower accuracy with some noise
                0.89 + (rand::random::<f64>() - 0.5) * 0.05
            }
            Variant::Treatment => {
                // Treatment version (v1.1.0) - higher accuracy with some noise
                0.92 + (rand::random::<f64>() - 0.5) * 0.05
            }
        };
        
        let latency = match variant {
            Variant::Control => 50.0 + rand::random::<f64>() * 20.0, // ms
            Variant::Treatment => 55.0 + rand::random::<f64>() * 20.0, // ms (slightly slower)
        };
        
        // Record observation
        let observation = Observation {
            observation_id: uuid::Uuid::new_v4(),
            experiment_id: "exp-sentiment-v2".to_string(),
            variant,
            model_version: version,
            timestamp: Utc::now(),
            primary_metric_value: accuracy,
            secondary_metrics: [("inference_latency".to_string(), latency)]
                .into_iter()
                .collect(),
            context_features: None,
            custom_metadata: HashMap::new(),
        };
        
        ab_manager.record_observation(observation)?;
        
        if i % 40 == 0 {
            print!(".");
        }
    }
    println!("\n  Traffic simulation complete.");
    
    // 6. Analyze experiment results
    println!("\nAnalyzing experiment results...");
    let result = ab_manager.analyze_experiment("exp-sentiment-v2")?;
    
    println!("  Total observations: {}", result.total_observations);
    println!("  Control (v1.0.0): {} obs, mean accuracy: {:.4}", 
        result.control_observations, result.control_mean);
    println!("  Treatment (v1.1.0): {} obs, mean accuracy: {:.4}", 
        result.treatment_observations, result.treatment_mean);
    println!("  Difference: {:.4} ({:.2}% improvement)", 
        result.mean_difference, result.relative_improvement);
    
    if let Some((lower, upper)) = result.confidence_interval {
        println!("  95% CI for difference: [{:.4}, {:.4}]", lower, upper);
    }
    
    match result.statistical_test {
        Some(ml_model_versioning::StatisticalTest::TTest { p_value, t_statistic, .. }) => {
            println!("  T-test: t = {:.3}, p = {:.4}", t_statistic, p_value);
            println!("  Statistically significant: {}", result.is_significant);
        }
        _ => println!("  No statistical test performed (insufficient data)"),
    }
    
    match result.recommendation {
        Some(Variant::Control) => println!("  Recommendation: Keep control version (v1.0.0)"),
        Some(Variant::Treatment) => println!("  Recommendation: Switch to treatment version (v1.1.0)"),
        None => println!("  Recommendation: Inconclusive - need more data"),
    }
    
    // 7. Stop the experiment
    ab_manager.stop_experiment("exp-sentiment-v2")?;
    println!("\nExperiment stopped.");
    
    println!("\n=== Demo Complete ===");
    Ok(())
}