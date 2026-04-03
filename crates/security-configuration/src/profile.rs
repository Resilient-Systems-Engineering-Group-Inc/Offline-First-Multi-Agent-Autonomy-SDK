//! Security profile management.

use crate::config::{Policy, SecurityProfile};
use crate::error::{Result, SecurityConfigError};
use std::collections::HashMap;

/// Manages a collection of security profiles.
#[derive(Debug, Clone)]
pub struct ProfileManager {
    profiles: HashMap<String, SecurityProfile>,
    default_profile_name: String,
}

impl ProfileManager {
    /// Creates a new profile manager from a map of profiles.
    pub fn new(
        profiles: HashMap<String, SecurityProfile>,
        default_profile_name: String,
    ) -> Self {
        Self {
            profiles,
            default_profile_name,
        }
    }

    /// Returns the profile with the given name, or the default if not found.
    pub fn get_profile(&self, name: Option<&str>) -> Result<&SecurityProfile> {
        let name = name.unwrap_or(&self.default_profile_name);
        self.profiles
            .get(name)
            .ok_or_else(|| SecurityConfigError::MissingField(format!("profile '{}'", name)))
    }

    /// Returns the default profile.
    pub fn default_profile(&self) -> Result<&SecurityProfile> {
        self.get_profile(None)
    }

    /// Returns all profiles.
    pub fn all_profiles(&self) -> &HashMap<String, SecurityProfile> {
        &self.profiles
    }

    /// Adds or updates a profile.
    pub fn upsert_profile(&mut self, name: String, profile: SecurityProfile) {
        self.profiles.insert(name, profile);
    }

    /// Removes a profile (cannot remove the default profile).
    pub fn remove_profile(&mut self, name: &str) -> Result<()> {
        if name == self.default_profile_name {
            return Err(SecurityConfigError::Inconsistent(
                "cannot remove the default profile".to_string(),
            ));
        }
        self.profiles.remove(name);
        Ok(())
    }

    /// Returns all policies from a given profile (including global policies if needed).
    pub fn policies_for_profile(&self, profile_name: &str) -> Result<Vec<&Policy>> {
        let profile = self.get_profile(Some(profile_name))?;
        let mut policies = Vec::new();
        for policy in &profile.policies {
            policies.push(policy);
        }
        Ok(policies)
    }

    /// Checks whether a profile is enabled.
    pub fn is_profile_enabled(&self, name: &str) -> bool {
        self.profiles
            .get(name)
            .map(|p| p.enabled)
            .unwrap_or(false)
    }

    /// Returns the names of all enabled profiles.
    pub fn enabled_profiles(&self) -> Vec<String> {
        self.profiles
            .iter()
            .filter(|(_, p)| p.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

impl From<HashMap<String, SecurityProfile>> for ProfileManager {
    fn from(profiles: HashMap<String, SecurityProfile>) -> Self {
        let default_profile_name = if profiles.contains_key("default") {
            "default".to_string()
        } else if !profiles.is_empty() {
            profiles.keys().next().unwrap().clone()
        } else {
            "default".to_string()
        };
        Self::new(profiles, default_profile_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PolicyRule;

    fn sample_profile(name: &str, enabled: bool) -> SecurityProfile {
        SecurityProfile {
            description: format!("Profile {}", name),
            policies: vec![Policy {
                id: format!("policy-{}", name),
                name: format!("Policy {}", name),
                description: "Test policy".to_string(),
                rules: vec![PolicyRule::Action {
                    action: "test".to_string(),
                    resource: "*".to_string(),
                    allow: true,
                    conditions: HashMap::new(),
                }],
                mandatory: false,
                tags: vec![],
            }],
            enabled,
            priority: 1,
        }
    }

    #[test]
    fn test_profile_manager() {
        let mut profiles = HashMap::new();
        profiles.insert("default".to_string(), sample_profile("default", true));
        profiles.insert("strict".to_string(), sample_profile("strict", false));

        let manager = ProfileManager::new(profiles, "default".to_string());

        assert!(manager.get_profile(Some("default")).is_ok());
        assert!(manager.get_profile(Some("strict")).is_ok());
        assert!(manager.get_profile(Some("missing")).is_err());

        assert_eq!(manager.default_profile().unwrap().description, "Profile default");
        assert!(manager.is_profile_enabled("default"));
        assert!(!manager.is_profile_enabled("strict"));

        let enabled = manager.enabled_profiles();
        assert_eq!(enabled, vec!["default"]);
    }

    #[test]
    fn test_upsert_and_remove() {
        let mut profiles = HashMap::new();
        profiles.insert("default".to_string(), sample_profile("default", true));
        let mut manager = ProfileManager::new(profiles, "default".to_string());

        manager.upsert_profile("new".to_string(), sample_profile("new", true));
        assert!(manager.get_profile(Some("new")).is_ok());

        let err = manager.remove_profile("default");
        assert!(err.is_err()); // cannot remove default
        assert!(manager.remove_profile("new").is_ok());
        assert!(manager.get_profile(Some("new")).is_err());
    }
}