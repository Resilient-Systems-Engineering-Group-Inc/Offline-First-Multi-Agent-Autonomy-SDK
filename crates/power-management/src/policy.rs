//! Power management policies.

use crate::error::{Result, Error};
use crate::monitor::PowerMetrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Power management mode (high‑level goal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerMode {
    /// Maximize performance, ignore power consumption.
    Performance,
    /// Balance performance and power.
    Balanced,
    /// Maximize battery life, reduce performance.
    PowerSaver,
    /// Custom mode with specific parameters.
    Custom,
}

/// Action that can be taken to adjust power consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PowerAction {
    /// Adjust CPU frequency (limit max frequency in MHz).
    CpuFrequencyLimit(u64),
    /// Adjust number of active CPU cores.
    CpuCoresLimit(usize),
    /// Enable or disable hardware acceleration (GPU/TPU).
    HardwareAcceleration(bool),
    /// Adjust screen brightness (0‑100%).
    ScreenBrightness(u8),
    /// Put device into sleep after inactivity (seconds).
    SleepTimeout(u64),
    /// Reduce network activity (e.g., lower polling rate).
    NetworkThrottling,
    /// No action.
    None,
}

/// Power policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerPolicy {
    /// Name of the policy.
    pub name: String,
    /// Target power mode.
    pub mode: PowerMode,
    /// List of actions to apply when this policy is active.
    pub actions: Vec<PowerAction>,
    /// Conditions for activating this policy (optional).
    pub conditions: Vec<PolicyCondition>,
    /// Priority (higher = more important).
    pub priority: u8,
}

/// Condition that triggers a policy change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Battery level below threshold (percent).
    BatteryBelow(f32),
    /// Battery level above threshold.
    BatteryAbove(f32),
    /// Power source is AC.
    OnAcPower,
    /// Power source is battery.
    OnBattery,
    /// System power draw exceeds threshold (watts).
    PowerDrawExceeds(f32),
    /// CPU temperature exceeds threshold (Celsius).
    TemperatureExceeds(f32),
    /// Always true (unconditional).
    Always,
}

impl PowerPolicy {
    /// Creates a new policy.
    pub fn new(name: &str, mode: PowerMode, actions: Vec<PowerAction>) -> Self {
        Self {
            name: name.to_string(),
            mode,
            actions,
            conditions: vec![],
            priority: 0,
        }
    }

    /// Adds a condition to the policy.
    pub fn with_condition(mut self, condition: PolicyCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Sets the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Evaluates whether the policy should be activated given current metrics.
    pub fn should_activate(&self, metrics: &PowerMetrics) -> bool {
        if self.conditions.is_empty() {
            return true;
        }
        // All conditions must be satisfied (AND logic).
        self.conditions.iter().all(|cond| cond.evaluate(metrics))
    }
}

impl PolicyCondition {
    /// Evaluates the condition against current power metrics.
    pub fn evaluate(&self, metrics: &PowerMetrics) -> bool {
        match self {
            PolicyCondition::BatteryBelow(threshold) => {
                metrics.battery_percent.map(|p| p < *threshold).unwrap_or(false)
            }
            PolicyCondition::BatteryAbove(threshold) => {
                metrics.battery_percent.map(|p| p > *threshold).unwrap_or(false)
            }
            PolicyCondition::OnAcPower => matches!(metrics.source, crate::monitor::PowerSource::Ac),
            PolicyCondition::OnBattery => {
                matches!(metrics.source, crate::monitor::PowerSource::Battery)
            }
            PolicyCondition::PowerDrawExceeds(threshold) => {
                metrics.system_power_watts.map(|w| w > *threshold).unwrap_or(false)
            }
            PolicyCondition::TemperatureExceeds(_) => {
                // Temperature not yet in PowerMetrics; ignore for now.
                false
            }
            PolicyCondition::Always => true,
        }
    }
}

/// Policy manager that selects and applies the appropriate policy.
pub struct PowerPolicyManager {
    policies: Vec<PowerPolicy>,
    active_policy: Option<String>,
}

impl PowerPolicyManager {
    /// Creates a new policy manager with default policies.
    pub fn new() -> Self {
        let default_policies = vec![
            PowerPolicy::new(
                "performance",
                PowerMode::Performance,
                vec![
                    PowerAction::CpuFrequencyLimit(0), // 0 = no limit
                    PowerAction::HardwareAcceleration(true),
                ],
            )
            .with_condition(PolicyCondition::OnAcPower)
            .with_priority(10),
            PowerPolicy::new(
                "balanced",
                PowerMode::Balanced,
                vec![
                    PowerAction::CpuFrequencyLimit(2000),
                    PowerAction::HardwareAcceleration(true),
                ],
            )
            .with_condition(PolicyCondition::OnBattery)
            .with_condition(PolicyCondition::BatteryAbove(20.0))
            .with_priority(5),
            PowerPolicy::new(
                "power_saver",
                PowerMode::PowerSaver,
                vec![
                    PowerAction::CpuFrequencyLimit(1000),
                    PowerAction::CpuCoresLimit(2),
                    PowerAction::HardwareAcceleration(false),
                    PowerAction::ScreenBrightness(30),
                    PowerAction::SleepTimeout(60),
                ],
            )
            .with_condition(PolicyCondition::OnBattery)
            .with_condition(PolicyCondition::BatteryBelow(20.0))
            .with_priority(20),
        ];

        Self {
            policies: default_policies,
            active_policy: None,
        }
    }

