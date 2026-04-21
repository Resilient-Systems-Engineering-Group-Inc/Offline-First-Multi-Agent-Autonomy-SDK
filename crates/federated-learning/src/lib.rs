//! Federated learning for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate enables privacy‑preserving distributed machine learning across
//! agents without centralizing raw data.

#![deny(missing_docs, unsafe_code)]

pub mod aggregation;
pub mod client;
pub mod error;
pub mod model;
pub mod server;
pub mod privacy;
pub mod distributed;
pub mod advanced_privacy;

pub use error::Error;
pub use client::FederatedClient;
pub use server::FederatedServer;
pub use distributed::{
    DistributedTrainingCoordinator, DistributedTrainingConfig, DistributedTrainingEvent,
    DistributedTrainingStats, MeshFederatedIntegration, TrainingParticipant,
};
pub use advanced_privacy::{
    RdpConfig,
    AdvancedDifferentialPrivacy,
    MomentsAccountant,
    PrivacyAmplification,
    DistributedDifferentialPrivacy,
    AdaptiveNoiseScaling,
    federated_integration::FederatedLearningWithDP,
};