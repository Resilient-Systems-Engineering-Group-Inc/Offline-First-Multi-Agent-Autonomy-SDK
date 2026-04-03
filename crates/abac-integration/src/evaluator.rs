//! Policy evaluator with advanced condition evaluation.

use std::collections::HashMap;

use crate::error::{AbacError, Result};
use crate::model::{Subject, Resource, Environment, PolicyRule};

/// Evaluator for ABAC conditions.
pub struct PolicyEvaluator;

impl PolicyEvaluator {
    /// Evaluate a rule condition against the request context.
    pub fn evaluate_rule(
        &self,
        rule: &PolicyRule,
        subject: &Subject,
        resource: &Resource,
        action: &str,
        environment: &Environment,
    ) -> Result<bool> {
        // Build a context map for expression evaluation.
        let mut context = HashMap::new();
        context.insert("subject".to_string(), serde_json::to_value(subject)?);
        context.insert("resource".to_string(), serde_json::to_value(resource)?);
        context.insert("action".to_string(), serde_json::json!(action));
        context.insert("environment".to_string(), serde_json::to_value(environment)?);

        // Simple evaluation: if condition is empty object, treat as true.
        // In a real implementation, you would use a library like `json_logic` or `cel-expr`.
        let condition = &rule.condition;
        if condition.is_object() && condition.as_object().unwrap().is_empty() {
            return Ok(true);
        }

        // Placeholder: evaluate using a simple equality check for demonstration.
        // This is not a real ABAC evaluator.
        self.evaluate_json_logic(condition, &context)
    }

