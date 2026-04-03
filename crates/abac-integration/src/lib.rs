//! Attribute‑Based Access Control (ABAC) integration for offline‑first multi‑agent systems.
//!
//! Provides policy evaluation based on attributes of subjects, resources, actions, and environment.

pub mod error;
pub mod model;
pub mod policy;
pub mod evaluator;
pub mod integration;

pub use error::AbacError;
pub use model::{Attribute, Subject, Resource, Environment, Policy};
pub use policy::PolicyEngine;
pub use evaluator::PolicyEvaluator;
pub use integration::{RbacAbacIntegration, MetadataAbacIntegration};

/// Re‑export of common types.
pub mod prelude {
    pub use super::{
        AbacError,
        Attribute,
        Subject,
        Resource,
        Environment,
        Policy,
        PolicyEngine,
        PolicyEvaluator,
        RbacAbacIntegration,
        MetadataAbacIntegration,
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}