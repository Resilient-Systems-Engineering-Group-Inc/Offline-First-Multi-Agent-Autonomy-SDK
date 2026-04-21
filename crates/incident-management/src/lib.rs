//! Incident management for offline‑first multi‑agent systems.
//!
//! Provides incident detection, tracking, escalation, and resolution.

pub mod error;
pub mod model;
pub mod detector;
pub mod tracker;
pub mod escalator;
pub mod resolver;

#[cfg(feature = "external")]
pub mod integration;

pub use error::IncidentError;
pub use model::{Incident, IncidentSeverity, IncidentStatus, IncidentSource};
pub use detector::IncidentDetector;
pub use tracker::IncidentTracker;
pub use escalator::EscalationPolicy;
pub use resolver::IncidentResolver;

#[cfg(feature = "external")]
pub use integration::*;

/// Re‑export of common types.
pub mod prelude {
    pub use super::{
        IncidentError,
        Incident,
        IncidentSeverity,
        IncidentStatus,
        IncidentSource,
        IncidentDetector,
        IncidentTracker,
        EscalationPolicy,
        IncidentResolver,
    };
    
    #[cfg(feature = "external")]
    pub use super::integration::*;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}