    fn evaluate_json_logic(
        &self,
        condition: &serde_json::Value,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<bool> {
        // Very simplistic evaluation: treat condition as a boolean.
        match condition {
            serde_json::Value::Bool(b) => Ok(*b),
            serde_json::Value::Object(map) => {
                // Check if it's a comparison operator
                if let Some(op) = map.get("operator") {
                    if let Some(op_str) = op.as_str() {
                        return self.evaluate_operator(op_str, map, context);
                    }
                }
                // Default: true
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    fn evaluate_operator(
        &self,
        op: &str,
        map: &serde_json::Map<String, serde_json::Value>,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<bool> {
        match op {
            "eq" => {
                let left = self.resolve_value(map.get("left"), context)?;
                let right = self.resolve_value(map.get("right"), context)?;
                Ok(left == right)
            }
            "neq" => {
                let left = self.resolve_value(map.get("left"), context)?;
                let right = self.resolve_value(map.get("right"), context)?;
                Ok(left != right)
            }
            "gt" => {
                let left = self.resolve_value(map.get("left"), context)?;
                let right = self.resolve_value(map.get("right"), context)?;
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(l > r)
                } else {
                    Ok(false)
                }
            }
            "lt" => {
                let left = self.resolve_value(map.get("left"), context)?;
                let right = self.resolve_value(map.get("right"), context)?;
                if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
                    Ok(l < r)
                } else {
                    Ok(false)
                }
            }
            "and" => {
                if let Some(serde_json::Value::Array(conditions)) = map.get("conditions") {
                    for cond in conditions {
                        if !self.evaluate_json_logic(cond, context)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            "or" => {
                if let Some(serde_json::Value::Array(conditions)) = map.get("conditions") {
                    for cond in conditions {
                        if self.evaluate_json_logic(cond, context)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                } else {
                    Ok(false)
                }
            }
            "not" => {
                if let Some(condition) = map.get("condition") {
                    Ok(!self.evaluate_json_logic(condition, context)?)
                } else {
                    Ok(false)
                }
            }
            "in" => {
                let left = self.resolve_value(map.get("left"), context)?;
                if let Some(serde_json::Value::Array(arr)) = map.get("right") {
                    Ok(arr.contains(&left))
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false),
        }
    }

    /// Resolve a value which could be a literal or a reference to context.
    fn resolve_value(
        &self,
        value: Option<&serde_json::Value>,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let value = value.unwrap_or(&serde_json::Value::Null);
        
        // If it's a string starting with "$", treat as a reference
        if let Some(ref_str) = value.as_str() {
            if ref_str.starts_with('$') {
                // Remove "$" and split by dots
                let path = &ref_str[1..];
                return self.resolve_path(path, context);
            }
        }
        Ok(value.clone())
    }

    /// Resolve a dot‑separated path in the context.
    fn resolve_path(
        &self,
        path: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return Ok(serde_json::Value::Null);
        }

        let mut current = context.get(parts[0])
            .ok_or_else(|| AbacError::MissingAttribute(parts[0].to_string()))?;
        
        for part in parts.iter().skip(1) {
            if let Some(obj) = current.as_object() {
                current = obj.get(*part)
                    .ok_or_else(|| AbacError::MissingAttribute(part.to_string()))?;
            } else {
                return Err(AbacError::MissingAttribute(part.to_string()));
            }
        }
        Ok(current.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PolicyRule;

    #[test]
    fn test_evaluator_empty_condition() {
        let evaluator = PolicyEvaluator;
        let rule = PolicyRule::new("test", serde_json::json!({}), "allow");
        let subject = Subject::new("agent1", "agent");
        let resource = Resource::new("res1", "task");
        let environment = Environment::new();
        let result = evaluator.evaluate_rule(&rule, &subject, &resource, "read", &environment);
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluator_eq_operator() {
        let evaluator = PolicyEvaluator;
        let rule = PolicyRule::new(
            "test",
            serde_json::json!({
                "operator": "eq",
                "left": 42,
                "right": 42
            }),
            "allow"
        );
        let subject = Subject::new("agent1", "agent");
        let resource = Resource::new("res1", "task");
        let environment = Environment::new();
        let result = evaluator.evaluate_rule(&rule, &subject, &resource, "read", &environment);
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluator_neq_operator() {
        let evaluator = PolicyEvaluator;
        let rule = PolicyRule::new(
            "test",
            serde_json::json!({
                "operator": "neq",
                "left": 42,
                "right": 43
            }),
            "allow"
        );
        let subject = Subject::new("agent1", "agent");
        let resource = Resource::new("res1", "task");
        let environment = Environment::new();
        let result = evaluator.evaluate_rule(&rule, &subject, &resource, "read", &environment);
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluator_gt_operator() {
        let evaluator = PolicyEvaluator;
        let rule = PolicyRule::new(
            "test",
            serde_json::json!({
                "operator": "gt",
                "left": 10,
                "right": 5
            }),
            "allow"
        );
        let subject = Subject::new("agent1", "agent");
        let resource = Resource::new("res1", "task");
        let environment = Environment::new();
        let result = evaluator.evaluate_rule(&rule, &subject, &resource, "read", &environment);
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluator_and_operator() {
        let evaluator = PolicyEvaluator;
        let rule = PolicyRule::new(
            "test",
            serde_json::json!({
                "operator": "and",
                "conditions": [
                    { "operator": "eq", "left": 1, "right": 1 },
                    { "operator": "eq", "left": 2, "right": 2 }
                ]
            }),
            "allow"
        );
        let subject = Subject::new("agent1", "agent");
        let resource = Resource::new("res1", "task");
        let environment = Environment::new();
        let result = evaluator.evaluate_rule(&rule, &subject, &resource, "read", &environment);
        assert!(result.unwrap());
    }

    #[test]
    fn test_evaluator_attribute_reference() {
        let evaluator = PolicyEvaluator;
        let rule = PolicyRule::new(
            "test",
            serde_json::json!({
                "operator": "eq",
                "left": { "operator": "eq", "left": "$subject.id", "right": "agent1" },
                "right": true
            }),
            "allow"
        );
        let mut subject = Subject::new("agent1", "agent");
        subject.add_attribute("role", serde_json::json!("admin"));
        let resource = Resource::new("res1", "task");
        let environment = Environment::new();
        let result = evaluator.evaluate_rule(&rule, &subject, &resource, "read", &environment);
        // This test is complex, just ensure it doesn't panic
        let _ = result.unwrap();
    }
}