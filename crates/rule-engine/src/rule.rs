//! Rule definitions and conditions.

use serde::{Deserialize, Serialize};

/// Business rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub condition: RuleCondition,
    pub actions: Vec<Action>,
    pub priority: i32,
    pub enabled: bool,
    pub tags: Vec<String>,
}

impl Rule {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            condition: RuleCondition::Always,
            actions: Vec::new(),
            priority: 50,
            enabled: true,
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    pub fn with_condition(mut self, condition: RuleCondition) -> Self {
        self.condition = condition;
        self
    }

    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_actions(mut self, actions: Vec<Action>) -> Self {
        self.actions = actions;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn summary(&self) -> RuleSummary {
        RuleSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            priority: self.priority,
            enabled: self.enabled,
            condition_type: self.condition.type_name(),
        }
    }
}

/// Rule summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummary {
    pub id: String,
    pub name: String,
    pub priority: i32,
    pub enabled: bool,
    pub condition_type: String,
}

/// Rule condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    /// Always matches
    Always,

    /// Field equals value
    Equals {
        field: String,
        value: serde_json::Value,
    },

    /// Field not equals value
    NotEquals {
        field: String,
        value: serde_json::Value,
    },

    /// Field greater than value
    GreaterThan {
        field: String,
        value: serde_json::Value,
    },

    /// Field less than value
    LessThan {
        field: String,
        value: serde_json::Value,
    },

    /// Field contains value
    Contains {
        field: String,
        value: serde_json::Value,
    },

    /// Field matches regex pattern
    Matches {
        field: String,
        pattern: String,
    },

    /// Field in list of values
    In {
        field: String,
        values: Vec<serde_json::Value>,
    },

    /// Field exists
    Exists {
        field: String,
    },

    /// AND combination of conditions
    And {
        conditions: Vec<RuleCondition>,
    },

    /// OR combination of conditions
    Or {
        conditions: Vec<RuleCondition>,
    },

    /// NOT condition
    Not {
        condition: Box<RuleCondition>,
    },

    /// Custom condition (JavaScript expression)
    Custom {
        expression: String,
    },
}

impl RuleCondition {
    pub fn type_name(&self) -> String {
        match self {
            Self::Always => "always".to_string(),
            Self::Equals { .. } => "equals".to_string(),
            Self::NotEquals { .. } => "not_equals".to_string(),
            Self::GreaterThan { .. } => "greater_than".to_string(),
            Self::LessThan { .. } => "less_than".to_string(),
            Self::Contains { .. } => "contains".to_string(),
            Self::Matches { .. } => "matches".to_string(),
            Self::In { .. } => "in".to_string(),
            Self::Exists { .. } => "exists".to_string(),
            Self::And { .. } => "and".to_string(),
            Self::Or { .. } => "or".to_string(),
            Self::Not { .. } => "not".to_string(),
            Self::Custom { .. } => "custom".to_string(),
        }
    }

    /// Evaluate condition against context.
    pub fn evaluate(&self, context: &serde_json::Value) -> bool {
        match self {
            Self::Always => true,

            Self::Equals { field, value } => {
                get_field(context, field) == Some(value)
            }

            Self::NotEquals { field, value } => {
                get_field(context, field) != Some(value)
            }

            Self::GreaterThan { field, value } => {
                if let Some(ctx_val) = get_field(context, field) {
                    compare_values(ctx_val, value) > 0
                } else {
                    false
                }
            }

            Self::LessThan { field, value } => {
                if let Some(ctx_val) = get_field(context, field) {
                    compare_values(ctx_val, value) < 0
                } else {
                    false
                }
            }

            Self::Contains { field, value } => {
                if let Some(ctx_val) = get_field(context, field) {
                    if let Some(arr) = ctx_val.as_array() {
                        arr.contains(value)
                    } else if let Some(str) = ctx_val.as_str() {
                        if let Some(val_str) = value.as_str() {
                            str.contains(val_str)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            Self::Matches { field, pattern } => {
                if let Some(ctx_val) = get_field(context, field) {
                    if let Some(str) = ctx_val.as_str() {
                        regex::Regex::new(pattern)
                            .map(|re| re.is_match(str))
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }

            Self::In { field, values } => {
                if let Some(ctx_val) = get_field(context, field) {
                    values.contains(ctx_val)
                } else {
                    false
                }
            }

            Self::Exists { field } => {
                get_field(context, field).is_some()
            }

            Self::And { conditions } => {
                conditions.iter().all(|c| c.evaluate(context))
            }

            Self::Or { conditions } => {
                conditions.iter().any(|c| c.evaluate(context))
            }

            Self::Not { condition } => {
                !condition.evaluate(context)
            }

            Self::Custom { expression } => {
                // Would evaluate JavaScript expression
                // For now, return true as placeholder
                tracing::debug!("Custom expression: {}", expression);
                true
            }
        }
    }
}

/// Get field value from JSON using dot notation.
fn get_field(value: &serde_json::Value, path: &str) -> Option<&serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current)
}

/// Compare two JSON values.
fn compare_values(a: &serde_json::Value, b: &serde_json::Value) -> i32 {
    match (a, b) {
        (serde_json::Value::Number(a_num), serde_json::Value::Number(b_num)) => {
            if let (Some(a_f), Some(b_f)) = (a_num.as_f64(), b_num.as_f64()) {
                if a_f > b_f { 1 } else if a_f < b_f { -1 } else { 0 }
            } else {
                0
            }
        }
        (serde_json::Value::String(a_str), serde_json::Value::String(b_str)) => {
            a_str.cmp(b_str) as i32
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_equals() {
        let condition = RuleCondition::Equals {
            field: "status".to_string(),
            value: serde_json::json!("active"),
        };

        let context = serde_json::json!({
            "status": "active",
            "count": 10
        });

        assert!(condition.evaluate(&context));

        let context2 = serde_json::json!({
            "status": "inactive",
            "count": 10
        });

        assert!(!condition.evaluate(&context2));
    }

    #[test]
    fn test_condition_and() {
        let condition = RuleCondition::And {
            conditions: vec![
                RuleCondition::Equals {
                    field: "status".to_string(),
                    value: serde_json::json!("active"),
                },
                RuleCondition::GreaterThan {
                    field: "count".to_string(),
                    value: serde_json::json!(5),
                },
            ],
        };

        let context = serde_json::json!({
            "status": "active",
            "count": 10
        });

        assert!(condition.evaluate(&context));

        let context2 = serde_json::json!({
            "status": "active",
            "count": 3
        });

        assert!(!condition.evaluate(&context2));
    }

    #[test]
    fn test_condition_nested() {
        let condition = RuleCondition::Equals {
            field: "user.role".to_string(),
            value: serde_json::json!("admin"),
        };

        let context = serde_json::json!({
            "user": {
                "name": "John",
                "role": "admin"
            }
        });

        assert!(condition.evaluate(&context));
    }
}