    /// Adds a custom policy.
    pub fn add_policy(&mut self, policy: PowerPolicy) {
        self.policies.push(policy);
    }

    /// Evaluates all policies and returns the highest‑priority policy that should be active.
    pub fn evaluate(&self, metrics: &PowerMetrics) -> Option<&PowerPolicy> {
        let mut candidates: Vec<&PowerPolicy> = self
            .policies
            .iter()
            .filter(|p| p.should_activate(metrics))
            .collect();
        candidates.sort_by_key(|p| std::cmp::Reverse(p.priority));
        candidates.first().copied()
    }

    /// Applies the best policy for the given metrics, returning the actions taken.
    pub fn apply_best_policy(&mut self, metrics: &PowerMetrics) -> Result<Vec<PowerAction>> {
        let policy = self.evaluate(metrics).ok_or_else(|| {
            Error::PolicyError("No suitable policy found".to_string())
        })?;
        self.active_policy = Some(policy.name.clone());
        Ok(policy.actions.clone())
    }

    /// Returns the currently active policy name.
    pub fn active_policy(&self) -> Option<&str> {
        self.active_policy.as_deref()
    }
}

/// Default policy set for edge devices.
pub fn default_policies() -> Vec<PowerPolicy> {
    let mut manager = PowerPolicyManager::new();
    manager.policies.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{PowerMetrics, PowerSource, BatteryStatus};

    fn sample_metrics(battery_percent: Option<f32>, source: PowerSource) -> PowerMetrics {
        PowerMetrics {
            source,
            battery_percent,
            battery_status: BatteryStatus::Discharging,
            battery_remaining_secs: None,
            cpu_frequency_mhz: None,
            cpu_power_watts: None,
            system_power_watts: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn test_policy_creation() {
        let policy = PowerPolicy::new("test", PowerMode::Balanced, vec![PowerAction::None]);
        assert_eq!(policy.name, "test");
        assert_eq!(policy.mode, PowerMode::Balanced);
    }

    #[test]
    fn test_condition_evaluation() {
        let metrics = sample_metrics(Some(30.0), PowerSource::Battery);
        let cond = PolicyCondition::BatteryBelow(20.0);
        assert!(!cond.evaluate(&metrics));
        let cond2 = PolicyCondition::OnBattery;
        assert!(cond2.evaluate(&metrics));
    }

    #[test]
    fn test_policy_manager() {
        let manager = PowerPolicyManager::new();
        let metrics = sample_metrics(Some(80.0), PowerSource::Ac);
        let policy = manager.evaluate(&metrics);
        assert!(policy.is_some());
        assert_eq!(policy.unwrap().name, "performance");
    }
}