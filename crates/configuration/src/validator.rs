//! Configuration validation.

use crate::error::Error;
use crate::schema::Configuration;
use validator::{Validate, ValidationErrors};

/// Validator for configuration.
#[derive(Debug, Default)]
pub struct Validator;

impl Validator {
    /// Create a new validator.
    pub fn new() -> Self {
        Self
    }

    /// Validate a configuration.
    pub fn validate(&self, config: &Configuration) -> Result<(), Error> {
        // Use `validator` crate if the feature is enabled.
        #[cfg(feature = "validation")]
        {
            config
                .validate()
                .map_err(|e| Error::Validation(format!("Validation failed: {}", e)))?;
        }

        // Custom validation logic.
        self.custom_validate(config)?;
        Ok(())
    }

    fn custom_validate(&self, config: &Configuration) -> Result<(), Error> {
        if config.mesh.listen_addr.is_empty() {
            return Err(Error::Validation("mesh.listen_addr cannot be empty".into()));
        }
        if config.agent.max_concurrent_tasks == 0 {
            return Err(Error::Validation(
                "agent.max_concurrent_tasks must be > 0".into(),
            ));
        }
        if config.state_sync.sync_interval_secs == 0 {
            return Err(Error::Validation(
                "state_sync.sync_interval_secs must be > 0".into(),
            ));
        }
        if config.resource_monitor.collection_interval_secs == 0 {
            return Err(Error::Validation(
                "resource_monitor.collection_interval_secs must be > 0".into(),
            ));
        }
        Ok(())
    }
}