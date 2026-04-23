//! Stress testing module.

use serde::{Deserialize, Serialize};

/// Stress test configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestConfig {
    pub name: String,
    pub max_requests_per_user: u64,
    pub ramp_rate: u64, // requests per second increase
    pub timeout_secs: u64,
}

impl StressTestConfig {
    pub fn new(name: &str, max_requests_per_user: u64) -> Self {
        Self {
            name: name.to_string(),
            max_requests_per_user,
            ramp_rate: 10,
            timeout_secs: 300,
        }
    }

    pub fn with_ramp_rate(mut self, rate: u64) -> Self {
        self.ramp_rate = rate;
        self
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout_secs = timeout;
        self
    }
}

/// Stress test scenarios.
pub mod scenarios {
    use super::*;

    /// Spike test - sudden increase in load.
    pub fn spike_test() -> StressTestConfig {
        StressTestConfig::new("spike", 10000).with_ramp_rate(1000)
    }

    /// Soak test - prolonged load.
    pub fn soak_test() -> StressTestConfig {
        StressTestConfig::new("soak", 1000).with_timeout(3600)
    }

    /// Breakpoint test - find system limits.
    pub fn breakpoint_test() -> StressTestConfig {
        StressTestConfig::new("breakpoint", 100000).with_timeout(600)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_test_config() {
        let config = StressTestConfig::new("test", 1000);
        assert_eq!(config.name, "test");
        assert_eq!(config.max_requests_per_user, 1000);

        let config = config.with_ramp_rate(100).with_timeout(600);
        assert_eq!(config.ramp_rate, 100);
        assert_eq!(config.timeout_secs, 600);
    }

    #[test]
    fn test_scenarios() {
        let spike = scenarios::spike_test();
        assert_eq!(spike.ramp_rate, 1000);

        let soak = scenarios::soak_test();
        assert_eq!(soak.timeout_secs, 3600);
    }
}
