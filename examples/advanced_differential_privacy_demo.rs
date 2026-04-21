//! Advanced Differential Privacy for Federated Learning Demo.
//!
//! This example demonstrates the advanced differential privacy techniques
//! implemented in the federated-learning crate, including:
//! - Renyi Differential Privacy (RDP) accounting
//! - Moments Accountant for privacy budget tracking
//! - Privacy Amplification by Sampling
//! - Distributed DP with secure aggregation
//! - Adaptive noise scaling
//! - Integration with federated learning pipeline

use std::sync::Arc;
use federated_learning::prelude::*;
use federated_learning::advanced_privacy::*;
use federated_learning::privacy::DifferentialPrivacyConfig;
use federated_learning::model::ModelUpdate;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Advanced Differential Privacy for Federated Learning Demo ===");
    
    // 1. Create a differential privacy configuration
    let dp_config = DifferentialPrivacyConfig {
        epsilon: 1.0,
        delta: 1e-5,
        sigma: 1.0,
        clip_norm: 2.0,
    };
    
    println!("DP Configuration: ε={}, δ={}, σ={}, clip_norm={}", 
        dp_config.epsilon, dp_config.delta, dp_config.sigma, dp_config.clip_norm);
    
    // 2. Create advanced DP engine with RDP configuration
    let rdp_config = RdpConfig {
        alphas: vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0],
        max_alpha: 32.0,
        orders: vec![1.5, 2.0, 4.0, 8.0, 16.0],
    };
    
    let advanced_dp = AdvancedDifferentialPrivacy::new(dp_config.clone(), Some(rdp_config));
    println!("Created AdvancedDifferentialPrivacy with RDP accounting");
    
    // 3. Demonstrate Gaussian mechanism
    let sensitive_vector = vec![1.2, -0.5, 3.7, 0.1, -2.3];
    let sensitivity = 1.0;
    
    let noisy_vector = advanced_dp.gaussian_mechanism(&sensitive_vector, sensitivity);
    println!("\nGaussian Mechanism:");
    println!("  Original vector: {:?}", sensitive_vector);
    println!("  Noisy vector: {:?}", noisy_vector);
    println!("  Noise added: {:?}", 
        noisy_vector.iter().zip(&sensitive_vector)
            .map(|(n, o)| n - o)
            .collect::<Vec<f64>>());
    
    // 4. Demonstrate Laplace mechanism
    let laplace_noisy = advanced_dp.laplace_mechanism(&sensitive_vector, sensitivity);
    println!("\nLaplace Mechanism:");
    println!("  Noisy vector: {:?}", laplace_noisy);
    
    // 5. Demonstrate adaptive clipping
    let gradients = vec![
        vec![1.0, 2.0, 3.0],
        vec![0.5, 1.5, 2.5],
        vec![2.0, 1.0, 0.5],
        vec![3.0, 2.0, 1.0],
        vec![0.1, 0.2, 0.3],
    ];
    
    let adaptive_clip_norm = advanced_dp.adaptive_clip(&gradients, 90.0);
    println!("\nAdaptive Clipping:");
    println!("  Original clip norm: {}", dp_config.clip_norm);
    println!("  Adaptive clip norm (90th percentile): {}", adaptive_clip_norm);
    
    // 6. Demonstrate DP-SGD
    println!("\nDP-SGD (Differentially Private Stochastic Gradient Descent):");
    let (dp_update, privacy_cost) = advanced_dp.dp_sgd(
        &gradients,
        0.01,  // learning rate
        10,    // batch size
        100,   // total samples
    );
    
    println!("  DP update vector (first 3 elements): {:?}", &dp_update[..3.min(dp_update.len())]);
    println!("  Privacy cost for this batch: ε={:.6}, δ={:.6}", 
        privacy_cost.0, privacy_cost.1);
    
    // 7. Demonstrate Moments Accountant
    let mut accountant = MomentsAccountant::new(dp_config.clone());
    accountant.update(10, 100, 5); // batch_size=10, total_samples=100, iterations=5
    
    let total_cost = accountant.compute_privacy_cost(10, 100, 5);
    println!("\nMoments Accountant:");
    println!("  Total privacy cost after 5 iterations: ε={:.6}, δ={:.6}", 
        total_cost.0, total_cost.1);
    
    // 8. Demonstrate Privacy Amplification
    let sampling_rate = 0.1; // 10% sampling
    let amplifier = PrivacyAmplification::new(sampling_rate);
    let (epsilon_base, delta_base) = (1.0, 1e-5);
    let (epsilon_amp, delta_amp) = amplifier.amplify(epsilon_base, delta_base);
    
    println!("\nPrivacy Amplification by Sampling:");
    println!("  Sampling rate: {}", sampling_rate);
    println!("  Base privacy: ε={}, δ={}", epsilon_base, delta_base);
    println!("  Amplified privacy: ε={:.6}, δ={:.6}", epsilon_amp, delta_amp);
    println!("  Privacy gain: {:.1}%", (1.0 - epsilon_amp/epsilon_base) * 100.0);
    
    // 9. Demonstrate Distributed Differential Privacy
    println!("\nDistributed Differential Privacy:");
    let distributed_dp = DistributedDifferentialPrivacy::new(
        Arc::new(advanced_dp),
        5,  // participants
        3,  // threshold (2/3)
    );
    
    let sensitivities = vec![1.0, 1.0, 1.0, 1.0, 1.0];
    let (aggregated_result, dist_privacy_cost) = distributed_dp.apply_distributed_dp(
        &gradients,
        &sensitivities,
    );
    
    println!("  Participants: 5, Threshold: 3");
    println!("  Aggregated result dimension: {}", aggregated_result.len());
    println!("  Distributed privacy cost: ε={:.6}, δ={:.6}", 
        dist_privacy_cost.0, dist_privacy_cost.1);
    
    // 10. Demonstrate Adaptive Noise Scaling
    println!("\nAdaptive Noise Scaling:");
    let mut noise_scaler = AdaptiveNoiseScaling::new(
        1.0,    // base_sigma
        0.1,    // adaptation_rate
        0.5,    // target_epsilon
    );
    
    for i in 0..5 {
        let current_epsilon = 0.6 - i as f64 * 0.05; // Simulated decreasing epsilon
        let adaptive_sigma = noise_scaler.adaptive_sigma(current_epsilon);
        println!("  Iteration {}: current_ε={:.3}, adaptive_σ={:.3}", 
            i + 1, current_epsilon, adaptive_sigma);
    }
    
    // 11. Demonstrate Federated Learning Integration
    println!("\nFederated Learning with DP Integration:");
    let mut fed_dp = FederatedLearningWithDP::new(dp_config.clone(), Some(3));
    
    // Create mock client updates
    let client_updates: Vec<ModelUpdate> = (0..3)
        .map(|i| ModelUpdate {
            parameters: vec![0.5 + i as f64 * 0.1, -0.2 + i as f64 * 0.05, 0.8 - i as f64 * 0.1],
            metadata: std::collections::HashMap::from([
                ("client_id".to_string(), i.to_string()),
                ("samples".to_string(), "100".to_string()),
            ]),
        })
        .collect();
    
    let (dp_update_result, round_cost) = fed_dp.apply_round(
        &client_updates,
        0.01,  // learning_rate
        10,    // batch_size
        100,   // total_samples
    );
    
    println!("  Applied DP to federated round with {} client updates", client_updates.len());
    println!("  Resulting update parameters: {:?}", &dp_update_result.parameters[..3.min(dp_update_result.parameters.len())]);
    println!("  Round privacy cost: ε={:.6}, δ={:.6}", round_cost.0, round_cost.1);
    
    let total_cost = fed_dp.total_privacy_cost();
    println!("  Total privacy cost so far: ε={:.6}, δ={:.6}", total_cost.0, total_cost.1);
    
    // 12. Check privacy budget exhaustion
    let budget_exhausted = fed_dp.is_budget_exhausted(2.0, 1e-4);
    println!("  Privacy budget exhausted (ε<2.0, δ<1e-4)? {}", budget_exhausted);
    
    println!("\n=== Demo Completed Successfully ===");
    println!("\nSummary of Advanced DP Techniques Demonstrated:");
    println!("1. Gaussian and Laplace mechanisms");
    println!("2. Adaptive clipping based on gradient distribution");
    println!("3. DP-SGD for federated learning");
    println!("4. Moments Accountant for precise privacy budget tracking");
    println!("5. Privacy Amplification by Sampling");
    println!("6. Distributed DP with secure aggregation");
    println!("7. Adaptive noise scaling");
    println!("8. Full integration with federated learning pipeline");
    
    Ok(())
}