//! Load testing module.

use serde::{Deserialize, Serialize};

/// Load test configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadTestConfig {
    pub name: String,
    pub requests_per_user: u64,
}

impl LoadTestConfig {
    pub fn new(name: &str, requests_per_user: u64) -> Self {
        Self {
            name: name.to_string(),
            requests_per_user,
        }
    }

    pub fn with_requests(mut self, requests: u64) -> Self {
        self.requests_per_user = requests;
        self
    }
}

/// Load test scenarios.
pub mod scenarios {
    use super::*;

    /// Basic API load test.
    pub fn basic_api_test() -> LoadTestConfig {
        LoadTestConfig::new("basic_api", 100)
    }

    /// High throughput test.
    pub fn high_throughput_test() -> LoadTestConfig {
        LoadTestConfig::new("high_throughput", 1000)
    }

    /// Sustained load test.
    pub fn sustained_load_test() -> LoadTestConfig {
        LoadTestConfig::new("sustained_load", 500)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_test_config() {
        let config = LoadTestConfig::new("test", 100);
        assert_eq!(config.name, "test");
        assert_eq!(config.requests_per_user, 100);

        let config = config.with_requests(200);
        assert_eq!(config.requests_per_user, 200);
    }

    #[test]
    fn test_scenarios() {
        let basic = scenarios::basic_api_test();
        assert_eq!(basic.requests_per_user, 100);

        let high = scenarios::high_throughput_test();
        assert_eq!(high.requests_per_user, 1000);
    }
}
