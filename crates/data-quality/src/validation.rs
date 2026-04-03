//! Data validation rules and validators.

use serde::{Deserialize, Serialize};
use regex::Regex;
use std::collections::HashMap;

use crate::error::{DataQualityError, Result};

/// Validation rule definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    /// Value must not be null/empty.
    NotEmpty,
    /// Value must match a regex pattern.
    Pattern(String),
    /// Value must be within a numeric range [min, max].
    Range { min: f64, max: f64 },
    /// Value must be one of the allowed values.
    Enum(Vec<String>),
    /// Value must satisfy a custom predicate (expressed as a string that can be evaluated).
    Custom(String),
}

/// Validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub passed: bool,
    /// Error message if validation failed.
    pub error: Option<String>,
    /// Rule that caused the failure.
    pub rule: Option<ValidationRule>,
}

/// Validator for a specific data field.
#[derive(Debug, Clone)]
pub struct FieldValidator {
    field_name: String,
    rules: Vec<ValidationRule>,
}

impl FieldValidator {
    /// Create a new validator for a field.
    pub fn new(field_name: impl Into<String>) -> Self {
        Self {
            field_name: field_name.into(),
            rules: Vec::new(),
        }
    }

    /// Add a validation rule.
    pub fn with_rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Validate a value.
    pub fn validate(&self, value: &serde_json::Value) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        for rule in &self.rules {
            let result = self.apply_rule(value, rule);
            results.push(result);
        }
        results
    }

    fn apply_rule(&self, value: &serde_json::Value, rule: &ValidationRule) -> ValidationResult {
        let passed = match rule {
            ValidationRule::NotEmpty => !value.is_null() && !value.as_str().map(|s| s.is_empty()).unwrap_or(false),
            ValidationRule::Pattern(pattern) => {
                let re = Regex::new(pattern).unwrap(); // In production, cache regex
                value.as_str().map(|s| re.is_match(s)).unwrap_or(false)
            }
            ValidationRule::Range { min, max } => {
                value.as_f64().map(|v| v >= *min && v <= *max).unwrap_or(false)
            }
            ValidationRule::Enum(allowed) => {
                value.as_str().map(|s| allowed.contains(&s.to_string())).unwrap_or(false)
            }
            ValidationRule::Custom(_) => true, // placeholder
        };

        ValidationResult {
            passed,
            error: if !passed {
                Some(format!("Field '{}' failed rule {:?}", self.field_name, rule))
            } else {
                None
            },
            rule: Some(rule.clone()),
        }
    }
}

/// Validator for a whole data object (e.g., JSON).
#[derive(Debug, Clone)]
pub struct Validator {
    field_validators: HashMap<String, FieldValidator>,
}

impl Validator {
    /// Create a new empty validator.
    pub fn new() -> Self {
        Self {
            field_validators: HashMap::new(),
        }
    }

    /// Add a field validator.
    pub fn add_field_validator(&mut self, validator: FieldValidator) {
        self.field_validators.insert(validator.field_name.clone(), validator);
    }

    /// Validate a JSON object.
    pub fn validate_object(&self, obj: &serde_json::Map<String, serde_json::Value>) -> Vec<ValidationResult> {
        let mut results = Vec::new();
        for (field, validator) in &self.field_validators {
            if let Some(value) = obj.get(field) {
                results.extend(validator.validate(value));
            } else {
                results.push(ValidationResult {
                    passed: false,
                    error: Some(format!("Field '{}' missing", field)),
                    rule: None,
                });
            }
        }
        results
    }

    /// Validate a JSON value (must be an object).
    pub fn validate(&self, value: &serde_json::Value) -> Result<Vec<ValidationResult>> {
        match value.as_object() {
            Some(obj) => Ok(self.validate_object(obj)),
            None => Err(DataQualityError::ValidationFailed("Expected JSON object".to_string())),
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_empty_rule() {
        let validator = FieldValidator::new("name")
            .with_rule(ValidationRule::NotEmpty);
        let value = serde_json::json!("hello");
        let results = validator.validate(&value);
        assert!(results[0].passed);
    }

    #[test]
    fn test_range_rule() {
        let validator = FieldValidator::new("age")
            .with_rule(ValidationRule::Range { min: 0.0, max: 120.0 });
        let value = serde_json::json!(25);
        let results = validator.validate(&value);
        assert!(results[0].passed);
    }
}